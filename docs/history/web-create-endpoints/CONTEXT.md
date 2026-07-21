# Context: the web create surface

Slice 4 of `new-shell-new-agent`, run as its own lane. This is the gateway's
first write surface beyond replying into a pane that already exists: the phone
asks for a destination list, then asks to create a shell or an agent in one of
them. The sheet that consumes all of this is slice 5; nothing here renders.

The frozen parent plan (`docs/history/new-shell-new-agent/plan.md`) handed this
slice four questions. All four are decided below.

## Question 1 — where is the destination list assembled?

**Decision: the backend, as one endpoint, from `snapshot.workspaces` — not the
frontend from the agent rows it already has.**

1. **The frontend's data cannot answer it.** Today's list is built from
   `snap.agents` (`src/web/api.rs:29-56`), and herdr's `agents[]` excludes any
   pane with no agent (`Terminal::is_agent_terminal`). A workspace whose panes
   are all plain shells therefore has no rows and cannot appear. The captured
   live snapshot contains exactly that case — and it was the *focused*
   workspace. A destination list built from agent rows would inherit the bug the
   feature exists to route around.
2. **The path join is already in Rust.** `Snapshot::anchor_cwd_for_workspace`
   (`src/herdr/wire.rs:242`) was built and fixture-proven in slice 1. Assembling
   in the frontend would mean shipping the same join twice, in the weaker
   language, over a payload that does not carry `panes[]` today.
3. **One place owns path truth.** P2 below makes the destination path carry its
   own provenance. That belongs where the join is.

## Question 2 — what does a destination show on Windows?

`foreground_cwd` is unix-only (`upstreams/herdr/src/pane.rs:2717-2740`) and on
Windows PowerShell the plain `cwd` is only correct while herdr's OSC 9;9 shell
integration is live, because PowerShell never updates its Win32 process cwd on
`Set-Location`. The parent plan listed three candidate answers: degrade to the
workspace label, show the path with a caveat, or block the action.

**Decision: show the path, and ship its provenance beside it — no platform
branch in the web layer.**

The framing "this is a Windows problem" is wrong, and taking it literally would
have produced a `cfg(windows)` fork in a layer that has none. The real predicate
is *which field the path came from*: `foreground_cwd` is the live directory,
`cwd` is the process's start directory. Windows always falls to `cwd`; unix
falls to it too whenever `foreground_cwd` is absent. Same uncertainty, one
cause.

So a destination carries `path` plus a boolean saying whether the path is the
live directory or the fallback. Blocking was rejected: it would make the whole
feature unusable on Windows for a path that is usually right. Label-only was
rejected: it discards a path that is usually right and leaves the operator with
strictly less information. Silently showing the fallback as certain is the one
thing D5 exists to forbid, and this avoids it without lying in the other
direction.

## Question 3 — the create error surface

**Decision: extend `send_reply`'s match (`src/web/screen.rs:56-71`), with one
new class — a destination that no longer exists is 409, never 404.**

| Outcome | HTTP |
|---|---|
| created | 200 + the new ids |
| destination gone (`WorkspaceNotFound`, herdr `agent_placement_not_found` / `agent_placement_conflict`) | 409 |
| unknown preset label in the request | 400 |
| anything else (`Remote`, `Request`, `Unavailable`, `Malformed`, `InvalidAgentArgv`) | 502 `{"error": …}` |
| unauthenticated | opaque 404, same as every other route |

404 is load-bearing in this codebase: it is what an unauthenticated request gets
so that an attacker cannot tell a real route from a missing one
(`src/web/auth.rs:86-88`). A stale destination returning 404 would make that
signal ambiguous for the one caller that must distinguish them — the sheet, which
needs to say "that workspace closed, pick another" rather than "log in again".

`AgentNameTaken` is deliberately absent: the collision retry lives inside the
port (`src/herdr/mod.rs:67-112`) and slice 2 settled that it must not be
surfaced or reimplemented here.

