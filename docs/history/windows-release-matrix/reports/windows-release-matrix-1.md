# windows-release-matrix-1

Status: DONE

Outcome: Added a new, separate top-level `release-windows` job to `.github/workflows/release.yml`, pinned to `windows-2022`. It builds the Windows binary, proves itself against real Herdr using ci.yml's checksum+smoke pattern, and packages a Windows-only archive (binary + docs only, no `install.sh`/`herdr-go.service`). The existing `build` job is byte-for-byte unchanged (confirmed via `git diff`: 94 unchanged lines followed by 82 appended lines).

Files touched: `.github/workflows/release.yml`

Full trace/evidence: `.bee/cells/windows-release-matrix-1.json`

Commit: dc288aa
