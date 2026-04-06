# Contributing to UpSlim

Thanks for taking the time to contribute.

## Development setup

You need:
- [Rust 1.86+](https://rustup.rs/)
- [Node.js 18+](https://nodejs.org/)
- [pnpm 10+](https://pnpm.io/installation)
- [Docker](https://docs.docker.com/get-docker/) (optional, for local container testing)

```bash
git clone https://github.com/AbianS/upslim
cd upslim
pnpm install
```

## Running the server locally

```bash
# Build and run with an example config
cargo run -p upslim-server -- --config packages/server/config/example.yaml

# For local Slack testing, copy the test config and fill in your credentials
cp packages/server/config/example.yaml packages/server/config/test-local.yaml
# edit test-local.yaml with your token and channel
cargo run -p upslim-server -- --config packages/server/config/test-local.yaml
```

`test-local.yaml` is gitignored — never commit real credentials.

## Running tests

```bash
# All tests
cargo test -p upslim-server

# Specific test file
cargo test -p upslim-server config_test

# With logs visible
cargo test -p upslim-server -- --nocapture
```

## Running the docs site

```bash
pnpm --filter @upslim/docs dev   # http://localhost:5173
pnpm --filter @upslim/docs build # static build
```

## Code style

- All comments and documentation in **English**
- Rust: `cargo fmt` before committing, `cargo clippy -- -D warnings` must pass
- No `unwrap()` in production code paths — use `?` or explicit error handling
- JS/TS: formatted by [Biome](https://biomejs.dev/) — run `pnpm lint:fix`

## Release process

Releases use [Changesets](https://github.com/changesets/changesets).

1. Make your changes
2. Run `pnpm changeset` and describe what changed
3. Open a pull request
4. Once merged, the release workflow creates a version PR automatically
5. Merging the version PR triggers the GitHub release and Docker publish

## Project structure

```
packages/
  server/   # Rust binary — uptime monitoring engine
  docs/     # VitePress documentation site
```

See `packages/server/CLAUDE.md` and `packages/docs/CLAUDE.md` for detailed internals.
