# Learnings — agent-pane-orchestration (2026-07-22)

Feature: an unattended development loop — two control agents in a cockpit, four working agents in a runtime, dispatch fills free slots and merge retires finished work. 8 cells across 2 slices, one review panel with two structural passes, one real spawn on live hardware.

Four things cost real effort to learn. Three are the *same* failure wearing different clothes.

---

## 1. Cell-scoped verifies all pass while the assembled system cannot run

**Category:** failure · **Tags:** [verification-scope, slice-completeness, false-green]

Three separate pieces were named in one cell's prose as "out of scope" and then owned by no later cell:

| Missing piece | Consequence had it shipped | Caught by |
|---|---|---|
| D6's lane classifier | the safety refusal had no mechanism at all | review panel |
| the real spawn proof | the only unattended-write path had zero live evidence | review panel |
| `references/dispatch-prompt.md` | the loop would print "prompt file not found" every 60s and, correctly, continue forever | the orchestrator, after slice 1 was capped and committed |

Every cell verify was green every time. They were green *because* each was scoped to its own file: byte-identity between roots, required strings present, D-ids cited. Nothing anywhere asked whether the pieces, assembled, could execute a single cycle.

**The rule:** a slice that produces a runnable thing needs at least one cell whose verify **runs the assembled thing**, not its own artifact. The fix here was cheap once seen — put a stub `claude` on `PATH` and run one real loop iteration:

```
PATH="$PWD/.bee/tmp/stubbin:$PATH" bash .../control-loop.sh --role dispatch --max-iterations 1
```

That one line proves the prompt is found, resolved and passed. A `test -s <file>` would not have.

**Smell to watch for:** any cell whose `action` says a piece is "not created here — out of scope for this cell". That sentence is a promise to a cell that may never exist. Either create the piece or create the cell that owns it, in the same breath.

---

## 2. A per-iteration skip is not a stop — polling turns "never retry" into retry-every-60-seconds

**Category:** failure · **Tags:** [loop-semantics, safety-gate, flaky-signal]

The merge role's first draft said the red worktree is not re-attempted *"this iteration"*, and in the next sentence that *"a later iteration will find it still finished and eligible"*. Both were true, which was the problem: in a 60-second loop they are the same sentence. With this repo's measured 1-in-12 verify flake, a red result would have gone green and merged within about twelve minutes — merging exactly what the red result existed to stop.

**The rule:** inside a polling loop, "skip for now" must be anchored to a **durable signal**, never to the iteration boundary. The iteration boundary is not a unit of time; it is a unit of forgetting.

The fix reused a mechanism the design had already been forced into elsewhere: read the chat pane's own scrollback for the earlier report and stay away while it is there. Both the anomaly de-duplication and this came out the same way because no state file was permitted — the constraint produced a better answer than a registry would have.

---

## 3. A document that tells an agent which commands to run needs one of them actually run

**Category:** failure · **Tags:** [verification-evidence, external-cli, doc-as-code]

Two defects shipped inside a skill that verified green:

- It passed `--json` to `herdr pane split`. herdr answers `unknown option: --json` — it emits JSON by default. That was the spawn step.
- It told the dispatcher to `herdr pane split` and then `herdr agent start`. Live, `agent start` opens its **own** pane and never attaches to the split one — so every dispatch would have leaked an empty stray pane, and four dispatches would have filled the concurrency cap with ghosts.

Both were found by the *next* cell's worker, one by running `herdr --help` before assuming a flag, the other by the one cell that ran the sequence for real. This extends the existing `default-agent-presets` pattern from *decisions* to *documents*: instructions are code that an agent executes, and the same evidence standard applies.

**The rule:** when a document hardcodes an external CLI's invocation, at least one of those invocations must be executed against the real binary before the document is called done. Grepping the document for its own content proves nothing about the world.

---

## 4. What a constraint bought: cold sessions and externalised state are the same decision

**Category:** success · **Tags:** [architecture, context-budget, statelessness]

The loop starts a **fresh** session every cycle instead of keeping one alive. The reason is not tidiness: a session that runs for days accumulates its whole history, so it degrades exactly when it has been running longest — and the failure reads as the agent getting confused rather than as a resource limit. Cold cycles cost the same at cycle 5000 as at cycle 1.

That only works because an earlier, independent decision had already put every fact outside the agent: identity in pane labels, progress in the workflow record, occupancy in the live pane list. The two decisions turned out to be one decision seen from two sides. Had a state file been chosen instead, the cold loop would have needed a schema, locking, and a staleness story; with the live system as the source, each cycle simply looks and acts.

**Worth reusing:** when a design forbids stored state, check whether that unlocks statelessness elsewhere. The forbidding is often the gift.

---

## Also recorded

- **`allowed_roots` is not a runtime boundary in this repo** (filed as PBI-044). `Boundary` in `src/security/paths.rs` implements the full ordered path check and is constructed nowhere outside its own tests. Not exploitable today because no endpoint accepts a client cwd — but it becomes a real hole the moment PBI-020 lands, and whoever does that will believe an allowlist is protecting them.
- **A backlog row's prose contains literal `|` characters**, so parsing its table columns by position silently misreads rows. Locate the status column by its closed vocabulary instead.
- **`bee cells cap` refuses without a recorded passing verify**, and refuses boilerplate red evidence under 80 characters. Neither can be talked past — which is why every claim in this feature has an output behind it.
