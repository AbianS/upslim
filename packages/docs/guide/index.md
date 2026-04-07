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

## Resource footprint

UpSlim is designed to stay completely invisible on the host. These numbers were measured against
the production Docker image running on Apple M4 Pro (arm64), with all monitors hitting a local
HTTP server at 5-second intervals:

| Scenario | Monitors | CPU avg | CPU peak | RAM avg | RAM peak |
|----------|----------|---------|----------|---------|----------|
| idle     | 1        | 0.02%   | 0.11%    | 1.2 MB  | 1.4 MB   |
| light    | 10       | 0.19%   | 0.64%    | 1.6 MB  | 2.1 MB   |
| medium   | 50       | 0.34%   | 0.63%    | 2.3 MB  | 3.1 MB   |
| heavy    | 100      | 0.32%   | 0.65%    | 2.4 MB  | 3.5 MB   |

**Docker image: 1.9 MB** — scratch-based, binary + CA certificates only.

To put that in perspective:

- A typical Node.js app Docker image weighs **150–500 MB**. UpSlim is **~100× smaller**.
- A Go uptime tool with its runtime typically uses **15–30 MB** of RAM under similar load. UpSlim uses **3.5 MB at peak** with 100 monitors.
- The jump from 50 → 100 monitors adds **0.1 MB of RAM and zero measurable CPU**. Each Tokio async task sleeps between checks — it only wakes up for the milliseconds an HTTP request is in-flight.

The CPU number tells the full story: 100 monitors checking every 5 seconds — 1,200 HTTP requests per minute — and the process sits at **0.32% average CPU**. The rest of the time it is completely idle.

This makes UpSlim safe to run on the smallest VPS, alongside your existing services, or inside a container with strict resource limits. See [Docker & Deploy](/reference/docker) for recommended production limits backed by this data.

## What UpSlim is not

- **Not a metrics platform** — there are no time-series metrics or dashboards
- **Not a distributed system** — it runs as a single process
- **Not a SaaS** — you host it yourself

If you need a hosted uptime service or rich dashboards, look at Grafana Cloud, Better Uptime, or UptimeRobot.
