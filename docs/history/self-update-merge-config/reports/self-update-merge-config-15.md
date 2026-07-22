# self-update-merge-config-15

**Status:** DONE

**Outcome:** Added `src/update/swap.rs` with `backup_and_swap_binary(target_path, new_bytes)` (backup-then-write-then-rename, unix 0o755 permission set) and a thin untested `backup_and_swap_running_binary` wrapper over `std::env::current_exe()`. Declared via `mod swap;` in `src/update/mod.rs`.

**Files touched:** `src/update/swap.rs` (new), `src/update/mod.rs`

**Verify:** passed — see `.bee/cells/self-update-merge-config-15.json` for the full trace and verification evidence.

**Reservations:** released.

**Commit:** `5e3d6fb` feat(self-update-merge-config-15): add binary backup and atomic swap
