use axum::extract::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct HelloWorldResponse {
    message: String,
}

// 根路由处理器
pub async fn root() -> Json<HelloWorldResponse> {
    Json(HelloWorldResponse {
        message: "Welcome to the Axum server!".into(),
    })
}
