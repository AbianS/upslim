---
layout: home

hero:
  name: UpSlim
  text: Uptime monitoring that stays out of your way
  tagline: Minimal Rust server for monitoring HTTP endpoints and TCP services. Configures in minutes, runs anywhere, alerts on Slack.
  image:
    src: /logo.svg
    alt: UpSlim
    width: 200
    height: 200
  actions:
    - theme: brand
      text: Get Started
      link: /guide/
    - theme: alt
      text: View on GitHub
      link: https://github.com/AbianS/upslim

features:
  - icon: 🦀
    title: Written in Rust
    details: Static binary under 5 MB. Near-zero idle memory. No runtime dependencies.

  - icon: ⚡
    title: Fast and concurrent
    details: Each monitor runs in its own async task. Semaphore-controlled concurrency. Configurable from 1 to N parallel checks.

  - icon: 📄
    title: YAML configuration
    details: Single file or directory of files. Supports ${ENV_VAR} substitution. No database, no UI, no magic.

  - icon: 🔔
    title: Smart alerting
    details: Configurable failure and recovery thresholds. Optional reminder intervals. Sends only when state actually changes.

  - icon: 🐳
    title: Docker-ready
    details: Multi-stage build produces a scratch image with only the binary and CA certificates. Works on amd64 and arm64.

  - icon: 🔌
    title: Extensible alerting
    details: Slack built-in. Provider trait designed for adding PagerDuty, email, webhooks, and more.
---
