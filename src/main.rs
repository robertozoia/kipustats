
use axum::{
    extract::{Json, State},
    routing::{get, post},
    Router,
    http::StatusCode,
    response::IntoResponse,
};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// Define the App State to hold the DB Connection
#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub url: String,
    pub referer: Option<String>,
    pub user_agent: String,
    pub country: Option<String>,
    pub city: Option<String>,
    pub timezone: Option<String>,
    pub timestamp: String,
    pub visitor_hash: String,
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    tracing_subscriber::fmt::init();

    // Initialize the DB
    // We call this synchronously in main before the async server starts

    let conn = init_db()?;

    // create the shared state
    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
    };

    let app = Router::new()
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/events", post(track_event))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await
        .expect("Failed to bind port 3001.");

    println!("-> Server listening on http://localhost:3001");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse{
    (StatusCode::OK, "ok")
}

// a handler that accepts state
#[axum::debug_handler]
async fn track_event(
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

    // lock the db and insert the data
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
    ).map_err(|e| {
        tracing::error!("Failed to insert event: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save event")
    })?;

    Ok((StatusCode::ACCEPTED, Json(event)))
}

pub fn init_db() -> rusqlite::Result<Connection> {
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "./data/analytics.db".to_string());

    // make sure directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).expect("Failed to create data directory");
    }

    let conn = Connection::open(&db_path)?;

    conn.execute(
       "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT NOT NULL,
            referer TEXT,
            user_agent TEXT NOT NULL,
            country TEXT,
            city TEXT,
            timezone TEXT,
            timestamp TEXT NOT NULL,
            visitor_hash TEXT NOT NULL
        )",
        [],    
    )?;

    Ok(conn)
}

