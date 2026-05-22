use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;

pub async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let start = Instant::now();

    let response = next.run(request).await;

    tracing::info!(
        method = %method,
        path = %path,
        status = response.status().as_u16(),
        elapsed = ?start.elapsed(),

    );

    response
}