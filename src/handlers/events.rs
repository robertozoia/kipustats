use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::db::AppState;
use crate::models::AnalyticsEvent;

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

#[axum::debug_handler]
pub async fn track_event(
    State(state): State<AppState>,
    Json(event): Json<AnalyticsEvent>,
) -> Result<(StatusCode, Json<AnalyticsEvent>), (StatusCode, &'static str)> {
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
