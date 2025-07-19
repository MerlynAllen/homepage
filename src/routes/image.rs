use crate::entities::images::{self, Entity as Image};
use crate::errors::error_response;
use axum::http::Uri;
use axum::{
    extract::{Multipart, OriginalUri, Path, State},
    http::{StatusCode, header},
    response::{Json, Response},
};
use log::{debug, error, info, warn};
use sea_orm::*;
use serde::Serialize;
use std::path::Path as StdPath;
use tokio::fs;
use uuid::Uuid;

#[derive(Serialize)]
pub struct ImageResponse {
    uuid: String,
    url: String,
    filename: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct UploadResponse {
    uuid: String,
    url: String,
    filename: String,
}

const IMAGE_UPLOAD_DIR: &str = "uploads/images";

pub async fn get_image(
    State(db): State<DatabaseConnection>,
    Path(uuid): Path<String>,
) -> Result<Response, Response> {
    info!("Getting image with UUID: {}", uuid);

    // 从数据库查询图片
    match Image::find()
        .filter(images::Column::Uuid.eq(&uuid))
        .one(&db)
        .await
    {
        Ok(Some(image)) => {
            debug!("Found image in database: {}", image.filename);

            // 检查文件是否存在
            if StdPath::new(&image.file_path).exists() {
                debug!("Image file exists at: {}", image.file_path);

                // 读取文件内容
                match fs::read(&image.file_path).await {
                    Ok(contents) => {
                        info!(
                            "Successfully read image file: {} (size: {} bytes)",
                            image.filename,
                            contents.len()
                        );

                        // 设置适当的 Content-Type
                        let content_type = image
                            .mime_type
                            .as_deref()
                            .unwrap_or("application/octet-stream");

                        Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, content_type)
                            .header(header::CACHE_CONTROL, "public, max-age=3600")
                            .body(contents.into())
                            .map_err(|_| {
                                error!("Failed to build response for image: {}", uuid);
                                error_response(
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Failed to build response",
                                )
                            })
                    }
                    Err(e) => {
                        error!("Failed to read file {}: {}", image.file_path, e);
                        Err(error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to read image file",
                        ))
                    }
                }
            } else {
                warn!("Image file not found on disk: {}", image.file_path);
                Err(error_response(
                    StatusCode::NOT_FOUND,
                    "Image file not found",
                ))
            }
        }
        Ok(None) => {
            warn!("Image not found in database: {}", uuid);
            Err(error_response(StatusCode::NOT_FOUND, "Image not found"))
        }
        Err(e) => {
            error!("Database error when getting image {}: {}", uuid, e);
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
            ))
        }
    }
}

