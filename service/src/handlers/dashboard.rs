use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use chrono::Local;
use std::collections::{BTreeSet, HashMap};

use crate::classify::{classify, ClientType};
use crate::db::{self, AppState};
use crate::models::*;

fn render<T: Template>(tmpl: T) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    tmpl.render()
        .map(Html)
        .map_err(|e| {
            tracing::error!("Template render failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Render failed")
        })
}

/// Resolve from/to query params. Defaults: last 30 days.
fn resolve_period(q: &StatsQuery) -> (String, String, String, String) {
    let today = Local::now().date_naive();
    let from = q
        .from
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| today - chrono::Duration::days(30));
    let to_inclusive = q
        .to
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or(today);
    let to_exclusive = to_inclusive + chrono::Duration::days(1);

    (
        from.format("%Y-%m-%d").to_string(),
        to_exclusive.format("%Y-%m-%d").to_string(),
        from.format("%Y-%m-%d").to_string(),
        to_inclusive.format("%Y-%m-%d").to_string(),
    )
}

// --- Overview ---

#[derive(Template)]
#[template(path = "dashboard/overview.html")]
pub struct OverviewTemplate {
    pub active_page: String,
    pub from: String,
    pub to: String,
    pub total_pageviews: i64,
    pub unique_visitors: i64,
    pub today_pageviews: i64,
    pub today_unique_visitors: i64,
    pub top_pages: Vec<PageStats>,
    pub timeseries: Vec<TimeseriesPoint>,
}

pub async fn dashboard_overview(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from_q, to_q, from_display, to_display) = resolve_period(&q);
    let conn = state.db.lock().unwrap();

    let (total_pageviews, unique_visitors) =
        db::query_count(&conn, &from_q, &to_q).map_err(db_err)?;

    let today = Local::now().date_naive();
    let today_str = today.format("%Y-%m-%d").to_string();
    let tomorrow_str = (today + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let (today_pageviews, today_unique_visitors) =
        db::query_count(&conn, &today_str, &tomorrow_str).map_err(db_err)?;

    let top_pages = db::query_top_pages(&conn, &from_q, &to_q, 10).map_err(db_err)?;
    let timeseries = db::query_timeseries(&conn, &from_q, &to_q, "day").map_err(db_err)?;

    render(OverviewTemplate {
        active_page: "overview".to_string(),
        from: from_display,
        to: to_display,
        total_pageviews,
        unique_visitors,
        today_pageviews,
        today_unique_visitors,
        top_pages,
        timeseries,
    })
}

// --- Articles ---

#[derive(Template)]
#[template(path = "dashboard/articles.html")]
pub struct ArticlesTemplate {
    pub active_page: String,
    pub from: String,
    pub to: String,
    pub articles: Vec<PageStats>,
    pub chart_height: u32,
}

pub async fn dashboard_articles(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from_q, to_q, from_display, to_display) = resolve_period(&q);
    let limit = q.limit.unwrap_or(30);
    let conn = state.db.lock().unwrap();

    let articles = db::query_articles(&conn, &from_q, &to_q, limit).map_err(db_err)?;
    let chart_height = (articles.len() as u32 * 28).max(200).min(800);

    render(ArticlesTemplate {
        active_page: "articles".to_string(),
        from: from_display,
        to: to_display,
        articles,
        chart_height,
    })
}

// --- RSS ---

#[derive(Template)]
#[template(path = "dashboard/rss.html")]
pub struct RssTemplate {
    pub active_page: String,
    pub from: String,
    pub to: String,
    pub total_hits: i64,
    pub total_subscribers: i64,
    pub client_count: usize,
    pub by_client: Vec<RssClientStats>,
    pub timeline_dates_json: String,
    pub timeline_series_json: String,
}

