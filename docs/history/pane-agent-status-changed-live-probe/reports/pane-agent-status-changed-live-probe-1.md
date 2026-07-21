# pane-agent-status-changed-live-probe-1

**Status:** [DONE]
**Outcome:** Live probe against real herdr 0.7.4 (isolated session `pane-probe-bob`, never `default`) proved `events.subscribe` delivers `pane.agent_status_changed` at 11-18ms latency, no 2x/1ms duplicate reproduced, but `done` never appears as its own value on this event (always `idle`) and rapid sub-~100ms updates coalesce. Qualified GO recommendation recorded.
**Files touched:** `docs/DISCOVERY.md` (committed, cell id in message), `.bee/spikes/pane-agent-status-changed-live-probe/findings.md` + probe scripts/logs (gitignored, evidence-only per repo convention).
**Full trace/evidence:** `.bee/cells/pane-agent-status-changed-live-probe-1.json`
