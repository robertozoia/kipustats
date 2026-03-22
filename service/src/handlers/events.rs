use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use crate::db::AppState;
use crate::models::AnalyticsEvent;

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

fn verify_token(headers: &HeaderMap, expected: &Option<String>) -> Result<(), (StatusCode, &'static str)> {
    let expected = match expected {
        Some(t) => t,
        None => return Ok(()), // no token configured, allow all
    };

    let header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header"))?;

    let token = header
        .strip_prefix("Bearer ")
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid Authorization format"))?;

    if token != expected {
        return Err((StatusCode::UNAUTHORIZED, "Invalid token"));
    }

    Ok(())
}

#[axum::debug_handler]
pub async fn track_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(event): Json<AnalyticsEvent>,
) -> Result<(StatusCode, Json<AnalyticsEvent>), (StatusCode, &'static str)> {
    verify_token(&headers, &state.auth_token)?;
    tracing::info!(
        url           = %event.url,
        referer       = ?event.referer,
        user_agent    = %event.user_agent,
        country       = ?event.country,
        city          = ?event.city,
        timezone      = ?event.timezone,
        timestamp     = %event.timestamp,
        visitor_hash  = %event.visitor_hash,
        "Analytics event received"
    );

    let db = state.db.lock().unwrap();

    db.execute(
        "INSERT INTO events (url, referer, user_agent, country, city, timezone, timestamp, visitor_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            &event.url,
            &event.referer,
            &event.user_agent,
            &event.country,
            &event.city,
            &event.timezone,
            &event.timestamp,
            &event.visitor_hash,
        ),
    )
    .map_err(|e| {
        tracing::error!("Failed to insert event: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save event")
    })?;

    Ok((StatusCode::ACCEPTED, Json(event)))
}
