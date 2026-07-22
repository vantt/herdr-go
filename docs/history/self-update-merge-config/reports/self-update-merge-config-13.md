# self-update-merge-config-13

**Status:** DONE

**Outcome:** Added `merge_config_on_upgrade` and `MergeUpgradeError` to `src/config/merge.rs`, composing `default_config_json` (cell-11), `merge_missing_fields` (cell-12), and the existing `write::backup_and_recreate` (D7) unchanged. Includes the 3 required named unit tests, including the fail-closed `deny_unknown_fields` case.

**Files touched:** `src/config/merge.rs`

Full trace/evidence: `.bee/cells/self-update-merge-config-13.json`
