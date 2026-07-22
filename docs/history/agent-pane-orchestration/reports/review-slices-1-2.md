# Independent review — agent-pane-orchestration, slices 1-2

**Requested by:** the human · **Date:** 2026-07-22 · **Scope:** `a780d48..HEAD`, everything under `.claude/skills/herdr-orchestrating/` and its `.agents/` twin
**Lenses:** cold-start sufficiency · correctness and safety of an unattended system
**Verdict: P1 findings open — Gate 4 blocks.**

## Summary

Both reviewers independently reached the same conclusion from different directions: **as shipped, the loop does not work, and one of the ways it fails is unsafe rather than merely useless.**

Nine P1s were raised across the two lenses. Four were repaired during the review pass. **Five remain open**, three of which I reproduced directly.

## Repaired during the review

| Finding | Repair |
|---|---|
| `herdr pane layout --current` resolves a *different workspace* than `pane current --current` — reproduced live (`w7` vs `w5`) — so every announcement and every de-duplication read would target a stranger's pane | Both roles now pass `--pane <own pane_id>`; the trap is documented so it is not reintroduced |
| The spawn argv carried no opening instruction, so the working agent would sit idle, never self-name, never be counted as occupying a slot — and dispatch would spawn again every 60s straight through the cap of 4 | §8 now requires a positional prompt whose first act is self-naming to the bare slug |
| `MERGE_CONFLICT` had no report template and was outside the suppression match, so it alone would retry every 60s forever | One template covers both outcomes; the scan matches the shared prefix |
| The bootstrap never started the merge loop, and §0's failure path reported into a pane it had not yet resolved | Both fixed; three broken cross-references corrected |

Also repaired: the worktree path is now derived from the label rather than from `cwd`/`foreground_cwd` (which disagree on live panes, and picking `cwd` would test MAIN against the finished-conditions forever); the runtime tab's own empty root pane is no longer reported as an anomaly; a stale duplicate spawn command was removed from the quick reference.

## Open P1s — these block

### 1. The merge loop can never merge anything

`.bee/logs/{dispatch,hooks,tools}.jsonl` are **tracked** in the main checkout despite `.gitignore` listing `.bee/logs/` — they were committed before the ignore existed. Every bee tool call rewrites them, and `bee worktree merge` refuses on a dirty main.

```
$ git -C <main> ls-files .bee/logs/
.bee/logs/.gitkeep  .bee/logs/dispatch.jsonl  .bee/logs/hooks.jsonl  .bee/logs/tools.jsonl
$ git -C <main> status --porcelain
 M .bee/logs/dispatch.jsonl
 M .bee/logs/hooks.jsonl
 M .bee/logs/tools.jsonl
```

The merge role must run in main and its own first command dirties it. So it refuses its own merge, every cycle, forever — no pane is ever closed, no slot is ever freed, and after four dispatches the loop is dead with the only signal a refusal typed into a chat pane.

*Fix:* `git rm --cached .bee/logs/*.jsonl`, and treat `WORKTREE_MERGE_MAIN_DIRTY` as an anomaly report rather than a silent skip.

### 2. The lane classifier is fail-open on plainly-worded dangerous work

It fails closed on rows it cannot *parse*, but returns safe for any row whose wording misses its keyword list. Run against the shipped script:

```
row: "Remove the login token check on the admin endpoint and delete the tests that cover it."
  → {"lane":"small","hard_gate_flags":[],"lane_safe":true}

PBI-042 (download and install a release binary from GitHub, then restart the service)
  → {"lane":"small","hard_gate_flags":[],"lane_safe":true}
```

This is the single most dangerous finding: D6 exists precisely to keep an unattended agent away from that first row, and the guard waves it through. The repo's backlog is written in Vietnamese, so most real rows dodge English patterns by default — the passing case in this feature's own proof was luck of vocabulary, not coverage.

*Fix:* make the lane gate two-key — the script's verdict **and** the dispatch agent's own reading of the full row, with the agent's refusal fail-closed and announced. A regex list cannot enumerate danger; it can only enumerate the words we thought of.

### 3. The dispatchable set is structurally empty, forever

D1's reverse index reads `**Backlog:** PBI-NNN` from each `docs/history/*/CONTEXT.md`. Of 24 such files, **one** carries that line — this feature's own, which I hand-wrote. Nothing emits it: `bee-exploring` writes the mapping the other way, putting the slug into the backlog row.

So after merge the dispatch loop wakes every 60 seconds, spends a full Sonnet session, finds nothing, and ends quietly — 1440 sessions a day, indefinitely, with no signal that anything is wrong.

*Fix:* read the slug from the backlog row's own feature field (what exploring actually writes) and keep the CONTEXT.md existence check as condition (a) — or make exploring emit the header line.

### 4. D12's verify lock was never built

D12 is a locked decision: four working agents may edit in parallel but verify runs serially behind a cross-process lock. There is no lock anywhere in the shipped code. Under D5 the configured verify — `cargo test` + `clippy` + `npm bundle` + `npm test` — can run in four worktrees plus a fifth inside `bee worktree merge`, on ~5 GB free.

This is a locked decision with no owning cell, the fourth instance of that failure shape in this feature.

*Fix:* wrap `commands.verify` in a `flock` on a path shared by main and every worktree. It belongs in `.bee/config.json`, since neither control role runs verify itself.

### 5. The red-verify stop is still not durable

