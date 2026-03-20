# blog-analytics-service

A lightweight analytics event collection service for a blog, built with Rust. It exposes a REST API to receive page view events and stores them in a local SQLite database.

## Tech Stack

- [Axum](https://github.com/tokio-rs/axum) — HTTP framework
- [Tokio](https://tokio.rs/) — async runtime
- [Rusqlite](https://github.com/nickel-org/rusqlite) — SQLite bindings (bundled)
- [Tracing](https://github.com/tokio-rs/tracing) — structured logging

## API

### `GET /api/v1/health`

Returns `200 OK` with body `ok`.

### `POST /api/v1/events`

Accepts a JSON body with the following fields:

```json
{
  "url": "https://example.com/post/hello-world",
  "referer": "https://google.com",
  "user_agent": "Mozilla/5.0 ...",
  "country": "US",
  "city": "New York",
  "timezone": "America/New_York",
  "timestamp": "2025-01-15T10:30:00Z",
  "visitor_hash": "abc123"
}
```

Required fields: `url`, `user_agent`, `timestamp`, `visitor_hash`. The rest are optional.

Returns `202 Accepted` with the event echoed back as JSON.

## Getting Started

```sh
# Build
cargo build --release

# Run (listens on port 3001)
cargo run
```

The server stores data in `./data/analytics.db`, creating the directory and database automatically on first run.
