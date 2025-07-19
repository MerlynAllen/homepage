mod routes;
mod errors;
mod entities;
use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post, delete},
    Router,
};

use clap::Parser;
use log;
use sea_orm::{Database, DatabaseConnection};
use std::{env, time::Instant};
use tokio;
use pretty_env_logger;
use axum::{
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"), author = env!("CARGO_PKG_AUTHORS"))]
#[command(about = env!("CARGO_PKG_DESCRIPTION"), long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Args {
    /// Port number to bind the server to
    #[arg(short, long, env = "PORT", default_value_t = 3000)]
    port: u16,
    /// Database URL for SeaORM
    #[arg(short, long, env = "DATABASE_URL", default_value = "sqlite://info.db")]
    database_url: String,
}


const MAX_IMAGE_UPLOAD_SIZE: u64 = 50 * 1024 * 1024; // 50 MB

// 请求日志中间件
async fn request_logging_middleware(
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    
    // 获取客户端 IP 地址
    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|hv| hv.to_str().ok())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|hv| hv.to_str().ok())
        })
        .unwrap_or("unknown");
    
    // 获取 User-Agent
    let user_agent = headers
        .get("user-agent")
        .and_then(|hv| hv.to_str().ok())
        .unwrap_or("unknown");

    log::info!("→ {} {} from {} - User-Agent: {}", method, uri, client_ip, user_agent);

    // 执行请求
    let response = next.run(request).await;
    
    let duration = start.elapsed();
    let status = response.status();
    
    log::info!("← {} {} {} - {}ms", method, uri, status, duration.as_millis());
    
    response
}


#[tokio::main]
async fn main() {
    // 解析命令行参数
    let args = Args::parse();
    let port = args.port;
    
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Off)  // 屏蔽所有其他模块的日志
        .filter_module("homepage_backend", log::LevelFilter::Debug)  // 只显示本程序的日志
        .init();

    let db: DatabaseConnection = Database::connect(&args.database_url)
        .await
        .expect("Failed to connect to the database");

    log::info!("Database connected successfully");
    log::debug!("Using database URL: {}", args.database_url);

    // 构建路由
    let app = Router::new()
        .route("/", get(routes::root::root))
        .route("/image", post(routes::image::post_image).layer(DefaultBodyLimit::max(MAX_IMAGE_UPLOAD_SIZE as usize)))
        .route("/image/{uuid}", get(routes::image::get_image))
        .route("/image/{uuid}", delete(routes::image::delete_image))
        .route("/images", get(routes::image::list_images))
        .layer(middleware::from_fn(request_logging_middleware))  // 添加请求日志中间件
        .with_state(db);

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
        
    log::info!("🚀 Server starting on port {}", port);
    
    axum::serve(listener, app).await.unwrap();
}
