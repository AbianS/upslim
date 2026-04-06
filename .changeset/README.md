# Changesets

This directory is used by [Changesets](https://github.com/changesets/changesets) to manage versioning and changelogs.

## How to add a changeset

```bash
pnpm changeset
```

Follow the prompts to describe your change. A new `.md` file will be created here.

## Release flow

1. Add a changeset: `pnpm changeset`
2. Push to `main` → CI creates a "Version Packages" PR
3. Merge the PR → GitHub releases are created automatically
