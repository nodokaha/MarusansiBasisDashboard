# MarusansiBasisDashboard

A lightweight Rust/Axum dashboard for monitoring and managing a Basis Server.

## Features

- Live server health monitor with player count, capacity, version, traffic counters, and timestamps.
- Admin panel for connected-player lookup, direct announcements, world loading/unloading, synchronized world switching, and broadcast announcements.
- Japanese and English admin interface. The first visit follows the browser language; choose a language from the selector to save the preference in that browser.
- Player search, responsive tables, optional five-second auto-refresh, URL/delay validation, and visible request-error feedback.
- Docker image and Docker Compose deployment support.

## Live Demo

http://maru.marusankakusikakuonline.com/

## Endpoints

- `/` — health dashboard
- `/api/health` — HTML health card
- `/api/health/json` — JSON health data
- `/admin` — administrative panel

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `PORT` | `4000` | Dashboard HTTP port |
| `HEALTH_PORT` | `10666` | Basis health endpoint port |
| `API_URL_BASE` | `http://localhost:10667` | Basis admin API base URL |
| `API_BEARER_TOKEN` | `default_token` | Bearer token used for internal Basis API requests |
| `RUST_LOG` | `info` | Rust tracing filter |

## Docker Compose

See [compose.yaml](./compose.yaml) for a working example. The dashboard shares the Basis Server network namespace so it can reach the health and admin API ports on localhost.

```bash
docker compose up -d --build
docker compose logs -f marusansi-basis-dashboard
```

## Security

The dashboard's administrative routes can issue commands to the Basis Server. Do not expose `/admin` directly to the public internet without an authentication layer and HTTPS (for example, an authenticated reverse proxy or private VPN). `API_BEARER_TOKEN` authenticates dashboard-to-Basis API requests; it does **not** itself authenticate visitors to the dashboard.

## Development

```bash
cargo test
cargo run
```
