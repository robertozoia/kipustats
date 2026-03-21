use serde::{Deserialize, Serialize};

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

// Query parameter struct shared by all stats endpoints
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<u32>,
    pub granularity: Option<String>,
}

// Response structs for stats API

#[derive(Debug, Serialize)]
pub struct Period {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct PageStats {
    pub url: String,
    pub views: i64,
    pub unique_visitors: i64,
}

#[derive(Debug, Serialize)]
pub struct OverviewResponse {
    pub period: Period,
    pub total_pageviews: i64,
    pub unique_visitors: i64,
    pub today_pageviews: i64,
    pub today_unique_visitors: i64,
    pub top_pages: Vec<PageStats>,
}

#[derive(Debug, Serialize)]
pub struct TimeseriesPoint {
    pub date: String,
    pub views: i64,
    pub unique_visitors: i64,
}

#[derive(Debug, Serialize)]
pub struct TimeseriesResponse {
    pub period: Period,
    pub granularity: String,
    pub data: Vec<TimeseriesPoint>,
}

#[derive(Debug, Serialize)]
pub struct ArticlesResponse {
    pub period: Period,
    pub articles: Vec<PageStats>,
}

#[derive(Debug, Serialize)]
pub struct RssClientStats {
    pub client_name: String,
    pub hits: i64,
    pub unique_subscribers: i64,
}

#[derive(Debug, Serialize)]
pub struct RssTimePoint {
    pub date: String,
    pub client_name: String,
    pub hits: i64,
}

#[derive(Debug, Serialize)]
pub struct RssResponse {
    pub period: Period,
    pub total_hits: i64,
    pub by_client: Vec<RssClientStats>,
    pub over_time: Vec<RssTimePoint>,
}

#[derive(Debug, Serialize)]
pub struct BotStats {
    pub client_name: String,
    pub hits: i64,
}

#[derive(Debug, Serialize)]
pub struct BotsResponse {
    pub period: Period,
    pub total_hits: i64,
    pub bots: Vec<BotStats>,
}

#[derive(Debug, Serialize)]
pub struct ReferrerStats {
    pub referrer: String,
    pub hits: i64,
    pub unique_visitors: i64,
}

#[derive(Debug, Serialize)]
pub struct ReferrersResponse {
    pub period: Period,
    pub referrers: Vec<ReferrerStats>,
}

#[derive(Debug, Serialize)]
pub struct CountryStats {
    pub country: String,
    pub hits: i64,
    pub unique_visitors: i64,
}

#[derive(Debug, Serialize)]
pub struct CityStats {
    pub city: String,
    pub country: String,
    pub hits: i64,
    pub unique_visitors: i64,
}

#[derive(Debug, Serialize)]
pub struct GeoResponse {
    pub period: Period,
    pub countries: Vec<CountryStats>,
    pub cities: Vec<CityStats>,
}

// Internal struct for user-agent grouped rows from DB
#[derive(Debug)]
pub struct UserAgentGroup {
    pub user_agent: String,
    pub hits: i64,
    pub unique_visitors: i64,
}

#[derive(Debug)]
pub struct UserAgentDayGroup {
    pub day: String,
    pub user_agent: String,
    pub hits: i64,
}
