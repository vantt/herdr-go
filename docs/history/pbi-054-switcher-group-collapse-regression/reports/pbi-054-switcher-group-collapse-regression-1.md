# pbi-054-switcher-group-collapse-regression-1 — Done Report

**Cell:** pbi-054-switcher-group-collapse-regression-1 — Add missing [hidden] override for .workspace-rows
**Lane:** tiny · **Tier:** extraction · **Worker:** exec-1
**Decisions:** D1, D2, D3 (`docs/history/pbi-054-switcher-group-collapse-regression/CONTEXT.md`)

## Diff (worker's, verbatim)

```diff
diff --git a/web/src/styles.css b/web/src/styles.css
index eb82013..699d655 100644
--- a/web/src/styles.css
+++ b/web/src/styles.css
@@ -648,6 +648,13 @@ button {
   margin: 0;
 }
 
+/* Workspace group card list — hidden when .workspace-header toggle collapses it.
+   The explicit [hidden] rule is required because .agent-list sets display:flex,
+   which would otherwise outrank the UA [hidden] { display: none } rule. */
+.workspace-rows[hidden] {
+  display: none;
+}
+
 /* Visually hidden but still reachable by screen readers (D4) -- clipped, not
    display:none, so assistive tech still announces it. */
 .sr-only {
```

No JS changes — matches D3 exactly (must_haves.key_links: "No changes to switcher.ts", confirmed via `git diff --stat`).

## Orchestrator's independent verify re-run (fresh, own shell)

```
$ flock ...verify.lock -c "cargo test --quiet && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && bash tests/rename_contract.sh && cd web && npm run bundle && npm run test -- --run"
rename contract: ok
✓ built in 650ms (CSS 24.15 kB, JS 319.71 kB)
Test Files  5 passed (5)
     Tests  88 passed (88)
```

Rust suite (cargo test, clippy -D warnings, fmt --check) passed silently under `--quiet` inside the `&&` chain — reaching the later bundle/test steps proves it did not fail. Frozen judge: `cells judge --id pbi-054-switcher-group-collapse-regression-1` → `hits: []` (no undeclared test/CI/lockfile/verify-config changes).

## Capture

Backlog `docs/backlog.md` PBI-054 row corrected during exploring: original "PBI-052 regression" framing replaced with the confirmed root cause (pre-existing `.agent-list`/`[hidden]` cascade-origin bug, unrelated to PBI-052) and this fix, still `in-flight` pending the user's own manual/visual confirmation that groups now actually collapse/expand (this repo has no automated way to assert a CSS-only display fix — jsdom does not load the external stylesheet).

## Next action

Commit (cell id in message), then invoke bee-scribing.
