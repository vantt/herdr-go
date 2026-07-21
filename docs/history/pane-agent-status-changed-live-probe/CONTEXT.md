# Live Probe: pane.agent_status_changed — Context

**Feature slug:** pane-agent-status-changed-live-probe
**Date:** 2026-07-20
**Exploring session:** complete
**Scope:** Quick
**Domain types:** RUN

## Feature Boundary

Prove, against a real running `herdr` (not the app's `FakeHerdr` test double), whether the `pane.agent_status_changed` subscribe event delivers with acceptable latency and whether the already-known 2×/1ms duplicate is correctly collapsed by the app's existing cursor de-dup logic. This is PBI-003, the residual item DISCOVERY.md already flags from PBI-001 ("đo agent_status_changed với agent thật"). The deliverable is evidence and a recommendation for a future poll→subscribe migration decision — this feature does not build that migration itself (D5).

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Isolated workspace = a dedicated throwaway `herdr` session, named by the probe (never `default`), fully separate socket/directory from the operator's live session. The probe starts this session itself and stops+deletes it on completion, regardless of pass/fail. The exact headless/scripted bring-up mechanism for that session (`herdr --session <name>` launches the interactive/foreground app per `herdr --help` — whether that's directly scriptable or needs another invocation shape) is empirical, deferred to planning (see Outstanding Questions), not settled fact. | `herdr session list` confirms the operator's real live session is named `default` and is currently running with its own socket at `~/.config/herdr`; `herdr`'s own CLI (`--session <name>`, `session stop`/`delete`) is a first-class isolation primitive, matching the backlog's explicit "không đụng agent sống của operator" requirement. Fresh-eyes review flagged that D1 originally overstated the bring-up mechanism as settled when only the isolation *contract* (separate session, torn down after) is actually locked. |
| D2 | The throwaway agent is a real process launched via `herdr agent start <name> -- <argv>` inside the isolated session. Status is observed via `herdr wait agent-status <pane_id> --status <idle\|working\|blocked\|done\|unknown>` (not `herdr agent wait`, whose `--status` flag does NOT accept `done` — confirmed via `herdr agent --help` vs `herdr wait --help`). Per DISCOVERY.md:71, a finished agent may report as `idle` rather than `done` in the live snapshot — the probe must observe and record whichever status actually fires, not assume `done` is guaranteed to appear. The exact command/output pattern used to reliably trigger each transition is discovered empirically during the spike itself. | `herdr wait --help` confirms `wait agent-status` accepts `done` where `agent wait` does not — using the wrong command would silently make "done" unreachable. DISCOVERY.md already documents the idle-vs-done ambiguity as a known real-world quirk, not a hypothetical; the probe's findings must report what actually happens, not paper over it. |
| D3 | Evidence goes to `.bee/spikes/pane-agent-status-changed-live-probe/findings.md`; the delta is appended to `docs/DISCOVERY.md`'s existing PBI-001 section, resolving the "Residual (verified-live TODO)" line already there — not a new DISCOVERY section. | Matches the exact convention PBI-001 and PBI-002 already established in this repo, and DISCOVERY.md already carries a placeholder line naming this exact residual. |
| D4 | The spike's deliverable is measured evidence — real `pane.agent_status_changed` delivery latency, explicit confirmation or refutation of the 2×/1ms duplicate, and confirmation the existing cursor de-dup logic collapses it correctly — plus a qualitative GO/NO-GO recommendation. Not a pre-committed numeric pass/fail threshold. | Matches PBI-001/PBI-002's own recorded verdict style (evidence + qualified recommendation) rather than inventing an ungrounded latency number now. |
| D5 | This spike does NOT implement the poll-to-subscribe migration in `src/watcher.rs` — it only produces the evidence/verdict a later feature would use to decide whether to build that migration. Explicit non-goal. | Matches the backlog's own framing ("điều kiện để chuyển... không chặn ship M0") and PBI-002's precedent of separating protocol-layer proof from the actual build. |
| D6 | "Latency" (D4) means the elapsed time from the timestamp the probe itself issues the triggering action for a state transition (the `herdr` CLI call that causes the transition — e.g. sending input to move idle→working) to the timestamp the subscribe client receives the corresponding `pane.agent_status_changed` event for that pane. Both timestamps are captured by the probe's own process, not by any external clock. | Fresh-eyes review flagged that D4 named "latency" without defining its two endpoints, leaving planning to guess what's actually being timed. This definition is concrete and directly measurable by the probe script itself, with no dependency on herdr's internal (unobservable) detection timing. |
| D7 | Before any `agent start` or `wait agent-status` call, the probe asserts its resolved session name is non-empty and literally not equal to `default`; any assertion failure aborts immediately (fail closed) before touching herdr at all. | Fresh-eyes review's safety finding: D1 states the isolation rule in prose, but nothing enforces it mechanically — an omitted or empty `--session` value would silently target the operator's live `default` session. This decision makes that check load-bearing, not optional. |

