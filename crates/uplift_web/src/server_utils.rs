use uuid::Uuid;

pub fn extract_session_id(headers: &axum::http::HeaderMap) -> Option<Uuid> {
    let cookie = headers
        .get(axum::http::header::COOKIE)?
        .to_str()
        .ok()?;
    cookie
        .split(';')
        .map(|s| s.trim())
        .find(|s| s.starts_with("uplift_session="))
        .and_then(|s| s.strip_prefix("uplift_session="))
        .and_then(|s| s.parse::<Uuid>().ok())
}