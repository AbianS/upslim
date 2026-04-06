# packages/server — Claude Code Guide

## What this is

The Rust binary that runs UpSlim. A single `upslim-server` process that:
1. Reads YAML config at startup
2. Spawns one Tokio task per monitor
3. Evaluates conditions after each check
4. Fires/resolves Slack alerts based on a state machine

## Commands

```bash
# Run all tests (unit + integration)
cargo test -p upslim-server

# Run a specific test
cargo test -p upslim-server config_test

# Build release binary
cargo build --release -p upslim-server

# Check + clippy
cargo clippy -p upslim-server -- -D warnings

# Run locally with a config file
cargo run -p upslim-server -- --config packages/server/config/example.yaml

# Build and run with Docker (local test)
cd packages/server
docker compose build
docker run --rm \
  -v "$(pwd)/config/test-local.yaml:/config/test-local.yaml:ro" \
  -v upslim-state:/data \
  -e UPSLIM_STATE_DIR=/data \
  -e RUST_LOG=info \
  upslim-server:local /upslim-server --config /config/test-local.yaml
```

## Module map

```
src/
  main.rs          # CLI args, startup, graceful shutdown (SIGTERM/Ctrl-C)
  lib.rs           # Re-exports all modules (needed for integration tests)
  config.rs        # YAML loader: file or directory merge, env var substitution
  types.rs         # Core types: Monitor, CheckResult, AlertState, AlertMessage
  condition.rs     # Condition DSL evaluator: [STATUS], [BODY].x, [CONNECTED]
  error.rs         # UpslimError enum + Result alias
  scheduler.rs     # Per-monitor Tokio tasks, semaphore, 222ms stagger
  state.rs         # JSON file persistence for AlertState (survives restarts)
  checker/
    mod.rs         # Checker trait
    http.rs        # reqwest-based HTTP checker
    tcp.rs         # tokio::net::TcpStream TCP checker
  alert/
    mod.rs         # AlertProvider trait + advance_state() pure function
    slack.rs       # Slack Bot API: chat.postMessage, Block Kit payloads
```

## Test structure

```
tests/
  common/mod.rs       # Shared test helpers
  config_test.rs      # Config loading, defaults, validation, directory merge
  http_check_test.rs  # HTTP checker against wiremock mock server
  slack_test.rs       # Slack provider: payload, validate, send (wiremock)
```

Integration tests live in `tests/` and import from the crate via `upslim_server::`.  
Unit tests live inside each `src/*.rs` file in `#[cfg(test)]` blocks.

## Key design decisions

**State machine** (`alert/mod.rs` — `advance_state`): pure function, no I/O. Takes `AlertState` + `CheckResult` + thresholds, returns `Option<AlertAction>`. Easy to unit test.

**Body reads are lazy** (`checker/http.rs`): the response body is only read if at least one condition references `[BODY]`. Avoids allocating large bodies unnecessarily.

**Staggered start** (`scheduler.rs`): monitors start 222ms apart to avoid thundering herd on boot.

**Graceful shutdown**: `CancellationToken` from `tokio-util`. SIGTERM or Ctrl-C cancels all monitor tasks cleanly.

**State persistence** (`state.rs`): `HashMap<String, AlertState>` in a `Mutex`, flushed to `{state_dir}/state.json` after every change. Key format: `"monitor_name:provider_name"`.

## Dependencies (all exact-pinned)

Only what's needed — no extras. See `Cargo.toml`. Key ones:
- `tokio` — async runtime, minimal features
- `reqwest` — HTTP client, rustls-tls only (no OpenSSL)
- `serde_yaml` — config parsing
- `shellexpand` — `${ENV_VAR}` substitution in YAML
- `async-trait` — dyn `AlertProvider`
- `wiremock` — mock HTTP server for integration tests

## Adding a new alert provider

1. Add a variant to `AlertProviderConfig` in `types.rs`
2. Add config fields to `RawAlertProvider` in `config.rs` and parse them in `process()`
3. Create `src/alert/myprovider.rs` implementing `AlertProvider`
4. Register in `scheduler.rs` where providers are instantiated
5. Add integration tests in `tests/`

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `UPSLIM_STATE_DIR` | `./state` | State persistence directory |
| `UPSLIM_MAX_CONCURRENT` | `10` | Max parallel checks |
| `RUST_LOG` | `info` | Tracing log level |