## Question 4 — what the phone sends to start an agent

**Decision: a preset label. The phone never sends `argv`.**

This is not a convenience choice; it is the reason this feature's mode gate did
not count an audit/security flag. `argv` is operator-authored, lives in the
gateway's config, and is edited through `doctor` (parent D4). A create request
that carried `argv` would turn an authenticated phone session into arbitrary
local command execution. Labels are unique — slice 3 rejects duplicates at
config load — so a label is a sufficient key.

## Locked decisions

| # | Decision |
|---|---|
| P1 | The destination list is assembled in the backend from `snapshot.workspaces`, one row per workspace, including workspaces with no agents. |
| P2 | A destination row is `{ workspace_id, label, path, path_is_live }`. `path` is `anchor_cwd_for_workspace`'s answer; `path_is_live` is true only when it came from `foreground_cwd`. `path` may be null when the join misses; the row still ships. |
| P3 | Where the workspace label and the resolved path disagree, the path wins (parent D11). The label is shown as context, never as the destination. |
| P4 | The sheet's two lists arrive in one round trip: destinations and agent presets from a single endpoint. The FAB opens on one fetch. |
| P5 | Creating a shell and starting an agent are two routes, not one route with a type field. They take different inputs and return different things. |
| P6 | An agent create request names a preset by `label`. `argv` is never accepted from the client, in any form. |
| P7 | A destination that vanished between opening the sheet and tapping is 409. 404 stays reserved for the opaque unauthenticated answer. |
| P8 | The web layer contains no platform branch. Path uncertainty is carried as data (P2), not as `cfg(windows)`. |
| P9 | Both create routes sit behind `AuthSession` as the first extractor, positionally, exactly like every other authenticated route. |
| P10 | **Added during validation, not at Gate 1.** When the destination's path does not resolve, the shell route omits `cwd` and lets herdr resolve the workspace anchor; the agent route **refuses with 409** instead. The two herdr verbs do not fall back alike — `tab.create` resolves the workspace's own anchor (`upstreams/herdr/src/app/api/tabs.rs:65-67`), while `agent.start` falls back to the **herdr process's own directory** (`upstreams/herdr/src/app/agents.rs:118-122`). Starting an agent there is the silent wrong-repo start parent D5 exists to forbid; opening a shell in herdr's own anchor is exactly what the desktop does. |

## Out of scope

- Rendering any of this. The FAB, the sheet, the caveat that `path_is_live:
  false` earns, and the navigate-into-the-new-pane step are all slice 5.
- Creating a workspace for a project that is not open yet (parent D8, PBI-020).
- The duplicate-label disambiguator (two workspaces sharing a label and a
  folder). It becomes visible once destinations list agentless workspaces, but
  the fix is a display concern — slice 5, PBI-024.

## Code context

- `src/web/mod.rs:51-61` — the route table this slice extends; `AppState:31-37`,
  which does **not** carry config today and must, to resolve preset labels.
- `src/web/screen.rs:56-71` — the mutating-endpoint precedent, copied not
  reinvented.
- `src/web/auth.rs:67-88` — `AuthSession` and `silent_404`.
- `src/herdr/mod.rs:139-154` — `tab_create` / `agent_start`; `:17-40` —
  `HerdrError`.
- `src/herdr/wire.rs:242-255` — the anchor join and the
  `foreground_cwd`-over-`cwd` precedence P2 reports on.
- `src/config/mod.rs:44,49-53` — `Config.agent_presets` and `AgentPreset`.
- `src/web/mod.rs:88-96` — `test_state()`, the `FakeHerdr`-backed harness every
  web test uses.

## Outstanding Questions

### Resolve Before Planning

- [ ] None.

### Deferred To Planning

- [ ] How `Config` reaches `AppState`. The state struct is constructed in
      `main.rs` and in `test_state()`; presets must reach the agent-create
      handler without dragging the whole config into the web layer if a
      narrower shape is honest.
