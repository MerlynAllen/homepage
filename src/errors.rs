use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub fn error_response(status: StatusCode, reason: &str) -> Response {
    let error_message = serde_json::json!({ "error": reason });
    (status, Json(error_message)).into_response()
}
