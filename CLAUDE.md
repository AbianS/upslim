# UpSlim — Claude Code Guide

## Project overview

Monorepo for UpSlim, a minimal Rust uptime monitoring server. Uses pnpm workspaces + Turborepo.

```
packages/
  server/   # Rust binary — the actual uptime monitor
  docs/     # VitePress documentation site
```

## Key commands

```bash
# Install all dependencies
pnpm install

# Build everything
pnpm build

# Run all linters
pnpm lint

# Build only the server (Rust)
cargo build -p upslim-server

# Run all server tests
cargo test -p upslim-server

# Build docs site
pnpm --filter @upslim/docs build

# Preview docs locally
pnpm --filter @upslim/docs dev
```

## Monorepo structure

- **Package manager**: pnpm 10 with workspaces (`pnpm-workspace.yaml`)
- **Task runner**: Turborepo (`turbo.json`)
- **JS linter/formatter**: Biome (`biome.json`)
- **Node version**: ≥18

## Workspace packages

| Package | Path | Language |
|---------|------|----------|
| `upslim-server` | `packages/server` | Rust |
| `@upslim/docs` | `packages/docs` | TypeScript / VitePress |

## Release workflow

Releases use Changesets. The flow:

1. Add a changeset: `pnpm changeset`
2. Push to `main` → `release.yml` creates a "Version Packages" PR
3. Merge the PR → `release.yml` creates GitHub releases with tags:
   - `upslim-server@x.x.x` → triggers Docker build + push
   - `docs@x.x.x` → triggers GitHub Pages deployment

## GitHub Actions

| Workflow | Trigger | Does |
|----------|---------|------|
| `ci.yml` | PR to main | Rust tests + clippy + fmt |
| `docker-publish.yml` | GitHub release | Docker build (amd64+arm64) or deploy docs |
| `release.yml` | Push to main | Changesets → version PRs → GitHub releases |

## Secrets required

| Secret | Used by |
|--------|---------|
| `PAT_TOKEN` | release.yml (push tags, create releases) |
| `DOCKERHUB_USERNAME` | docker-publish.yml |
| `DOCKERHUB_TOKEN` | docker-publish.yml |

## Important conventions

- All code comments and docs in **English**
- Rust edition 2024, rust-version 1.86
- No Spanish in any committed file
- `packages/server/config/test-local.yaml` is gitignored — never commit real credentials
