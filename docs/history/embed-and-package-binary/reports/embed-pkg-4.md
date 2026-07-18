# embed-pkg-4 Report

**Status:** [DONE]

**Outcome:** Removed the `cp -r static` line from the release workflow Package step, as the UI is now embedded at compile-time in the binary (embed-pkg-2).

**Files touched:**
- `.github/workflows/release.yml` — removed static asset copy command from Package step

**Full trace and evidence:** [.bee/cells/embed-pkg-4.json](.bee/cells/embed-pkg-4.json)

**Verification:** `! grep -q 'cp -r static' .github/workflows/release.yml` — passed

**Commit:** dcba859 — feat(embed-pkg-4): remove static assets from release package step
