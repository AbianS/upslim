<p align="center">
  <img src="packages/docs/public/logo.svg" width="64" height="64" alt="UpSlim logo" />
</p>

<h1 align="center">UpSlim</h1>

<p align="center">
  Minimal uptime monitoring server written in Rust.
  <br />
  Checks HTTP endpoints and TCP services. Alerts on Slack. Runs anywhere.
</p>

<p align="center">
  <a href="https://github.com/AbianS/upslim/actions/workflows/ci.yml">
    <img src="https://github.com/AbianS/upslim/actions/workflows/ci.yml/badge.svg" alt="CI" />
  </a>
  <a href="https://hub.docker.com/r/abians7/upslim-server">
    <img src="https://img.shields.io/docker/v/abians7/upslim-server?label=docker&color=0284c7" alt="Docker" />
  </a>
  <img src="https://img.shields.io/badge/rust-1.86%2B-orange" alt="Rust 1.86+" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="MIT" />
</p>

---

## What it does

UpSlim runs as a single process, reads a YAML config, and periodically checks your services. When something goes down it alerts you on Slack. When it recovers, it tells you that too.

- **HTTP monitors** — checks status codes, response times, and JSON body fields
- **TCP monitors** — verifies that a port is reachable within a timeout
- **Smart alerting** — configurable failure/success thresholds to avoid flapping noise
- **Reminder intervals** — re-alerts every N hours while a service stays down
- **State persistence** — survives restarts without duplicate alerts

## Quick start

```yaml
# config/monitors.yaml
alerting:
  - name: slack-ops
    type: slack
    token: ${SLACK_BOT_TOKEN}
    channel: "#ops-alerts"

monitors:
  - name: api
    type: http
    url: "https://api.example.com/health"
    conditions:
      - "[STATUS] == 200"
      - "[RESPONSE_TIME] < 500"
    alerts:
      - name: slack-ops

  - name: database
    type: tcp
    host: "db.internal"
    port: 5432
    conditions:
      - "[CONNECTED] == true"
    alerts:
      - name: slack-ops
```

```bash
docker run -d \
  -v ./config:/config:ro \
  -v upslim-state:/data \
  -e UPSLIM_STATE_DIR=/data \
  -e SLACK_BOT_TOKEN=xoxb-... \
  abians7/upslim-server:latest
```

## Docker Compose

```yaml
services:
  upslim:
    image: abians7/upslim-server:latest
    volumes:
      - ./config:/config:ro
      - upslim-state:/data
    environment:
      UPSLIM_STATE_DIR: /data
      SLACK_BOT_TOKEN: ${SLACK_BOT_TOKEN}
      RUST_LOG: info
    restart: unless-stopped

volumes:
  upslim-state:
```

## Configuration

| Field | Default | Description |
|-------|---------|-------------|
| `interval` | `60s` | Check frequency |
| `timeout` | `30s` | Max wait per check |
| `failure_threshold` | `3` | Failures before alerting |
| `success_threshold` | `2` | Successes before marking recovered |
| `send_on_resolved` | `true` | Send recovery notification |

Supports `${ENV_VAR}` substitution in any string value. Load from a single file or a directory — all `*.yaml` files are merged in lexicographic order.

## Conditions

```
[STATUS] == 200
[RESPONSE_TIME] < 500
[BODY].status == healthy
[CONNECTED] == true
```

## Build from source

```bash
git clone https://github.com/AbianS/upslim
cd upslim
cargo build --release -p upslim-server
./target/release/upslim-server --config ./config/
```

Requires Rust 1.86+.

## Documentation

Full docs at **[abians7.github.io/upslim](https://abians7.github.io/upslim)**

## License

MIT
