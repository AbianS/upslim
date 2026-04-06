---
title: Configuration
description: Complete reference for the UpSlim YAML configuration file.
---

# Configuration

UpSlim is configured entirely through YAML files. There is no database, no web UI, and no CLI wizard.

## File structure

A minimal config file looks like this:

```yaml
monitors:
  - name: my-api
    type: http
    url: "https://api.example.com/health"
    conditions:
      - "[STATUS] == 200"
```

A full config with all sections:

```yaml
defaults:
  interval: 60s
  timeout: 30s
  failure_threshold: 3
  success_threshold: 2
  send_on_resolved: true

alerting:
  - name: slack-ops
    type: slack
    token: ${SLACK_BOT_TOKEN}
    channel: "#ops-alerts"
    reminder_interval: 2h

monitors:
  - name: my-api
    type: http
    url: "https://api.example.com/health"
    conditions:
      - "[STATUS] == 200"
      - "[RESPONSE_TIME] < 500"
    alerts:
      - name: slack-ops
```

## Environment variable substitution

Any string value can reference environment variables using `${VAR_NAME}`:

```yaml
alerting:
  - name: slack-ops
    type: slack
    token: ${SLACK_BOT_TOKEN}   # resolved at startup
    channel: "#ops-alerts"
```

If the referenced variable is not set, UpSlim will exit with a config error at startup.

## `defaults`

Global defaults applied to every monitor unless overridden.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `interval` | duration | `60s` | How often to run the check |
| `timeout` | duration | `30s` | Maximum time to wait for a response |
| `failure_threshold` | integer | `3` | Consecutive failures before alerting |
| `success_threshold` | integer | `2` | Consecutive successes to mark as recovered |
| `send_on_resolved` | boolean | `true` | Send a recovery notification |

## `alerting`

A list of alert providers. Each provider has a `name` used to reference it from monitors.

See [Alerting → Overview](/alerting/) for the full provider reference.

## `monitors`

A list of monitors. Each monitor can override any field from `defaults`.

See [Monitors](/guide/monitors) for the full monitor reference.

## Duration format

Durations are strings with a unit suffix:

| Suffix | Unit |
|--------|------|
| `ms` | Milliseconds |
| `s` | Seconds |
| `m` | Minutes |
| `h` | Hours |

Examples: `500ms`, `30s`, `5m`, `2h`

## Loading from a directory

Pass a directory path instead of a file. UpSlim loads all `*.yaml` and `*.yml` files in lexicographic order and merges them:

- `defaults` — taken from the first file that defines them
- `alerting` — concatenated from all files
- `monitors` — concatenated from all files

This is useful for splitting configuration by team or service:

```
config/
├── 01-defaults.yaml       # global defaults + alerting
├── 02-platform.yaml       # infrastructure monitors
└── 03-apps.yaml           # application monitors
```

::: tip
Use numeric prefixes to control merge order. `01-` comes before `02-`.
:::
