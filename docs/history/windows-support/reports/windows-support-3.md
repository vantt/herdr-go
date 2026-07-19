# windows-support-3

[BLOCKED]

Added an additive pinned Windows Server 2022 CI proof and a fail-fast real-Herdr runtime/security smoke. The Linux jobs are unchanged. This Linux workspace has no `pwsh` or Windows target, and repository mutation cannot trigger GitHub Actions, so the mandatory real Windows verification has not run and the cell is intentionally uncapped.

Files touched:

- `.github/workflows/ci.yml`
- `scripts/windows-runtime-smoke.ps1`
- `docs/history/windows-support/reports/windows-support-3.md`

Full state and failed verification record: [`.bee/cells/windows-support-3.json`](../../../../.bee/cells/windows-support-3.json)
