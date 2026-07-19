[BLOCKED]

Mode-isolated legacy migration, mutation-safe development preflight, and truthful pre-release installation guidance are implemented.

Files touched: `src/main.rs`, `dev-deploy.sh`, `README.md`, `docs/installation.md`, `docs/usage.md`, `docs/specs/installation.md`, `docs/specs/system-overview.md`, `.github/workflows/release.yml`, `tests/rename_contract.sh`.

Full trace and verification evidence: [rename-herdr-go-3 cell](../../../../.bee/cells/rename-herdr-go-3.json).

Blocker: implementation and cell cap are complete, but the sandbox mounts `.git` read-only, so Git could not create `index.lock` for the required cell commit.
