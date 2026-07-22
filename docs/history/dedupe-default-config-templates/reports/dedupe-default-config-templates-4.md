# dedupe-default-config-templates-4

**Status:** [DONE]

**Outcome:** `install.sh` now writes `config.json` by capturing `herdr-go --internal-print-default-config` stdout instead of a hand-written `printf` JSON literal that was missing `agent_presets`. Idempotent only-if-missing guard preserved; nonzero exit or empty output now fails the install via `die`.

**Files touched:** `install.sh`

**Commit:** `19151ba` — `feat(dedupe-default-config-templates-4): install.sh writes config.json from binary's canonical output`

Full trace and verification evidence: `.bee/cells/dedupe-default-config-templates-4.json`
