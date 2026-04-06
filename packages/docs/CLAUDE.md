# packages/docs — Claude Code Guide

## What this is

VitePress documentation site for UpSlim. Publishes to GitHub Pages at `abians7.github.io/upslim`.

## Commands

```bash
# Install dependencies (from repo root)
pnpm install

# Start local dev server at http://localhost:5173
pnpm --filter @upslim/docs dev

# Build static site to .vitepress/dist/
pnpm --filter @upslim/docs build

# Preview the built site locally
pnpm --filter @upslim/docs preview
```

## Structure

```
packages/docs/
  .vitepress/
    config.ts          # Site config: nav, sidebar, search, base URL
  public/
    logo.svg           # SVG logo (dark background, pulse line, green dot)
  index.md             # Home page (layout: home, hero + features)
  guide/
    index.md           # What is UpSlim?
    installation.md    # Docker, docker-compose, build from source
    configuration.md   # YAML config reference, env vars, directory loading
    monitors.md        # HTTP and TCP monitor fields and examples
  alerting/
    index.md           # Alert state machine, thresholds, reminders
    slack.md           # Slack Bot token setup, scopes, troubleshooting
  reference/
    conditions.md      # Condition DSL: [STATUS], [BODY], [CONNECTED]
    docker.md          # Production Docker tips, secrets, resource limits
```

## Base URL

The site deploys to a GitHub Pages subdirectory. `config.ts` sets:

```ts
const base = process.env.GITHUB_ACTIONS ? '/upslim/' : '/'
```

All internal links work locally (`/`) and in production (`/upslim/`) without change.

## Deployment

Triggered automatically by a `docs@x.x.x` GitHub release tag (see `docker-publish.yml`).  
The `deploy-docs` job in `docker-publish.yml` builds the site and deploys via `actions/deploy-pages`.

To deploy manually: GitHub → Actions → "Docker Publish" → Run workflow.

## Adding a new page

1. Create a `.md` file in the appropriate directory
2. Add it to the `sidebar` in `.vitepress/config.ts`
3. Use frontmatter for page title and description:

```md
---
title: My Page
description: Short description for SEO.
---
```

## Writing conventions

- All content in **English**
- Project name is **UpSlim** in prose
- Binary/image names stay lowercase: `upslim-server`
- Package names stay as-is: `@upslim/docs`, `UPSLIM_STATE_DIR`
- GitHub: `AbianS/upslim` — DockerHub: `abians7/upslim-server`
- Use VitePress admonitions for tips and warnings:
  ```md
  ::: tip
  Short helpful note.
  :::

  ::: warning
  Important caveat.
  :::
  ```
