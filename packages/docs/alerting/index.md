---
title: Alerting Overview
description: How UpSlim alert providers and the alert state machine work.
---

# Alerting

UpSlim sends alerts through **providers**. Each provider is defined once in the `alerting` section and referenced by name from individual monitors.

## Provider configuration

```yaml
alerting:
  - name: slack-ops          # reference name used in monitors
    type: slack
    token: ${SLACK_BOT_TOKEN}
    channel: "#ops-alerts"
    reminder_interval: 2h    # optional: resend while still down
```

## How the state machine works

UpSlim does not alert on every failed check. It tracks a state per `(monitor, provider)` pair and transitions between states based on thresholds.

```
          failure_threshold reached
Healthy ─────────────────────────────► Firing
           (sends DOWN alert)

           success_threshold reached
Firing  ─────────────────────────────► Recovered
           (sends RECOVERED alert, if send_on_resolved: true)

           (resets to Healthy)
```

### Firing

A check must fail consecutively `failure_threshold` times (default: 3) before the first alert is sent. This prevents flapping from causing alert noise.

### Recovery

A check must pass consecutively `success_threshold` times (default: 2) before a recovery alert is sent. This prevents a single transient success from triggering a false recovery.

### Reminders

If `reminder_interval` is set on a provider, UpSlim resends the alert while the monitor stays down:

```yaml
alerting:
  - name: slack-ops
    type: slack
    token: ${SLACK_BOT_TOKEN}
    channel: "#ops-alerts"
    reminder_interval: 2h   # resend every 2 hours while still DOWN
```

Remove `reminder_interval` entirely if you only want one alert per incident.

## Alert payload

Each alert contains:

- **Monitor name**
- **URL** (HTTP monitors)
- **Timestamp**
- **Response time**
- **Status code** (HTTP monitors)
- **Failure reason** — the condition that failed, e.g. `[STATUS] == 200: got '503'`

## Supported providers

| Provider | Status |
|----------|--------|
| [Slack](/alerting/slack) | ✅ Available |
| PagerDuty | Planned |
| Email | Planned |
| Webhook | Planned |

## Using multiple providers

A monitor can send to multiple providers simultaneously:

```yaml
alerting:
  - name: slack-team
    type: slack
    token: ${SLACK_TOKEN}
    channel: "#alerts"

  - name: slack-oncall
    type: slack
    token: ${SLACK_TOKEN}
    channel: "#oncall"

monitors:
  - name: payments
    type: http
    url: "https://payments.example.com/health"
    conditions:
      - "[STATUS] == 200"
    alerts:
      - name: slack-team
      - name: slack-oncall    # both providers notified
```
