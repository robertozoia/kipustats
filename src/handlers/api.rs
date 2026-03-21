use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{Local, NaiveDate};
use std::collections::HashMap;

use crate::classify::{classify, ClientType};
use crate::db::{self, AppState};
use crate::models::*;

/// Resolve from/to query params into ISO date strings.
/// Defaults: from = 30 days ago, to = tomorrow (exclusive upper bound).
fn resolve_period(q: &StatsQuery) -> (String, String, Period) {
    let today = Local::now().date_naive();

    let from = q
        .from
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| today - chrono::Duration::days(30));

    let to_inclusive = q
        .to
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or(today);

    // Use exclusive upper bound for queries (to + 1 day)
    let to_exclusive = to_inclusive + chrono::Duration::days(1);

    let period = Period {
        from: from.format("%Y-%m-%d").to_string(),
        to: to_inclusive.format("%Y-%m-%d").to_string(),
    };

    (
        from.format("%Y-%m-%d").to_string(),
        to_exclusive.format("%Y-%m-%d").to_string(),
        period,
    )
}

pub async fn stats_overview(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from, to, period) = resolve_period(&q);
    let db = state.db.lock().unwrap();

    let (total_pageviews, unique_visitors) =
        db::query_count(&db, &from, &to).map_err(|e| {
            tracing::error!("query_count failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
        })?;

    // Today's counts
    let today = Local::now().date_naive();
    let today_str = today.format("%Y-%m-%d").to_string();
    let tomorrow_str = (today + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let (today_pageviews, today_unique_visitors) =
        db::query_count(&db, &today_str, &tomorrow_str).map_err(|e| {
            tracing::error!("query_count (today) failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
        })?;

    let top_pages = db::query_top_pages(&db, &from, &to, 10).map_err(|e| {
        tracing::error!("query_top_pages failed: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
    })?;

    Ok(Json(OverviewResponse {
        period,
        total_pageviews,
        unique_visitors,
        today_pageviews,
        today_unique_visitors,
        top_pages,
    }))
}

pub async fn stats_timeseries(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from, to, period) = resolve_period(&q);
    let granularity = q.granularity.as_deref().unwrap_or("day");
    let db = state.db.lock().unwrap();

    let data = db::query_timeseries(&db, &from, &to, granularity).map_err(|e| {
        tracing::error!("query_timeseries failed: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
    })?;

    Ok(Json(TimeseriesResponse {
        period,
        granularity: granularity.to_string(),
        data,
    }))
}

pub async fn stats_articles(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from, to, period) = resolve_period(&q);
    let limit = q.limit.unwrap_or(50);
    let db = state.db.lock().unwrap();

    let articles = db::query_articles(&db, &from, &to, limit).map_err(|e| {
        tracing::error!("query_articles failed: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
    })?;

    Ok(Json(ArticlesResponse { period, articles }))
}

pub async fn stats_rss(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from, to, period) = resolve_period(&q);
    let db = state.db.lock().unwrap();

    // Aggregate by user_agent, then classify in Rust
    let ua_groups = db::query_by_user_agent(&db, &from, &to).map_err(|e| {
        tracing::error!("query_by_user_agent failed: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
    })?;

    let mut by_client_map: HashMap<String, (i64, i64)> = HashMap::new();
    for group in &ua_groups {
        let info = classify(&group.user_agent);
        if info.client_type == ClientType::RssReader {
            let entry = by_client_map.entry(info.client_name).or_insert((0, 0));
            entry.0 += group.hits;
            entry.1 += group.unique_visitors;
        }
    }

    let total_hits: i64 = by_client_map.values().map(|(h, _)| h).sum();

    let mut by_client: Vec<RssClientStats> = by_client_map
        .into_iter()
        .map(|(name, (hits, unique_subscribers))| RssClientStats {
            client_name: name,
            hits,
            unique_subscribers,
        })
        .collect();
    by_client.sort_by(|a, b| b.hits.cmp(&a.hits));

    // Daily breakdown
    let daily_groups = db::query_by_user_agent_daily(&db, &from, &to).map_err(|e| {
        tracing::error!("query_by_user_agent_daily failed: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
    })?;

    let mut over_time: Vec<RssTimePoint> = Vec::new();
    let mut daily_map: HashMap<(String, String), i64> = HashMap::new();
    for group in &daily_groups {
        let info = classify(&group.user_agent);
        if info.client_type == ClientType::RssReader {
            *daily_map
                .entry((group.day.clone(), info.client_name))
                .or_insert(0) += group.hits;
        }
    }
    for ((date, client_name), hits) in daily_map {
        over_time.push(RssTimePoint {
            date,
            client_name,
            hits,
        });
    }
    over_time.sort_by(|a, b| a.date.cmp(&b.date).then(a.client_name.cmp(&b.client_name)));

    Ok(Json(RssResponse {
        period,
        total_hits,
        by_client,
        over_time,
    }))
}

pub async fn stats_bots(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from, to, period) = resolve_period(&q);
    let db = state.db.lock().unwrap();

    let ua_groups = db::query_by_user_agent(&db, &from, &to).map_err(|e| {
        tracing::error!("query_by_user_agent failed: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
    })?;

    let mut bot_map: HashMap<String, i64> = HashMap::new();
    for group in &ua_groups {
        let info = classify(&group.user_agent);
        if info.client_type == ClientType::Bot {
            *bot_map.entry(info.client_name).or_insert(0) += group.hits;
        }
    }

    let total_hits: i64 = bot_map.values().sum();

    let mut bots: Vec<BotStats> = bot_map
        .into_iter()
        .map(|(client_name, hits)| BotStats { client_name, hits })
        .collect();
    bots.sort_by(|a, b| b.hits.cmp(&a.hits));

    Ok(Json(BotsResponse {
        period,
        total_hits,
        bots,
    }))
}

pub async fn stats_referrers(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from, to, period) = resolve_period(&q);
    let limit = q.limit.unwrap_or(50);
    let db = state.db.lock().unwrap();

    let referrers = db::query_referrers(&db, &from, &to, limit).map_err(|e| {
        tracing::error!("query_referrers failed: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
    })?;

    Ok(Json(ReferrersResponse { period, referrers }))
}

pub async fn stats_geo(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from, to, period) = resolve_period(&q);
    let db = state.db.lock().unwrap();

    let countries = db::query_countries(&db, &from, &to).map_err(|e| {
        tracing::error!("query_countries failed: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
    })?;

    let cities = db::query_cities(&db, &from, &to, 50).map_err(|e| {
        tracing::error!("query_cities failed: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
    })?;

    Ok(Json(GeoResponse {
        period,
        countries,
        cities,
    }))
}
