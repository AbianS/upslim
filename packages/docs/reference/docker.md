---
title: Docker & Deploy
description: How to deploy UpSlim with Docker, including image details and production tips.
---

# Docker & Deploy

## Image

UpSlim ships as a multi-stage Docker build that produces a `scratch`-based image containing only:

- The statically compiled `upslim-server` binary
- CA certificates (for HTTPS checks)

The result is a minimal image with no shell, no OS libraries, and no attack surface.

**Supported architectures:** `linux/amd64`, `linux/arm64`

## Docker Compose

```yaml
services:
  upslim:
    image: abians7/upslim-server:latest
    volumes:
      - ./config:/config:ro       # config directory, read-only
      - upslim-state:/data        # persistent alert state
    environment:
      UPSLIM_STATE_DIR: /data
      UPSLIM_MAX_CONCURRENT: "10"
      RUST_LOG: info
    restart: unless-stopped

volumes:
  upslim-state:
```

### Start

```bash
docker compose up -d
```

### Logs

```bash
docker compose logs -f
```

### Reload config

UpSlim reads config at startup only. To reload after a config change:

```bash
docker compose restart
```

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `UPSLIM_STATE_DIR` | `./state` | Directory for persisting alert state across restarts |
| `UPSLIM_MAX_CONCURRENT` | `10` | Max parallel checks running at any moment |
| `RUST_LOG` | `info` | Log level: `trace` `debug` `info` `warn` `error` |

## Volumes

| Mount path | Purpose |
|------------|---------|
| `/config` | YAML config files (single file or directory) |
| `/data` | Alert state JSON files (set `UPSLIM_STATE_DIR=/data`) |

Mount `/config` as read-only (`:ro`) — UpSlim never writes to it.

## Secrets management

Never hardcode tokens in your config files. Use environment variable substitution:

```yaml
# config/alerting.yaml
alerting:
  - name: slack-ops
    type: slack
    token: ${SLACK_BOT_TOKEN}
    channel: "#ops-alerts"
```

Then pass the secret via Docker:

```yaml
# docker-compose.yml
services:
  upslim:
    environment:
      SLACK_BOT_TOKEN: ${SLACK_BOT_TOKEN}   # from host env or .env file
```

Or use Docker secrets for production deployments.

## Building the image locally

The `Dockerfile` is at `packages/server/Dockerfile`. Build context is the workspace root:

```bash
# From the workspace root (upslim/)
docker build \
  -f packages/server/Dockerfile \
  -t upslim-server:local \
  .
```

Or with Docker Compose from the `packages/server/` directory:

```bash
cd packages/server
docker compose build
docker compose run --rm upslim /upslim-server --config /config/my-config.yaml
```

## Resource usage

Benchmarked against the production Docker image on Apple M4 Pro (arm64), running HTTP monitors
at 5-second intervals against a local mock server:

| Scenario | Monitors | CPU avg | CPU peak | RAM avg | RAM peak |
|----------|----------|---------|----------|---------|----------|
| idle     | 1        | 0.02%   | 0.11%    | 1.2 MB  | 1.4 MB   |
| light    | 10       | 0.19%   | 0.64%    | 1.6 MB  | 2.1 MB   |
| medium   | 50       | 0.34%   | 0.63%    | 2.3 MB  | 3.1 MB   |
| heavy    | 100      | 0.32%   | 0.65%    | 2.4 MB  | 3.5 MB   |

**Docker image size: 1.9 MB** (scratch-based, binary + CA certificates only).

Scaling from 50 to 100 monitors adds ~0.1 MB of RAM and no measurable CPU increase. The Tokio
async runtime keeps each monitor task dormant between checks — CPU spikes only during the brief
window when HTTP requests are in-flight.

::: tip
Run `pnpm bench` from the repo root to reproduce these numbers on your own hardware.
:::

## Production tips

**Pin the image version** — use `upslim-server:v1.2.3` instead of `:latest` to avoid unexpected updates.

**Resource limits** — the benchmark data above confirms that even 100 monitors stay well under
4 MB of RAM and 1% CPU. Safe conservative limits for most deployments:

```yaml
services:
  upslim:
    deploy:
      resources:
        limits:
          memory: 32M
          cpus: "0.1"
```

**Health check** — since UpSlim has no HTTP server, use a process check:

```yaml
services:
  upslim:
    healthcheck:
      test: ["CMD-SHELL", "pgrep upslim-server || exit 1"]
      interval: 30s
      timeout: 5s
      retries: 3
```

::: warning
The scratch-based image has no shell. The `pgrep` healthcheck above will not work with scratch images. Use `restart: unless-stopped` instead for automatic recovery.
:::
