use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use crate::auth::AuthConfig;
use crate::models::*;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub auth: AuthConfig,
    pub auth_token: Option<String>,
}

pub fn init_db() -> rusqlite::Result<Connection> {
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "./data/analytics.db".to_string());

    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).expect("Failed to create data directory");
    }

    let conn = Connection::open(&db_path)?;

    // Enable WAL mode for better concurrent read/write performance
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;

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

    // Add indexes for query performance
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events (timestamp);
         CREATE INDEX IF NOT EXISTS idx_events_url ON events (url);
         CREATE INDEX IF NOT EXISTS idx_events_user_agent ON events (user_agent);
         CREATE INDEX IF NOT EXISTS idx_events_visitor_hash_timestamp ON events (visitor_hash, timestamp);",
    )?;

    Ok(conn)
}

// --- Query functions ---

pub fn query_count(conn: &Connection, from: &str, to: &str) -> rusqlite::Result<(i64, i64)> {
    let mut stmt = conn.prepare(
        "SELECT COUNT(*), COUNT(DISTINCT visitor_hash)
         FROM events WHERE timestamp >= ?1 AND timestamp < ?2",
    )?;
    stmt.query_row((from, to), |row| Ok((row.get(0)?, row.get(1)?)))
}

pub fn query_top_pages(
    conn: &Connection,
    from: &str,
    to: &str,
    limit: u32,
) -> rusqlite::Result<Vec<PageStats>> {
    let mut stmt = conn.prepare(
        "SELECT url, COUNT(*) as views, COUNT(DISTINCT visitor_hash) as uv
         FROM events WHERE timestamp >= ?1 AND timestamp < ?2
         GROUP BY url ORDER BY views DESC LIMIT ?3",
    )?;
    let rows = stmt.query_map((from, to, limit), |row| {
        Ok(PageStats {
            url: row.get(0)?,
            views: row.get(1)?,
            unique_visitors: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn query_timeseries(
    conn: &Connection,
    from: &str,
    to: &str,
    granularity: &str,
) -> rusqlite::Result<Vec<TimeseriesPoint>> {
    let group_expr = match granularity {
        "week" => "strftime('%Y-W%W', timestamp)",
        "month" => "strftime('%Y-%m', timestamp)",
        _ => "DATE(timestamp)",
    };

    let sql = format!(
        "SELECT {group_expr} as period, COUNT(*) as views, COUNT(DISTINCT visitor_hash) as uv
         FROM events WHERE timestamp >= ?1 AND timestamp < ?2
         GROUP BY period ORDER BY period"
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map((from, to), |row| {
        Ok(TimeseriesPoint {
            date: row.get(0)?,
            views: row.get(1)?,
            unique_visitors: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn query_articles(
    conn: &Connection,
    from: &str,
    to: &str,
    limit: u32,
) -> rusqlite::Result<Vec<PageStats>> {
    let mut stmt = conn.prepare(
        "SELECT url, COUNT(*) as views, COUNT(DISTINCT visitor_hash) as uv
         FROM events WHERE timestamp >= ?1 AND timestamp < ?2
           AND url NOT LIKE '%/index.xml%'
         GROUP BY url ORDER BY views DESC LIMIT ?3",
    )?;
    let rows = stmt.query_map((from, to, limit), |row| {
        Ok(PageStats {
            url: row.get(0)?,
            views: row.get(1)?,
            unique_visitors: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn query_by_user_agent(
    conn: &Connection,
    from: &str,
    to: &str,
) -> rusqlite::Result<Vec<UserAgentGroup>> {
    let mut stmt = conn.prepare(
        "SELECT user_agent, COUNT(*) as hits, COUNT(DISTINCT visitor_hash) as uv
         FROM events WHERE timestamp >= ?1 AND timestamp < ?2
         GROUP BY user_agent",
    )?;
    let rows = stmt.query_map((from, to), |row| {
        Ok(UserAgentGroup {
            user_agent: row.get(0)?,
            hits: row.get(1)?,
            unique_visitors: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn query_by_user_agent_daily(
    conn: &Connection,
    from: &str,
    to: &str,
) -> rusqlite::Result<Vec<UserAgentDayGroup>> {
    let mut stmt = conn.prepare(
        "SELECT DATE(timestamp) as day, user_agent, COUNT(*) as hits
         FROM events WHERE timestamp >= ?1 AND timestamp < ?2
         GROUP BY day, user_agent",
    )?;
    let rows = stmt.query_map((from, to), |row| {
        Ok(UserAgentDayGroup {
            day: row.get(0)?,
            user_agent: row.get(1)?,
            hits: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn query_referrers(
    conn: &Connection,
    from: &str,
    to: &str,
    limit: u32,
) -> rusqlite::Result<Vec<ReferrerStats>> {
    let mut stmt = conn.prepare(
        "SELECT referer, COUNT(*) as hits, COUNT(DISTINCT visitor_hash) as uv
         FROM events WHERE timestamp >= ?1 AND timestamp < ?2
           AND referer IS NOT NULL AND referer != ''
         GROUP BY referer ORDER BY hits DESC LIMIT ?3",
    )?;
    let rows = stmt.query_map((from, to, limit), |row| {
        Ok(ReferrerStats {
            referrer: row.get(0)?,
            hits: row.get(1)?,
            unique_visitors: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn query_countries(
    conn: &Connection,
    from: &str,
    to: &str,
) -> rusqlite::Result<Vec<CountryStats>> {
    let mut stmt = conn.prepare(
        "SELECT country, COUNT(*) as hits, COUNT(DISTINCT visitor_hash) as uv
         FROM events WHERE timestamp >= ?1 AND timestamp < ?2
           AND country IS NOT NULL AND country != ''
         GROUP BY country ORDER BY hits DESC",
    )?;
    let rows = stmt.query_map((from, to), |row| {
        Ok(CountryStats {
            country: row.get(0)?,
            hits: row.get(1)?,
            unique_visitors: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn query_cities(
    conn: &Connection,
    from: &str,
    to: &str,
    limit: u32,
) -> rusqlite::Result<Vec<CityStats>> {
    let mut stmt = conn.prepare(
        "SELECT city, country, COUNT(*) as hits, COUNT(DISTINCT visitor_hash) as uv
         FROM events WHERE timestamp >= ?1 AND timestamp < ?2
           AND city IS NOT NULL AND city != ''
         GROUP BY city, country ORDER BY hits DESC LIMIT ?3",
    )?;
    let rows = stmt.query_map((from, to, limit), |row| {
        Ok(CityStats {
            city: row.get(0)?,
            country: row.get(1)?,
            hits: row.get(2)?,
            unique_visitors: row.get(3)?,
        })
    })?;
    rows.collect()
}
