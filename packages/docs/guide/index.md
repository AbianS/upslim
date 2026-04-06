---
title: What is UpSlim?
description: An overview of UpSlim — a minimal Rust uptime monitoring server.
---

# What is UpSlim?

**UpSlim** is a lightweight uptime monitoring server written in Rust. It periodically checks HTTP endpoints and TCP services, evaluates configurable conditions, and sends alerts when something goes down or recovers.

## Key concepts

### Monitors

A **monitor** is a periodic check against a target. Each monitor has:

- A **type** — `http` or `tcp`
- A **target** — URL, host + port
- One or more **conditions** that must all pass for the check to succeed
- An optional list of **alert providers** to notify

### Conditions

Conditions are simple expressions evaluated after each check:

```
[STATUS] == 200
[RESPONSE_TIME] < 500
[BODY].status == healthy
[CONNECTED] == true
```

See the [Conditions DSL reference](/reference/conditions) for the full syntax.

### Alert state machine

UpSlim does not send an alert on every failed check. Instead it maintains a state per monitor + provider pair:

| State | Meaning |
|-------|---------|
| `Healthy` | All checks passing |
| `Firing` | Threshold reached, alert sent |
| `Recovered` | Back to healthy, recovery alert sent |

**`failure_threshold`** — consecutive failures required before firing (default: 3)  
**`success_threshold`** — consecutive successes required to mark as recovered (default: 2)  
**`reminder_interval`** — resend the alert if still down after this duration (optional)

### Configuration files

UpSlim loads YAML from a single file or a directory. When loading a directory it merges all `*.yaml` / `*.yml` files in lexicographic order. This lets you split your config into logical files:

```
config/
├── 01-defaults.yaml
├── 02-alerting.yaml
└── 03-monitors.yaml
```

### State persistence

Alert state survives restarts. UpSlim persists state as JSON files in the directory set by `UPSLIM_STATE_DIR` (default: `./state`). This prevents duplicate alerts after a restart.

## What UpSlim is not

- **Not a metrics platform** — there are no time-series metrics or dashboards
- **Not a distributed system** — it runs as a single process
- **Not a SaaS** — you host it yourself

If you need a hosted uptime service or rich dashboards, look at Grafana Cloud, Better Uptime, or UptimeRobot.
