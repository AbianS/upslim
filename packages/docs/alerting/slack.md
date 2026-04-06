---
title: Slack
description: How to configure Slack alerting in UpSlim using a Bot token.
---

# Slack

UpSlim sends Slack alerts using the [chat.postMessage](https://api.slack.com/methods/chat.postMessage) API with a Bot token. Messages use Block Kit with a colored border to distinguish DOWN, STILL DOWN, and RECOVERED states.

## Setup

### 1. Create a Slack App

Go to [api.slack.com/apps](https://api.slack.com/apps) and create a new app **From scratch**.

### 2. Add Bot Token Scopes

In **OAuth & Permissions → Scopes → Bot Token Scopes**, add:

| Scope | Required for |
|-------|-------------|
| `chat:write` | Sending messages |

### 3. Install to workspace

Click **Install to Workspace**. Copy the **Bot User OAuth Token** — it starts with `xoxb-`.

### 4. Invite the bot to the channel

In Slack, open the target channel and type:

```
/invite @your-bot-name
```

The bot must be a member of the channel before it can post.

### 5. Get the channel ID

Right-click the channel in Slack → **Copy link** or open channel details. The channel ID is a string like `C0AR8F8BUDA`. You can use either the ID or the channel name (`#ops-alerts`).

## Configuration

```yaml
alerting:
  - name: slack-ops
    type: slack
    token: ${SLACK_BOT_TOKEN}    # xoxb- token
    channel: "C0AR8F8BUDA"       # channel ID or "#channel-name"
    reminder_interval: 2h        # optional
```

Store your token as an environment variable and reference it with `${SLACK_BOT_TOKEN}`.

## Provider fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Reference name for monitors |
| `type` | `slack` | yes | Must be `slack` |
| `token` | string | yes | Bot token starting with `xoxb-` |
| `channel` | string | yes | Channel ID (`C…`) or name (`#…`) |
| `reminder_interval` | duration | no | Resend interval while still down |

## Message format

UpSlim sends Block Kit messages with a color-coded sidebar:

| State | Color | Header |
|-------|-------|--------|
| DOWN | 🔴 Red `#DD0000` | `service-name  is DOWN` |
| STILL DOWN | 🟠 Orange `#FF8C00` | `service-name  is STILL DOWN` |
| RECOVERED | 🟢 Green `#36A64F` | `service-name  is RECOVERED` |

Each message includes monitor name, timestamp, response time, URL (HTTP), and the condition that failed.

## Validation errors

UpSlim validates the Slack config at startup and exits immediately if:

- The token is empty
- The token does not start with `xoxb-` (user tokens starting with `xoxp-` are not supported)
- The channel is empty

```
Fatal error: Config error: Slack provider 'slack-ops': token must start with 'xoxb-' (Bot token)
```

## Troubleshooting

**`channel_not_found`** — The bot has not been invited to the channel. Run `/invite @bot-name` in the channel.

**`not_authed`** — The token is invalid or expired. Regenerate it in the Slack app settings.

**`invalid_auth`** — The token format is wrong. Ensure it starts with `xoxb-`.
