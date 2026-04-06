---
title: Monitors
description: HTTP and TCP monitor configuration reference for UpSlim.
---

# Monitors

A monitor defines what to check and how often. UpSlim supports two monitor types: `http` and `tcp`.

## Common fields

These fields apply to both HTTP and TCP monitors.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Unique name for this monitor |
| `type` | `http` \| `tcp` | required | Monitor type |
| `interval` | duration | `60s` | How often to run the check |
| `timeout` | duration | `30s` | Maximum time to wait |
| `failure_threshold` | integer | `3` | Consecutive failures before alerting |
| `success_threshold` | integer | `2` | Consecutive successes to mark recovered |
| `send_on_resolved` | boolean | `true` | Send recovery notification |
| `conditions` | list | required | Conditions that must all pass |
| `alerts` | list | `[]` | Alert providers to notify |

## HTTP monitor

Performs an HTTP request and evaluates the response.

```yaml
monitors:
  - name: api-health
    type: http
    url: "https://api.example.com/health"
    interval: 30s
    timeout: 10s
    method: GET                           # optional, default: GET
    headers:
      Authorization: "Bearer ${API_TOKEN}"
    conditions:
      - "[STATUS] == 200"
      - "[RESPONSE_TIME] < 500"
      - "[BODY].status == healthy"
    alerts:
      - name: slack-ops
```

### HTTP-specific fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | string | required | Full URL including scheme |
| `method` | string | `GET` | HTTP method |
| `headers` | map | `{}` | Request headers |
| `body` | string | none | Request body (for POST/PUT) |

### Available conditions for HTTP

| Expression | Description |
|-----------|-------------|
| `[STATUS] == 200` | HTTP status code equals 200 |
| `[STATUS] < 400` | HTTP status code less than 400 |
| `[RESPONSE_TIME] < 500` | Response time in milliseconds |
| `[BODY] == ok` | Raw response body equals string |
| `[BODY].field == value` | JSON body dot-path equals value |

See the [Conditions DSL](/reference/conditions) for the full syntax.

## TCP monitor

Opens a TCP connection and checks it succeeds within the timeout.

```yaml
monitors:
  - name: postgres
    type: tcp
    host: "db.internal"
    port: 5432
    interval: 60s
    timeout: 5s
    conditions:
      - "[CONNECTED] == true"
    alerts:
      - name: slack-ops
```

### TCP-specific fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | string | required | Hostname or IP address |
| `port` | integer | required | Port number (1–65535) |

### Available conditions for TCP

| Expression | Description |
|-----------|-------------|
| `[CONNECTED] == true` | TCP handshake completed successfully |
| `[RESPONSE_TIME] < 100` | Time to establish connection in ms |

## Per-monitor alert overrides

Thresholds can be overridden per alert reference, not just at the monitor level:

```yaml
monitors:
  - name: critical-api
    type: http
    url: "https://payments.example.com/health"
    conditions:
      - "[STATUS] == 200"
    failure_threshold: 3    # monitor-level default
    alerts:
      - name: slack-ops
        failure_threshold: 1  # alert fires after 1 failure for this provider
```

::: warning
Per-alert `failure_threshold` overrides are read by the alert state machine. The monitor still runs every `interval`, but the alert fires only when the overridden threshold is reached.
:::

## Multiple monitors example

```yaml
defaults:
  interval: 60s
  timeout: 10s

monitors:
  - name: web
    type: http
    url: "https://example.com"
    conditions:
      - "[STATUS] == 200"
      - "[RESPONSE_TIME] < 2000"

  - name: api
    type: http
    url: "https://api.example.com/health"
    interval: 30s        # override global default
    conditions:
      - "[STATUS] == 200"
      - "[BODY].status == healthy"

  - name: database
    type: tcp
    host: "db.internal"
    port: 5432
    conditions:
      - "[CONNECTED] == true"

  - name: cache
    type: tcp
    host: "redis.internal"
    port: 6379
    conditions:
      - "[CONNECTED] == true"
```
