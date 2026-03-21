mod auth;
mod classify;
mod db;
mod handlers;
mod models;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use std::sync::{Arc, Mutex};
use tower_cookies::CookieManagerLayer;

use auth::AuthConfig;
use db::AppState;
use handlers::api;
use handlers::dashboard;
use handlers::events::{health_check, track_event};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Handle --hash-password flag: read password from stdin, print bcrypt hash, exit
    if std::env::args().any(|a| a == "--hash-password") {
        let mut password = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut password)?;
        let password = password.trim_end();
        if password.is_empty() {
            eprintln!("Error: no password provided. Usage: echo -n 'yourpassword' | blog-analytics-service --hash-password");
            std::process::exit(1);
        }
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        println!("{hash}");
        return Ok(());
    }

    tracing_subscriber::fmt::init();

    let conn = db::init_db()?;

    // Read auth config from environment.
    // DASHBOARD_PASSWORD (plaintext, hashed at startup) is the recommended approach.
    // DASHBOARD_PASSWORD_HASH (pre-hashed bcrypt) is also supported.
    let password_hash = if let Ok(password) = std::env::var("DASHBOARD_PASSWORD") {
        tracing::info!("Hashing DASHBOARD_PASSWORD at startup");
        bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .expect("Failed to hash DASHBOARD_PASSWORD")
    } else if let Ok(hash) = std::env::var("DASHBOARD_PASSWORD_HASH") {
        hash
    } else {
        tracing::warn!("DASHBOARD_PASSWORD not set, using default dev password 'admin'");
        bcrypt::hash("admin", bcrypt::DEFAULT_COST).unwrap()
    };

    let cookie_secret = std::env::var("COOKIE_SECRET")
        .unwrap_or_else(|_| {
            tracing::warn!("COOKIE_SECRET not set, using random secret (sessions won't survive restarts)");
            use std::collections::hash_map::RandomState;
            use std::hash::{BuildHasher, Hasher};
            let s = RandomState::new();
            format!("{:016x}{:016x}{:016x}{:016x}",
                s.build_hasher().finish(),
                s.build_hasher().finish(),
                s.build_hasher().finish(),
                s.build_hasher().finish(),
            )
        });

    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
        auth: AuthConfig {
            password_hash,
            cookie_secret: cookie_secret.into_bytes(),
        },
    };

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/events", post(track_event))
        .route("/dashboard/login", get(auth::login_page))
        .route("/dashboard/login", post(auth::login_submit));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        .route("/api/v1/stats/overview", get(api::stats_overview))
        .route("/api/v1/stats/timeseries", get(api::stats_timeseries))
        .route("/api/v1/stats/articles", get(api::stats_articles))
        .route("/api/v1/stats/rss", get(api::stats_rss))
        .route("/api/v1/stats/bots", get(api::stats_bots))
        .route("/api/v1/stats/referrers", get(api::stats_referrers))
        .route("/api/v1/stats/geo", get(api::stats_geo))
        // Dashboard HTML routes
        .route("/dashboard", get(dashboard::dashboard_overview))
        .route("/dashboard/articles", get(dashboard::dashboard_articles))
        .route("/dashboard/rss", get(dashboard::dashboard_rss))
        .route("/dashboard/bots", get(dashboard::dashboard_bots))
        .route("/dashboard/referrers", get(dashboard::dashboard_referrers))
        .route("/dashboard/geo", get(dashboard::dashboard_geo))
        .route("/dashboard/logout", get(auth::logout))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(CookieManagerLayer::new())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await
        .expect("Failed to bind port 3001.");

    println!("-> Server listening on http://localhost:3001");

    axum::serve(listener, app).await?;

    Ok(())
}