### Agent's Discretion

The exact `herdr agent start` invocation and script used to drive idle→working→blocked→done (per D2) is left to planning/execution to determine empirically — try a few candidate scripts, observe what herdr classifies each as, and record what worked in the findings file.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Isolated workspace | A `herdr` session started with a probe-specific `--session <name>`, never the operator's `default` session — separate socket, separate state, torn down after the probe. |
| Throwaway agent | A real (not faked) process launched via `herdr agent start` inside the isolated session, used only to observe herdr's real status-detection behavior — discarded after the probe. |

## Existing Code Context

### Reusable Assets

- `windows-runtime-smoke.ps1` / `macos-install-smoke.sh` — both already demonstrate the "start an isolated named session, drive it, tear it down in a trap/finally" shape against real herdr/gateway processes; the wait-poll pattern (`Wait-Until`/`wait_until`) is directly reusable for polling subscribe events too.
- `src/watcher.rs` — the current poll-based (500ms) implementation this spike's evidence would eventually inform a decision about; not touched by this feature (D5).
- `docs/DISCOVERY.md` (2026-07-17 entry, points 3-4 and the "Residual" line) — already documents subscribe's replay-ring-buffer behavior, the mandatory cursor de-dup requirement, and the two dot-kind filtered events (`pane.agent_status_changed` requires `pane_id`, subscribed per-pane).

### Established Patterns

- `.bee/spikes/tier2-observe-control/findings.md` (PBI-002) and the PBI-001 spike — the exact evidence-file + DISCOVERY.md-delta shape this feature reuses (D3).

## Canonical References

- `docs/DISCOVERY.md` — the 2026-07-17 entry's points 3-4 and its "Residual (verified-live TODO)" line, which this spike resolves.
- `docs/backlog.md` PBI-001, PBI-002 — the spike-shape precedent this feature follows.
- `herdr --help`, `herdr agent --help`, `herdr session --help`, `herdr workspace --help` — the CLI primitives confirmed live on this machine (herdr 0.7.4) that ground D1/D2.

## Outstanding Questions

### Deferred To Planning

- [ ] Exact `herdr agent start` command/script to reliably drive idle→working→blocked→done, and the exact headless bring-up mechanism for the isolated session itself (per D1/D2) — empirical.
- [ ] How to observe the subscribe event stream itself for timing measurement (reuse this app's own subscribe client code, or a raw socket probe against herdr directly) — implementation detail.
- [ ] Whether `done` reliably fires as its own `pane.agent_status_changed` event, or whether (per D2/DISCOVERY.md:71) it is indistinguishable from `idle` in practice — the probe records what actually happens either way.

## Deferred Ideas

- (none new this session — the actual poll→subscribe migration itself, if the verdict is GO, is already implicitly deferred per D5 and is not a new backlog item since PBI-003 already exists to gate it.)

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked decisions, code context, canonical references, and deferred-to-planning questions. Validating and reviewing use locked decisions for coverage and UAT.
