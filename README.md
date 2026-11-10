# MarusansiBasisDashboard

A lightweight Rust/Axum dashboard for monitoring and managing a Basis Server.

## Features

- Live server health monitor with player count, capacity, version, traffic counters, and timestamps.
- Admin panel for connected-player lookup, direct announcements, world loading/unloading, synchronized world switching, and broadcast announcements.
- Japanese and English admin interface. The first visit follows the browser language; choose a language from the selector to save the preference in that browser.
- Player search, responsive tables, optional five-second auto-refresh, URL/delay validation, and visible request-error feedback.
- Docker image and Docker Compose deployment support.

## Endpoints

- `/` — health dashboard
- `/api/health` — HTML health card
- `/api/health/json` — JSON health data
- `/admin` — administrative panel

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `PORT` | `4000` | Dashboard HTTP port |
| `HEALTH_URL` | `http://localhost:10666/health` | Full Basis health endpoint URL; overrides `HEALTH_PORT` |
| `HEALTH_PORT` | `10666` | Basis health port used when `HEALTH_URL` is not set |
| `API_URL_BASE` | `http://localhost:10667` | Basis admin API base URL |
| `API_BEARER_TOKEN` | required for admin actions | Server-side credential for dashboard-to-Basis API requests; there is no insecure default |
| `RUST_LOG` | `info` | Rust tracing filter |

The dashboard no longer requires the Basis container's network namespace. Configure
`HEALTH_URL` and `API_URL_BASE` with addresses reachable from the dashboard
container. In a shared Compose project, the Basis service name can be used as the
hostname. In separate Coolify resources, connect both resources to a private
Docker network or use private service addresses.

## Docker Compose

See [compose.yaml](./compose.yaml) for a same-stack deployment. The dashboard
and Basis server use the normal Compose network; the dashboard has its own
port mapping and does not share the Basis container's network namespace.

```bash
export BASIS_API_TOKEN='the-token-configured-on-the-Basis-server'
docker compose up -d --build
docker compose logs -f marusansi-basis-dashboard
```

## Security and central authentication

The dashboard's administrative routes can issue commands to the Basis Server.
They are **not yet protected by central AccountServ/OIDC visitor authentication**.
Do not expose `/admin` or its management endpoints to the public internet until
the OIDC login/session integration is complete, or protect them with a trusted
authentication gateway and HTTPS in the meantime.

`API_BEARER_TOKEN` authenticates dashboard-to-Basis API requests; it does not
authenticate dashboard visitors. The permissive cross-origin CORS policy has
been removed because the dashboard UI uses same-origin requests. The upcoming
AccountServ integration will use Authorization Code + PKCE and a server-side
session; browser JavaScript will not receive long-lived tokens.
 
## Development

```bash
cargo test
cargo run
```
