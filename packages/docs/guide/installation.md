---
title: Installation
description: How to install and run UpSlim with Docker or from source.
---

# Installation

## Docker Compose (recommended)

This is the easiest way to get UpSlim running. You only need Docker installed.

### Step 1 — Create your project folder

```bash
mkdir my-upslim && cd my-upslim
```

### Step 2 — Create your config file

Create a `config/` folder and add a YAML file inside it:

```bash
mkdir config
```

```yaml
# config/monitors.yaml
alerting:
  - name: slack-ops
    type: slack
    token: ${SLACK_BOT_TOKEN}
    channel: "#ops-alerts"

monitors:
  - name: my-website
    type: http
    url: "https://example.com"
    conditions:
      - "[STATUS] == 200"
      - "[RESPONSE_TIME] < 2000"
    alerts:
      - name: slack-ops
```

::: tip Where does this file go?
Put it anywhere on your machine — just make sure to mount the folder into Docker in the next step. The example above puts it in `./config/monitors.yaml` relative to where your `docker-compose.yml` will live.
:::

::: info What is `:ro`?
The `:ro` at the end of the volume mount stands for **read-only**. It means the container can read your config files but cannot modify them. This is a security best practice — UpSlim only needs to read your config, never write to it.
:::

### Step 3 — Create `docker-compose.yml`

In your project folder (next to `config/`), create this file:

```yaml
# docker-compose.yml
services:
  upslim:
    image: abians7/upslim-server:latest
    volumes:
      - ./config:/config:ro   # :ro = read-only, the container cannot modify your files
      - upslim-state:/data    # stores alert state across restarts
    environment:
      UPSLIM_STATE_DIR: /data
      SLACK_BOT_TOKEN: ${SLACK_BOT_TOKEN}   # read from your shell or .env file
      RUST_LOG: info
    restart: unless-stopped

volumes:
  upslim-state:
```

Your folder structure should now look like this:

```
my-upslim/
├── docker-compose.yml
└── config/
    └── monitors.yaml
```

### Step 4 — Start UpSlim

```bash
# Set your Slack token (or add it to a .env file)
export SLACK_BOT_TOKEN=xoxb-...

# Start in the background
docker compose up -d

# Follow the logs
docker compose logs -f
```

You should see:

```
INFO  Configuration loaded monitors=1 providers=1
INFO  Starting 1 monitors
INFO  Monitor started monitor=my-website interval_secs=60
```

That's it — UpSlim is running.

---

## Using multiple config files

UpSlim can load a whole folder of YAML files and merge them. This is useful for splitting config by team or service. Files are merged in alphabetical order.

```
config/
├── 01-alerting.yaml    ← alert providers
├── 02-platform.yaml    ← infra monitors
└── 03-apps.yaml        ← application monitors
```

The volume mount in `docker-compose.yml` stays the same — UpSlim reads everything in `./config/` automatically.

---

## docker run (without Compose)

If you prefer a single command:

```bash
docker run -d \
  --name upslim \
  -v /absolute/path/to/config:/config:ro \
  -v upslim-state:/data \
  -e UPSLIM_STATE_DIR=/data \
  -e SLACK_BOT_TOKEN=xoxb-... \
  -e RUST_LOG=info \
  --restart unless-stopped \
  abians7/upslim-server:latest
```

::: warning Use an absolute path
`-v /absolute/path/to/config:/config:ro` — Docker requires an absolute path on the left side when using `docker run`. With Docker Compose you can use `./config` (relative).
:::

---

## Build from source

You need [Rust 1.86+](https://rustup.rs/) and Cargo.

```bash
git clone https://github.com/AbianS/upslim
cd upslim
cargo build --release -p upslim-server
```

The binary is at `target/release/upslim-server`.

```bash
# Point it at a single file
./target/release/upslim-server --config ./config/monitors.yaml

# Or a whole directory
./target/release/upslim-server --config ./config/
```

---

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `UPSLIM_STATE_DIR` | `./state` | Directory for persisting alert state |
| `UPSLIM_MAX_CONCURRENT` | `10` | Maximum parallel checks at any point |
| `RUST_LOG` | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error` |

---

## Verifying it works

**Checks passing:**
```
INFO  Configuration loaded monitors=2 providers=1
INFO  Monitor started monitor=my-website interval_secs=60
INFO  Monitor started monitor=my-db interval_secs=60
```

**A check fails — alert fired:**
```
WARN  Check FAILED monitor=my-website response_time_ms=5012 reason=Some("[STATUS] == 200: got '503'")
INFO  Alert sent monitor=my-website provider=slack-ops action=Fire
```

**Service recovers — recovery alert sent:**
```
INFO  Check passed monitor=my-website response_time_ms=112
INFO  Alert sent monitor=my-website provider=slack-ops action=Resolve
```