pub async fn dashboard_rss(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from_q, to_q, from_display, to_display) = resolve_period(&q);
    let conn = state.db.lock().unwrap();

    // By client
    let ua_groups = db::query_by_user_agent(&conn, &from_q, &to_q).map_err(db_err)?;
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
    let total_subscribers: i64 = by_client_map.values().map(|(_, s)| s).sum();

    let mut by_client: Vec<RssClientStats> = by_client_map
        .into_iter()
        .map(|(name, (hits, unique_subscribers))| RssClientStats {
            client_name: name,
            hits,
            unique_subscribers,
        })
        .collect();
    by_client.sort_by(|a, b| b.hits.cmp(&a.hits));

    // Timeline data
    let daily_groups = db::query_by_user_agent_daily(&conn, &from_q, &to_q).map_err(db_err)?;
    let mut daily_map: HashMap<(String, String), i64> = HashMap::new();
    let mut all_dates: BTreeSet<String> = BTreeSet::new();
    let mut all_clients: BTreeSet<String> = BTreeSet::new();

    for group in &daily_groups {
        let info = classify(&group.user_agent);
        if info.client_type == ClientType::RssReader {
            all_dates.insert(group.day.clone());
            all_clients.insert(info.client_name.clone());
            *daily_map
                .entry((group.day.clone(), info.client_name))
                .or_insert(0) += group.hits;
        }
    }

    let dates_vec: Vec<String> = all_dates.into_iter().collect();
    let timeline_dates_json = serde_json::to_string(&dates_vec).unwrap_or_else(|_| "[]".to_string());

    // Build ECharts series array
    let mut series_arr: Vec<serde_json::Value> = Vec::new();
    for client in &all_clients {
        let data: Vec<i64> = dates_vec
            .iter()
            .map(|d| {
                daily_map
                    .get(&(d.clone(), client.clone()))
                    .copied()
                    .unwrap_or(0)
            })
            .collect();
        series_arr.push(serde_json::json!({
            "name": client,
            "type": "line",
            "stack": "total",
            "areaStyle": {},
            "data": data
        }));
    }
    let timeline_series_json =
        serde_json::to_string(&series_arr).unwrap_or_else(|_| "[]".to_string());

    let client_count = by_client.len();

    render(RssTemplate {
        active_page: "rss".to_string(),
        from: from_display,
        to: to_display,
        total_hits,
        total_subscribers,
        client_count,
        by_client,
        timeline_dates_json,
        timeline_series_json,
    })
}

// --- Bots ---

#[derive(Template)]
#[template(path = "dashboard/bots.html")]
pub struct BotsTemplate {
    pub active_page: String,
    pub from: String,
    pub to: String,
    pub total_hits: i64,
    pub bot_count: usize,
    pub bots: Vec<BotStats>,
    pub chart_height: u32,
}

pub async fn dashboard_bots(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from_q, to_q, from_display, to_display) = resolve_period(&q);
    let conn = state.db.lock().unwrap();

    let ua_groups = db::query_by_user_agent(&conn, &from_q, &to_q).map_err(db_err)?;
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

    let bot_count = bots.len();
    let chart_height = (bot_count as u32 * 28).max(200).min(600);

    render(BotsTemplate {
        active_page: "bots".to_string(),
        from: from_display,
        to: to_display,
        total_hits,
        bot_count,
        bots,
        chart_height,
    })
}

// --- Referrers ---

#[derive(Template)]
#[template(path = "dashboard/referrers.html")]
pub struct ReferrersTemplate {
    pub active_page: String,
    pub from: String,
    pub to: String,
    pub referrers: Vec<ReferrerStats>,
    pub chart_height: u32,
}

pub async fn dashboard_referrers(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from_q, to_q, from_display, to_display) = resolve_period(&q);
    let limit = q.limit.unwrap_or(30);
    let conn = state.db.lock().unwrap();

    let referrers = db::query_referrers(&conn, &from_q, &to_q, limit).map_err(db_err)?;
    let chart_height = (referrers.len() as u32 * 28).max(200).min(800);

    render(ReferrersTemplate {
        active_page: "referrers".to_string(),
        from: from_display,
        to: to_display,
        referrers,
        chart_height,
    })
}

// --- Geography ---

#[derive(Template)]
#[template(path = "dashboard/geo.html")]
pub struct GeoTemplate {
    pub active_page: String,
    pub from: String,
    pub to: String,
    pub countries: Vec<CountryStats>,
    pub cities: Vec<CityStats>,
    pub chart_height: u32,
}

pub async fn dashboard_geo(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (from_q, to_q, from_display, to_display) = resolve_period(&q);
    let conn = state.db.lock().unwrap();

    let countries = db::query_countries(&conn, &from_q, &to_q).map_err(db_err)?;
    let cities = db::query_cities(&conn, &from_q, &to_q, 50).map_err(db_err)?;
    let chart_height = (countries.len() as u32 * 28).max(200).min(600);

    render(GeoTemplate {
        active_page: "geo".to_string(),
        from: from_display,
        to: to_display,
        countries,
        cities,
        chart_height,
    })
}

fn db_err(e: rusqlite::Error) -> (StatusCode, &'static str) {
    tracing::error!("Database query failed: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "Query failed")
}