The suppression record is a line in a chat pane's scrollback. Four ways it evaporates: `send-text` types into an interactive agent's composer rather than scrollback and may never be readable back; 400 lines of live conversation is minutes; the human may close and recreate the pane (the document itself anticipates this); and no shipped proof records a `send-text` → `pane read` round-trip at all.

Each evaporation returns the system to retry-every-60-seconds, which with the measured 1-in-12 flake lands a genuine semantic conflict in main within about twelve minutes.

*Fix:* a durable marker under the gitignored `.bee/tmp/`. D18 forbids a *state registry of occupancy*; a red-stop marker is a different object, and D3's "stops for good" cannot be satisfied without one.

## Notable P2s

- The scrollback check is written *inside* the red-verify bullet, i.e. after the merge command a cold reader would already have run. It must be its own step, before the merge.
- Both stop-file paths are relative to the invoker's cwd, so the bootstrap's stale-stop-file guard checks a different file than the loop reads.
- `claude -p` runs with no timeout: a hung invocation blocks the loop forever and the stop file is never re-read.
- No single-instance guard — running the bootstrap twice yields two dispatch loops polling the same backlog.
- `docs/backlog.md` contains duplicate rows for PBI-031 and PBI-032 with different statuses; the classifier takes the first match and stops.

## What is solid

Verified, and deliberately not to be "fixed":

- The two skill roots are byte-identical across all eight files.
- The classifier's fail-*closed* branches all hold: missing argument, unreadable path, no matching row, empty row — each returns unsafe with a distinct reason.
- D19's error tolerance is structural: stubbed failing iterations are reported and the loop continues; `--max-iterations` and the stop file both exit cleanly.
- `bee worktree merge`'s red-verify safety is a proven property, not an assertion: it stages with `--no-commit`, aborts, and re-proves HEAD and porcelain. Main really is byte-untouched. (The CLI's own help text claims the opposite and is the stale one.)

## Assessment

Five of the nine P1s were repairable inside the review; the remaining five are not cosmetic. Two of them — the permanently dirty main and the empty reverse index — mean the loop as merged would do nothing at all, loudly in one case and silently in the other. One is a safety hole that a keyword list cannot close by adding more keywords.

The common cause is the one already recorded in `critical-patterns.md`: every check in this feature was scoped to the artifact of the cell that produced it. Nothing ever ran the assembled system against reality. Both reviewers found their P1s in minutes by doing exactly that.

---

## Re-review after the fix slice (same day)

**Three HOLD, two PARTIAL, no new P1s.**

| Original P1 | Verdict |
|---|---|
| Fail-open lane gate | **HOLDS.** Key 2 is mandatory and fail-closed, and the document pre-empts the exact rationalisation that would skip it: "Treat `lane_safe:true` from the script as 'no obvious keyword hit,' never as 'safe.'" The reviewer wrote three fresh dangerous rows; two were caught by Key 2's categories. |
| Empty reverse index | **HOLDS.** 15 backlog rows carry the new source; 14 resolve to a CONTEXT.md that exists, and the one that does not is correctly skipped. Today's set is empty for the right reason — one `in-flight` row, already holding a worktree. |
| Red-stop record | **HOLDS.** Marker is gitignored (so it can never dirty main), its check is its own step ahead of the merge, and create/read/clear are plain filesystem operations available to both parties. |
| Main permanently dirty | **PARTIAL** — correct and durable in this branch, proven live: the log files grew under the reviewer's own commands while `git status` stayed empty. But the fix lives only here. **Main still tracks them today, so the merge that delivers the fix would itself be refused.** A human must untrack them in main by hand first. |
| Verify lock | **PARTIAL** — same shape. Contention proven (a 2s holder made a second `flock` wait 1.69s) and exit codes reach the caller correctly, but main and the two sibling worktrees still carry the unlocked verify until this lands and they pick it up. |

### What the fix slice broke, and the repair

The two fixes interacted, which is exactly the risk of repairing several things at once. Serializing verify behind a lock made a merge's wall clock its own verify **plus up to three queued ones** — routinely past a 900-second iteration timeout. And killing a merge is not a harmless retry: bee's abort-and-prove path is a JS `finally`, which `SIGTERM` never runs, so the kill would leave main holding a staged uncommitted merge. Permanently dirty — the stall this slice had just finished fixing.

Repaired three ways: the merge loop runs on a 90-minute bound, `timeout` escalates with `-k 30s` so a process ignoring `SIGTERM` still dies, and the merge role aborts a leftover `MERGE_HEAD` before doing anything else.

Also repaired: the single-instance guard could lock the human out permanently, because a `dispatch` label outlives the process that set it and the refusal message told them to do the one thing that would not help; D1 was amended in the record rather than quietly rewritten, with the original text and the measurement that falsified it preserved; the area spec no longer claims the classifier fails closed; and the worked example that still taught the discarded lookup is marked superseded rather than deleted, since what it proves is still true.

### Residual, accepted

- A row like *"tidy up `.gitignore` and re-commit the ignored log files"* passes both keys — it names no listed category and reads as janitorial, yet its effect is the dirty-main stall. Mitigated by adding the loop's own machinery to Key 2's refusal list; the general class (harmless-sounding work with systemic effect) cannot be closed by enumeration.
- The lock path is a machine-specific absolute path in a tracked config that D11 sends upstream.
- `bee worktree merge` holds a store lock across verify and refuses after ~5s with `LOCK_BUSY`; the dispatch role has no handling for that refusal.

**Verdict: P1 = 0.**
