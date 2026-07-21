---
artifact_contract: bee-plan/v1
mode: spike
approved_gate2: 2026-07-20
---

# Plan: Live Probe pane.agent_status_changed

Mode: `spike` — 1 risk flag (external systems: a real `herdr` process). Spike overrides flag-count lane sizing per the mode gate: this is a single yes/no proof, not a behavior change.
Why this is the least workflow that protects the work: the whole deliverable is evidence + a recommendation (D4); there is no source behavior to build, so any heavier lane would be ceremony over a measurement task.

## Requirements (from CONTEXT.md)

- D1: isolated throwaway `herdr` session, never `default`, torn down on completion regardless of outcome.
- D2: drive the agent via `herdr agent start`; observe status via `herdr wait agent-status <pane_id> --status <idle|working|blocked|done|unknown>` (not `herdr agent wait`, which lacks `done`); record whatever status actually fires for a finished agent (DISCOVERY.md:71's idle-vs-done ambiguity).
- D3: evidence in `.bee/spikes/pane-agent-status-changed-live-probe/findings.md`; delta appended to `docs/DISCOVERY.md`'s existing PBI-001 section.
- D4: deliverable is measured evidence (latency numbers, duplicate/de-dup confirmation) + a qualitative GO/NO-GO recommendation — no pre-set numeric threshold.
- D5: no change to `src/watcher.rs` or any migration — evidence only.
- D6: latency = elapsed time from the probe's own trigger-action timestamp to the subscribe client's event-received timestamp, both captured by the probe process itself.
- D7: fail-closed assertion — session name non-empty and `!= "default"` — before any `agent start`/`wait agent-status` call.

## Discovery

L1 quick verify, run live during planning (evidence below, not a separate discovery.md per the L0/L1 fan-out rule):

- **Headless bring-up confirmed:** `herdr --session <name> server &` genuinely creates an isolated session with its own directory and socket (`~/.config/herdr/sessions/<name>/`), fully separate from `default` (`~/.config/herdr/`). Verified live: started `pbi003-discovery-check`, confirmed via `herdr session list` it ran alongside `default` untouched, then `herdr session stop`/`delete` cleanly removed it — `herdr session list` afterward showed only `default` again. This resolves CONTEXT.md's first Deferred-To-Planning item outright.
- **Remaining empirical unknown (by design, per D2):** the exact `herdr agent start` command/script that reliably drives idle→working→blocked→done, and whether `done` fires as its own event or presents as `idle` (DISCOVERY.md:71). This is the spike's actual question — not resolved during planning, since resolving it *is* the execution.

## Shape

**The one yes/no question:** does `events.subscribe`'s `pane.agent_status_changed` deliver status changes for a real agent with acceptable latency and a correctly-collapsed duplicate, such that a future poll→subscribe migration would be evidence-backed rather than speculative?

**What proves YES (worth recommending the migration be considered):** the probe captures at least one full idle→working→blocked cycle (done/idle per DISCOVERY.md:71's caveat) with subscribe delivering every transition, latency (per D6) in the same order of magnitude as or better than the current 500ms poll interval, and the existing cursor de-dup logic (already implemented for other event types) demonstrably collapsing the known 2×/1ms duplicate to one observed transition.

**What NO implies (recommend staying on poll for now, or investigate further before considering a switch):** subscribe drops or meaningfully delays a transition, the duplicate isn't cleanly collapsible with the existing de-dup approach, or a state (most likely `done`) cannot be reliably observed at all via subscribe.

**Evidence location:** `.bee/spikes/pane-agent-status-changed-live-probe/findings.md` (per D3), delta into `docs/DISCOVERY.md`'s existing PBI-001 section.

## Test matrix

Spike lane — the 2-3 dimensions that bite:
- **Real external dependency behavior** (not mocked): the probe must run against real `herdr` 0.7.4, never `FakeHerdr` — that's the entire point (this is what PBI-001 deferred specifically to avoid disturbing the operator, per D1's isolation).
- **Timing/concurrency**: latency measurement and the 2×/1ms duplicate are inherently timing-sensitive; the probe's own timestamp capture (D6) must be precise enough to resolve millisecond-scale gaps.
- **Cleanup/idempotency**: the isolated session must be torn down even if the probe crashes mid-run (D1/D7) — a leaked session/process is real operator-visible pollution, not just a test artifact.

## Out of scope

- The actual poll-to-subscribe migration in `src/watcher.rs` (D5) — a separate future feature if this spike's evidence supports it.
- Any state other than idle/working/blocked/done — no other `pane.*` or resource event types are in scope.
- Multi-agent/multi-pane concurrent probing — single throwaway agent, single pane, per D1/D2.
