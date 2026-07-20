# Validation Report — doctor-config-surface, Slice 2 (interactive surface)

**Date:** 2026-07-20
**Mode:** high-risk
**Cells:** doctor-config-surface-4, doctor-config-surface-5
**Gate-bypass level active:** full

## Reality Gate

| Dimension | Verdict | Evidence |
|---|---|---|
| MODE FIT | PASS | Same feature, same locked high-risk mode-gate record from plan.md (6 flags incl. audit/security hard gate). Slice 2 adds no new flags — it wires already-approved foundations together. |
| REPO FIT | PASS | Every cited function signature in both cells (write.rs, secrets.rs, prompt.rs, config/mod.rs) verified against live source by the plan-checker and independently re-verified by the orchestrator (`Check::info`'s construction for missing config, `config::home()`'s private visibility, `ensure_web_secret` vs the read-only checks.rs helper, module declarations in `src/doctor/mod.rs` vs `src/lib.rs`). |
| ASSUMPTIONS | PASS | The socket-chain restructuring caveat plan.md explicitly flagged before Slice 2 cells could be written was resolved and independently confirmed: early return at `checks.rs:105`, only checks 4/5 consume the `socket` value, both now degrade to skipped. |
| SMALLER PATH | PASS | Slice 2 is already the smallest coherent unit within the locked plan's slicing (write foundations → interactive wiring → docs). Splitting further would separate tightly-coupled phase-2/phase-3 orchestration from the fixes it exists to run. |
| PROOF SURFACE | PASS | `cells schedule` reports 2 waves, 0 cycles: `{cell-4}` then `{cell-5}` (cell 5 genuinely depends on cell 4's phase structure and shares its allowed_roots breadth-confirmation function, not just file ownership). |

## Baseline

Re-verified fresh at Slice 2 validation start: `cargo test --quiet` → 132 passed, 0 failed. `cargo clippy --quiet -- -D warnings` → silent. `bash tests/rename_contract.sh` → green (now part of the standing verify command, mechanized during Slice 1 compounding).

## Plan-Checker (adversarial, bee-review / opus)

Dispatched against CONTEXT.md, plan.md's Slice 2 scope note and socket-chain caveat, both new cells, and the actual current source they cite. Initial verdict: **NOT READY**.

**2 BLOCKERs, both resolved:**
1. Cell 4's action text said `--check`/non-interactive stays phase-1-only, while a truth said phase 3 always re-runs "in both interactive and --check/non-interactive modes" — a direct contradiction that would have broken D5's non-interactive parity. Fixed: the re-run truth now scopes to interactive mode only.
2. Cell 4 gated the config guided-fix on `!check.ok`, but a missing config produces `Check::info` (`ok: true`) — the literal must-have would have silently dropped D3's own headline example ("config missing → create it now?"). Fixed: the fix trigger now matches by check identity across both its failure-shaped states (missing = info, invalid = fail), independently confirmed against `checks.rs:71-87`.

**7 WARNINGs, all resolved:**
- The "skipped" check state had no way to actually render differently from a non-critical fail — fixed by specifying a new `Check` struct field and exactly which two render/predicate sites need it.
- "build_checks() always returns all 10 checks" was factually wrong (checks 7/9/10 are already conditionally absent today) — fixed to "no early abort," preserving today's variable count.
- The allowed_roots fix trigger conflated two different entry points (a missing directory vs. an empty list, which never reaches check 7 at all) — fixed to name both explicitly.
- `classify_root_breadth`'s `home` argument had no cited source, and the real resolver (`config::home()`) is private — a wrong reimplementation would have silently bypassed D9's security gate. Fixed: cells now require reusing the existing native-profile logic, not reinventing it.
- Cell 5's web-token fix cited a read-only diagnostic helper that fixes nothing — fixed to cite the real creation path, `config::ensure_web_secret()`.
- D16's non-loopback warning text lives only in `main.rs`, which wasn't in cell 5's read scope — fixed by adding it and citing the exact source lines.
- A key-link claimed `src/lib.rs` declares the edit module; it's actually `src/doctor/mod.rs` — corrected.

Every finding was independently re-verified by the orchestrator against live source before the cells were revised (not accepted on the reviewer's word alone).

## Feasibility Matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| Early-return removal correctly scoped to only the 2 checks that consume `socket` | MEDIUM | Inspect every check for a `socket` dependency | Confirmed: only checks 4 (`socket.exists()`, `:110`) and 5 (`SocketHerdr::new(socket)`, `:122`) touch it; checks 6-10 don't. | PASS |
| `classify_root_breadth`'s home resolution doesn't bypass D9's security gate | HIGH | Confirm the cited resolver exists and is reusable | `config::home()` exists but is private (`mod.rs:735`) — cells now require exposing/reusing it, explicitly prohibiting reimplementation. | PASS (cell repaired) |
| Web-token guided fix actually fixes something | MEDIUM | Inspect the cited helper's real behavior | `ensure_web_secret_readonly_impl` (checks.rs) is read-only; `config::ensure_web_secret()` (mod.rs:775) is the real path. Cell corrected. | PASS (cell repaired) |
| `cells schedule` reports zero cycles, correct wave shape | required | `bee cells schedule --feature doctor-config-surface --json` | 2 waves: `{cell-4}` then `{cell-5}`; 0 cycles. | PASS |

## Cell Review (cold pickup)

- **doctor-config-surface-4:** COLD-PICKUP READY after resolving both blockers and 4 warnings — phase structure, fix triggers, and the skipped-state mechanics are now unambiguous and source-verified.
- **doctor-config-surface-5:** COLD-PICKUP READY after resolving 3 warnings — correct helper citations, correct module-declaration site, D16's real source now in scope.

## Decision

**READY** (after cell repair; initial plan-checker pass was NOT READY, both blockers and all warnings resolved and independently re-verified).

## Gate 3 — Execution Approval

Advisor: not configured (AO2b, recorded and proceeding). Fresh `advisor_ref` recorded against the current feature, newest decision id, and `plan.md` sha256 (the Slice-1 ref had gone stale from decisions logged during Slice 1 compounding).

Gate-bypass level `full` is active: execution is self-approved rather than asked, per the human's own prior choice to lift the high-risk/hard-gate checkpoint. Recorded below, machine report linked, work proceeds to swarming.
