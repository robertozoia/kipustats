# Blog Analytics

A privacy-focused, self-hosted analytics system for a blog. It consists of two components that work together: a **Cloudflare Worker** that captures page views at the edge, and a **Rust backend service** that stores events in SQLite and serves a password-protected dashboard.

```
                        ┌────────────────────┐
   Visitor ──request──▶ │  Cloudflare Worker  │ ──response──▶ Visitor
                        │  (worker/)          │
                        └────────┬───────────┘
                                 │ POST /api/v1/events
                                 ▼
                        ┌────────────────────┐
                        │  Analytics Service  │
                        │  (service/)         │
                        │  ┌──────────────┐   │
                        │  │   SQLite DB   │  │
                        │  └──────────────┘   │
                        │  ┌──────────────┐   │
                        │  │  Dashboard UI │  │
                        │  └──────────────┘   │
                        └────────────────────┘
```

## How It Works

1. The **Cloudflare Worker** sits in front of the blog (configured as a route on the domain). Every incoming request passes through it. For trackable pages (paths ending in `/` or `/index.xml`), the worker extracts metadata — URL, referer, user-agent, country, city, and timezone (from Cloudflare's GeoIP) — and creates a privacy-preserving visitor hash (SHA-256 of the IP + current date, rotated daily). It then sends this event asynchronously to the analytics service via `POST /api/v1/events`, without delaying the response to the visitor.

2. The **Analytics Service** receives events, classifies the user-agent (browser, RSS reader, or bot), and stores everything in SQLite. It serves a web dashboard with six views: Overview, Articles, RSS, Bots, Referrers, and Geography. All views support date range filtering.

## Repository Structure

```
blog-analytics/
├── service/                     # Rust backend (Axum + SQLite)
│   ├── src/
│   │   ├── main.rs              # Entrypoint, router, auth config
│   │   ├── db.rs                # Database init and query functions
│   │   ├── models.rs            # Data structures
│   │   ├── classify.rs          # User-agent classification
│   │   ├── auth.rs              # Authentication middleware
│   │   └── handlers/
│   │       ├── events.rs        # Event ingestion + health check
│   │       ├── api.rs           # JSON stats API
│   │       └── dashboard.rs     # HTML dashboard (Askama templates)
│   ├── templates/               # Server-rendered HTML templates
│   ├── Cargo.toml
│   ├── Dockerfile
│   ├── docker-compose.yml
│   └── analytics.env.example
│
└── worker/                      # Cloudflare Worker (TypeScript)
    ├── src/
    │   └── index.ts             # Worker entry point
    ├── test/
    ├── wrangler.jsonc            # Cloudflare routes and config
    └── package.json
```

## Setup

### Prerequisites

- **Service:** Rust 1.85+ or Docker
- **Worker:** Node.js 18+ and a Cloudflare account with Workers enabled

### 1. Analytics Service

```sh
cd service
cp analytics.env.example analytics.env
```

Edit `analytics.env` and set:

```sh
# Pick a strong password for the dashboard
DASHBOARD_PASSWORD=your-secure-password

# Generate a random secret for session cookies
# openssl rand -hex 32
COOKIE_SECRET=your-random-hex-string
```

**Run locally (development):**

```sh
cargo run
```

The server starts at `http://localhost:3001`. If `DASHBOARD_PASSWORD` is not set, the default is `admin`.

**Run with Docker (production):**

```sh
docker compose up -d --build
```

The container listens on `127.0.0.1:8082` — put it behind a reverse proxy (Caddy, Nginx, etc.) that handles TLS.

Example Caddy config:

```
analytics.yourdomain.com {
    reverse_proxy localhost:8082
}
```

### 2. Cloudflare Worker

```sh
cd worker
npm install
```

Edit `wrangler.jsonc` to set your domain routes and analytics endpoint:

```jsonc
{
    "vars": {
        "ANALYTICS_ENDPOINT": "https://analytics.yourdomain.com/api/v1/events"
    },
    "routes": [
        { "pattern": "yourdomain.com/*", "zone_name": "yourdomain.com" }
    ]
}
```

**Local development:**

```sh
npm run dev
```

**Deploy to Cloudflare:**

```sh
npm run deploy
```

## Environment Variables (Service)

| Variable | Required | Default | Description |
|---|---|---|---|
| `DASHBOARD_PASSWORD` | Yes (prod) | `admin` | Dashboard password (plaintext, hashed at startup) |
| `DASHBOARD_PASSWORD_HASH` | No | — | Alternative: pre-hashed bcrypt password |
| `COOKIE_SECRET` | Yes (prod) | Random | Secret for signing session cookies |
| `DATABASE_PATH` | No | `./data/analytics.db` | Path to the SQLite database file |

## Dashboard

After logging in at `/dashboard/login`, the dashboard provides:

| View | URL | Description |
|---|---|---|
| **Overview** | `/dashboard` | Total pageviews, unique visitors, time-series chart, top pages |
| **Articles** | `/dashboard/articles` | Top articles ranked by views (excludes RSS) |
| **RSS** | `/dashboard/rss` | RSS reader breakdown, subscriber counts, stacked area chart |
| **Bots** | `/dashboard/bots` | Bot traffic by name |
| **Referrers** | `/dashboard/referrers` | Top referrer URLs with hit counts |
| **Geography** | `/dashboard/geo` | Visitor breakdown by country and city |

All views include a date range picker. The default range is the last 30 days.

## API

### Public Endpoints

**`GET /api/v1/health`** — Returns `200 OK` with body `ok`.

**`POST /api/v1/events`** — Accepts a page view event (called by the worker):

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

Required fields: `url`, `user_agent`, `timestamp`, `visitor_hash`. Returns `202 Accepted`.

### Protected Endpoints

All stats endpoints require authentication and accept optional query parameters: `from`, `to` (dates in `YYYY-MM-DD`), `limit`, and `granularity` (`day`, `week`, `month`).

| Endpoint | Description |
|---|---|
| `GET /api/v1/stats/overview` | Summary stats and top pages |
| `GET /api/v1/stats/timeseries` | Pageviews and visitors over time |
| `GET /api/v1/stats/articles` | Top articles by views |
| `GET /api/v1/stats/rss` | RSS reader breakdown |
| `GET /api/v1/stats/bots` | Bot traffic by name |
| `GET /api/v1/stats/referrers` | Top referrer URLs |
| `GET /api/v1/stats/geo` | Country and city breakdown |

## Tech Stack

**Service:** Rust, [Axum](https://github.com/tokio-rs/axum), [Tokio](https://tokio.rs/), [Rusqlite](https://github.com/rusqlite/rusqlite) (bundled SQLite), [Askama](https://github.com/askama-rs/askama) templates, [Apache ECharts](https://echarts.apache.org/), [htmx](https://htmx.org/)

**Worker:** TypeScript, [Cloudflare Workers](https://developers.cloudflare.com/workers/), [Wrangler](https://developers.cloudflare.com/workers/wrangler/)

## CLI Utilities

**Hash a password** (for generating `DASHBOARD_PASSWORD_HASH`):

```sh
echo -n 'my-password' | cargo run -- --hash-password
```

## Data Persistence

The SQLite database is stored in a Docker volume mapped to `./analytics-data/` on the host. This directory persists across container rebuilds.

## Privacy

- Visitor IPs are never stored. The worker hashes the IP with the current date (SHA-256) to produce a daily-rotating identifier that cannot be reversed back to the original IP.
- No cookies are set on visitors. Only the dashboard uses session cookies for authentication.
- All data stays in your own SQLite database.

## License

MIT