pub async fn post_image(
    State(db): State<DatabaseConnection>,
    OriginalUri(original_uri): OriginalUri,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, Response> {
    info!("Starting image upload process");

    // 创建上传目录
    if let Err(e) = fs::create_dir_all(IMAGE_UPLOAD_DIR).await {
        error!(
            "Failed to create upload directory {}: {}",
            IMAGE_UPLOAD_DIR, e
        );
        return Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create upload directory",
        ));
    }

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        debug!("Processing multipart field: {}", name);

        if name == "image" || name == "file" {
            let original_filename = field.file_name().unwrap_or("unknown").to_string();
            let content_type = field.content_type().map(|ct| ct.to_string());

            info!(
                "Uploading file: {} (content-type: {:?})",
                original_filename, content_type
            );

            let data = match field.bytes().await {
                Ok(bytes) => {
                    info!("Successfully read {} bytes from upload", bytes.len());
                    bytes
                }
                Err(e) => {
                    error!("Failed to read field {}: {}", name, e);
                    return Err(error_response(
                        e.status(),
                        &format!("Failed to read field {}: {}", name, e),
                    ));
                }
            };

            let uuid = Uuid::new_v4();
            let extension = StdPath::new(&original_filename)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("jpg");

            let filename = format!("{}.{}", uuid, extension);
            let file_path = format!("{}/{}", IMAGE_UPLOAD_DIR, filename);

            debug!("Generated filename: {} -> {}", original_filename, filename);
            debug!("File will be saved to: {}", file_path);

            // 保存文件到磁盘
            match fs::write(&file_path, &data).await {
                Ok(_) => {
                    info!("Successfully saved file to disk: {}", file_path);

                    // 保存到数据库
                    let image_model = images::ActiveModel {
                        uuid: Set(uuid.to_string()),
                        filename: Set(filename.clone()),
                        mime_type: Set(content_type),
                        file_path: Set(file_path.clone()),
                        created_at: Set(chrono::Utc::now()),
                        ..Default::default()
                    };
                    let hostname = original_uri.host().unwrap_or("localhost").to_string();
                    let port = original_uri.port_u16().unwrap_or(80);
                    let url = Uri::builder()
                        .scheme("http")
                        .authority(format!("{}:{}", hostname, port))
                        .path_and_query(format!("/image/{}", uuid))
                        .build()
                        .unwrap()
                        .to_string();

                    debug!("Generated URL: {}", url);

                    match image_model.insert(&db).await {
                        Ok(saved_image) => {
                            info!(
                                "Successfully saved image to database: {} (UUID: {})",
                                saved_image.filename, saved_image.uuid
                            );
                            return Ok(Json(UploadResponse {
                                uuid: saved_image.uuid,
                                url,
                                filename: saved_image.filename,
                            }));
                        }
                        Err(e) => {
                            error!("Failed to save to database: {}", e);
                            // 删除已保存的文件
                            if let Err(remove_err) = fs::remove_file(&file_path).await {
                                warn!("Failed to cleanup file after DB error: {}", remove_err);
                            }
                            return Err(error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to save to database",
                            ));
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to save file to {}: {}", file_path, e);
                    return Err(error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to save file",
                    ));
                }
            }
        }
    }

    warn!("No image or file field found in multipart data");
    Err(error_response(
        StatusCode::BAD_REQUEST,
        "No image or file field found in multipart data",
    ))
}

// 额外的查询示例
pub async fn list_images(
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<ImageResponse>>, Response> {
    info!("Listing images (limit: 50)");

    match Image::find()
        .order_by_desc(images::Column::CreatedAt)
        .limit(50)
        .all(&db)
        .await
    {
        Ok(images) => {
            info!("Found {} images in database", images.len());

            let response: Vec<ImageResponse> = images
                .into_iter()
                .map(|img| {
                    debug!("Processing image: {} ({})", img.filename, img.uuid);
                    ImageResponse {
                        uuid: img.uuid.clone(),
                        url: format!("http://localhost:3000/image/{}", img.uuid),
                        filename: img.filename,
                        created_at: img.created_at,
                    }
                })
                .collect();
            Ok(Json(response))
        }
        Err(e) => {
            error!("Database error when listing images: {}", e);
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
            ))
        }
    }
}

pub async fn delete_image(
    State(db): State<DatabaseConnection>,
    Path(uuid): Path<String>,
) -> Result<Json<serde_json::Value>, Response> {
    info!("Deleting image with UUID: {}", uuid);

    // 先查找图片
    match Image::find()
        .filter(images::Column::Uuid.eq(&uuid))
        .one(&db)
        .await
    {
        Ok(Some(image)) => {
            debug!(
                "Found image to delete: {} at {}",
                image.filename, image.file_path
            );

            // 删除文件
            match fs::remove_file(&image.file_path).await {
                Ok(_) => {
                    info!("Successfully deleted file: {}", image.file_path);
                }
                Err(e) => {
                    warn!("Failed to delete file {}: {}", image.file_path, e);
                }
            }

            // 从数据库删除
            match Image::delete_by_id(image.id).exec(&db).await {
                Ok(_) => {
                    info!(
                        "Successfully deleted image from database: {} (UUID: {})",
                        image.filename, uuid
                    );
                    Ok(Json(
                        serde_json::json!({ "message": "Image deleted successfully" }),
                    ))
                }
                Err(e) => {
                    error!("Failed to delete from database: {}", e);
                    Err(error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to delete from database",
                    ))
                }
            }
        }
        Ok(None) => {
            warn!("Image not found for deletion: {}", uuid);
            Err(error_response(StatusCode::NOT_FOUND, "Image not found"))
        }
        Err(e) => {
            error!("Database error when deleting image {}: {}", uuid, e);
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
            ))
        }
    }
}
