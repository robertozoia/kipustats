# blog-analytics-service

A lightweight, self-hosted analytics service for a blog. It collects page view events from a Cloudflare Worker, stores them in SQLite, and provides a password-protected web dashboard with charts and insights.

## Features

- **Event collection** — receives page view events via a REST API from a Cloudflare Worker
- **Web dashboard** — server-rendered HTML with interactive charts (Apache ECharts) and htmx
- **User-agent classification** — automatically categorizes traffic into RSS readers, bots, and browsers
- **RSS reader tracking** — identifies Feedly, Inoreader, NewsBlur, Miniflux, NetNewsWire, Newsboat, Thunderbird, and 15+ other readers
- **Bot detection** — identifies Googlebot, GPTBot, ClaudeBot, social media crawlers, SEO bots, and others
- **Top articles** — most read pages by day, week, or month with date range picker
- **Referrer analysis** — see where traffic is coming from
- **Geographic breakdown** — visitor counts by country and city (using Cloudflare's GeoIP data)
- **Authentication** — bcrypt password protection with signed cookie sessions
- **Single binary** — templates are compiled into the binary at build time; no runtime files needed

## Tech Stack

- [Axum](https://github.com/tokio-rs/axum) — HTTP framework
- [Tokio](https://tokio.rs/) — async runtime
- [Rusqlite](https://github.com/rusqlite/rusqlite) — SQLite bindings (bundled)
- [Askama](https://github.com/askama-rs/askama) — compile-time HTML templates
- [Apache ECharts](https://echarts.apache.org/) — charts (loaded via CDN)
- [htmx](https://htmx.org/) — interactive UI without a JS framework
- [Tracing](https://github.com/tokio-rs/tracing) — structured logging

## Getting Started

### Prerequisites

- Rust 1.85+ (edition 2024)
- Or Docker for containerized deployment

### 1. Clone and configure

```sh
git clone <repo-url>
cd blog-analytics-service
cp analytics.env.example analytics.env
```

### 2. Set a dashboard password

Edit `analytics.env` and set your password:

```
DASHBOARD_PASSWORD=your-secure-password
```

The password is hashed automatically at startup using bcrypt.

### 3. Set a cookie secret

Generate a random secret for signing session cookies:

```sh
openssl rand -hex 32
```

Set it in `analytics.env`:

```
COOKIE_SECRET=your-random-hex-string
```

### 4. Run the service

**Local development:**

```sh
cargo run
```

The server starts on `http://localhost:3001`. If `DASHBOARD_PASSWORD` is not set, the default password is `admin`.

**Production (Docker):**

```sh
docker compose up -d --build
```

The container listens on `127.0.0.1:8082` (intended to sit behind a reverse proxy like Caddy or Nginx).

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `DATABASE_PATH` | No | `./data/analytics.db` | Path to the SQLite database file |
| `DASHBOARD_PASSWORD` | Yes (prod) | Dev default: `admin` | Dashboard password (plaintext, hashed at startup) |
| `DASHBOARD_PASSWORD_HASH` | No | — | Alternative: pre-hashed bcrypt password (use one or the other) |
| `COOKIE_SECRET` | Yes (prod) | Random (per restart) | Secret key for signing session cookies |

## Dashboard

After logging in at `/dashboard/login`, the dashboard provides six views:

| Page | URL | Description |
|---|---|---|
| **Overview** | `/dashboard` | Total pageviews, unique visitors, today's stats, time-series chart, top 10 pages |
| **Articles** | `/dashboard/articles` | Top articles ranked by views with bar chart (excludes RSS feed hits) |
| **RSS** | `/dashboard/rss` | RSS reader breakdown by client, subscriber counts, stacked area chart over time |
| **Bots** | `/dashboard/bots` | Bot traffic summary by name with bar chart |
| **Referrers** | `/dashboard/referrers` | Top referrer URLs with hit counts and unique visitors |
| **Geography** | `/dashboard/geo` | Visitor breakdown by country and city |

All pages include a date range picker. The default range is the last 30 days.

## API

### Public endpoints

These endpoints do not require authentication.

#### `GET /api/v1/health`

Health check. Returns `200 OK` with body `ok`.

#### `POST /api/v1/events`

Accepts a page view event. This is the endpoint the Cloudflare Worker calls.

```json
{
  "url": "https://example.com/post/hello-world/",
  "referer": "https://google.com",
  "user_agent": "Mozilla/5.0 ...",
  "country": "US",
  "city": "New York",
  "timezone": "America/New_York",
  "timestamp": "2025-01-15T10:30:00Z",
  "visitor_hash": "abc123def456..."
}
```

Required fields: `url`, `user_agent`, `timestamp`, `visitor_hash`. The rest are optional.

Returns `202 Accepted` with the event echoed back as JSON.

### Protected endpoints

These endpoints require authentication (session cookie or redirect to login).

All stats endpoints accept optional query parameters:

| Parameter | Default | Description |
|---|---|---|
| `from` | 30 days ago | Start date (inclusive), format `YYYY-MM-DD` |
| `to` | Today | End date (inclusive), format `YYYY-MM-DD` |
| `limit` | Varies | Max number of results (where applicable) |
| `granularity` | `day` | Time grouping: `day`, `week`, or `month` (timeseries only) |

#### `GET /api/v1/stats/overview`

Summary stats: total pageviews, unique visitors, today's counts, top 10 pages.

#### `GET /api/v1/stats/timeseries`

Pageviews and unique visitors over time. Supports `granularity` parameter.

#### `GET /api/v1/stats/articles`

Top articles ranked by views. Excludes RSS feed URLs. Default limit: 50.

#### `GET /api/v1/stats/rss`

RSS reader breakdown: hits and unique subscribers per client, plus daily time-series data.

#### `GET /api/v1/stats/bots`

Bot traffic grouped by bot name.

#### `GET /api/v1/stats/referrers`

Top referrer URLs with hit counts and unique visitors. Default limit: 50.

#### `GET /api/v1/stats/geo`

Country breakdown (all) and top 50 cities with hit counts and unique visitors.

## Project Structure

```
blog-analytics-service/
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── analytics.env.example
├── src/
│   ├── main.rs              # Entrypoint, router setup, auth config
│   ├── db.rs                # Database init, indexes, query functions
│   ├── models.rs            # Data structures for events and API responses
│   ├── classify.rs          # User-agent classification (RSS, bots, browsers)
│   ├── auth.rs              # Authentication middleware, login/logout handlers
│   └── handlers/
│       ├── mod.rs
│       ├── events.rs        # POST /api/v1/events, GET /api/v1/health
│       ├── api.rs           # JSON stats API endpoints
│       └── dashboard.rs     # HTML dashboard handlers (Askama templates)
└── templates/
    ├── base.html             # Shared layout (nav, CSS, JS includes)
    └── dashboard/
        ├── overview.html
        ├── articles.html
        ├── rss.html
        ├── bots.html
        ├── referrers.html
        └── geo.html
```

## Deployment

The service is designed to run behind a reverse proxy (e.g., Caddy, Nginx) that handles TLS.

**Docker Compose** maps the container's port 3001 to `127.0.0.1:8082` on the host. Configure your reverse proxy to forward traffic to this port.

Example Caddy configuration:

```
analytics.example.com {
    reverse_proxy localhost:8082
}
```

### Data persistence

The SQLite database is stored in a Docker volume mapped to `./analytics-data/` on the host. This directory persists across container rebuilds.

## CLI

### `--hash-password`

Reads a password from stdin and prints its bcrypt hash, then exits. Useful for generating the `DASHBOARD_PASSWORD_HASH` value.

```sh
echo -n 'my-password' | cargo run -- --hash-password
# or with the compiled binary:
echo -n 'my-password' | blog-analytics-service --hash-password
```

## License

MIT
