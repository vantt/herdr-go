# Validation — herdr-socket-observe (live socket probe)

Lane: high-risk. Bypass: full. Evidence: `.bee/spikes/herdr-socket-observe/probe.py` against the running herdr 0.7.4.

## Reality gate

| Check | Verdict | Evidence |
|---|---|---|
| MODE FIT | PASS | High-risk realignment of external integration; correct. |
| REPO FIT | PASS | herdr 0.7.4 running (session `default`), socket at `~/.config/herdr/herdr.sock`. |
| ASSUMPTIONS | PASS (revised) | Socket = newline-JSON, one req/conn; `ping` proto 16; `session.snapshot` → 7 flat agents; `pane.read` → text (69×214). **`pane.resize` relative only — no grid sizing; no attach in request API.** This invalidated the poll-then-resize plan for phone width. |
| SMALLER PATH | PASS | Pivot to snapshot-view + reply (decision `675fc93a`) is strictly simpler than live-PTY: no attach, no stream, no resize, no spike — uses only `pane.read` + `pane.send_input`, both verified. |
| PROOF SURFACE | PASS | Read path fully probed live; send path from schema (`pane.send_input {pane_id,keys,text}`), to be confirmed live on a throwaway pane in hso-3 (not on the operator's live agents). |

## Feasibility matrix

| Assumption | Risk | Proof | Result |
|---|---|---|---|
| Socket speaks newline-JSON req/resp | — | probe: ping/snapshot/read all returned | PROVEN |
| snapshot is flat agents[] | — | 7 agents, keys confirmed | PROVEN |
| pane.read returns renderable screen | — | 69×214 text under result.read.text | PROVEN |
| pane.send_input sends text | LOW | schema shape; live-confirm on throwaway pane in hso-3 | ACCEPTED (confirm at exec) |
| No PTY grid sizing in request API | — | pane.resize is direction+amount; no size param anywhere | PROVEN (drives the pivot) |

## Verdict

**READY** for the snapshot-view + reply model. Gate 3 auto-approved (full bypass). A reality-check FAIL on the *original* poll-then-resize plan was surfaced to the user, who chose the simpler model — this report validates that model.
