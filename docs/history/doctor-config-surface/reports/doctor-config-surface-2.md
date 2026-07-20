# doctor-config-surface-2

[DONE] Added `src/config/write.rs`: validated persist (D6), field-by-field
repair with per-field diagnosis (D7), timestamped backup-then-recreate
fallback for unparseable files (D7), and an `allowed_roots` breadth
classifier for filesystem root / home / symlink (D9). All pure functions,
no prompting or terminal use; nothing calls this module yet.

Files touched: `src/config/write.rs` (new), `src/config/mod.rs` (additive
`pub mod write;` declaration).

Full trace/evidence: `.bee/cells/doctor-config-surface-2.json`.
