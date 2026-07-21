---
artifact_contract: bee-plan/v1
mode: high-risk
feature: web-create-endpoints
context: docs/history/web-create-endpoints/CONTEXT.md
parent: docs/history/new-shell-new-agent/plan.md
---

# Slice 4 — The Web Create Surface

## Mode Gate Record

**Risk flags counted: 4** → `high-risk` (threshold is 4+).

| Flag | Evidence |
|---|---|
| public contracts | three new HTTP routes, the app's first create surface; plus a widened `Herdr` trait signature |
| external systems | the two herdr create verbs reach the socket for the first time from a request |
| cross-platform | the destination path's trustworthiness differs by platform; P2/P8 decide how that is carried |
| data model | `AppState` grows config-derived state; `Snapshot`'s anchor join grows a provenance-carrying return |

Not counted — auth: no auth logic changes. The new routes take the existing
`AuthSession` extractor positionally (P9); the extractor and its `silent_404`
are untouched. Not counted — weakening proof: every change is additive and the
one signature widening (`cwd: &str` → `Option<&str>`) keeps all existing
call-site behavior by passing `Some(..)`.

**Product files: 8** — `src/herdr/wire.rs`, `src/herdr/mod.rs`,
`src/herdr/socket.rs`, `src/herdr/fake.rs`, `src/web/mod.rs`, `src/web/api.rs`,
`src/web/create.rs` (new), `src/main.rs`. Well past `small`'s cap of three, and
the cross-platform flag alone forbids anything below `standard`.

### Deviation from the frozen parent plan's file list

The parent plan (`docs/history/new-shell-new-agent/plan.md`) scoped slice 4 to
`src/web/mod.rs`, `src/web/api.rs` and one new handler module. Two files below
that layer are added here, both for reasons that only became visible with slice
1 and 2 built. The parent's **exit state** is unchanged and remains what binds.

1. **`src/herdr/wire.rs`** — `anchor_cwd_for_workspace` returns `Option<String>`
   and discards which field the path came from. P2 needs that fact. Recomputing
   it in the web layer would mean re-walking the same four-hop join a second
   time, in a second place, with a second chance to disagree with the first.
2. **`src/herdr/{mod,socket,fake}.rs`** — herdr's `tab.create` takes `cwd` as an
   **optional** parameter and, when it is absent, resolves the workspace's own
   anchor itself (`upstreams/herdr/src/app/api/tabs.rs:65-67` →
   `resolve_new_terminal_cwd`). Our port declares `cwd: &str`, which cannot
   express "absent". That is a defect in the port's fidelity to the protocol,
   and slice 4 is where it first bites: a workspace whose anchor join misses
   (P2 says the row still ships) would otherwise be either uncreatable or
   created at the empty path. Widening to `Option<&str>` makes the miss degrade
   into exactly what the herdr desktop does.

## Discovery

**L1 — quick verify. No `discovery.md`.**

Every candidate was already settled: the mutating-endpoint contract is copied
from `send_reply` (`src/web/screen.rs:56-71`), the auth posture from
`src/web/auth.rs:67-88`, the fake-backed test harness from `test_state()`
(`src/web/mod.rs:88-96`). The one live lookup was upstream herdr's `cwd`
handling, quoted above, which changed the slice's shape and is recorded in the
deviation note rather than a research file.

## Approach

### Chosen path

Bottom-up again, for the same reason slice 1 was: the layer every route rests on
gets proven before a route exists. Provenance first (a pure function with
fixtures), then the port's optional `cwd` (a trait change with three population
sites), then the read endpoint, then the two writes. Each cell is independently
verifiable and nothing user-visible depends on an unproven join.

### Rejected alternatives

- **Compute provenance in the web layer** from the raw `Pane`. Rejected: it
  duplicates the four-hop join and lets two implementations of the same rule
  drift. The precedent is explicit — `groupByWorkspace` is exported and
  unit-tested precisely so the rule has one home.
- **Keep `cwd: &str` and refuse to create into a destination whose path did not
  resolve** (409). Rejected: P2 ships that row deliberately so the operator can
  still see the workspace; a row that renders and then always fails is worse
  than either shipping it working or hiding it. Refusing also diverges from what
  the herdr desktop does in the same situation, for no gain.
- **One `POST /api/create` with a `type` field.** Rejected by P5: the two calls
  take different inputs (a shell takes nothing beyond the destination; an agent
  takes a preset label) and return different things (an agent also returns its
  generated name). One route would need both halves optional and would validate
  by hand what the type system otherwise does.
- **Two fetches when the sheet opens** (destinations, then presets). Rejected by
  P4: the sheet cannot render either list without both, so two round trips buys
  a slower FAB and a partial-failure state to design.

### Wire contract

```
GET  /api/create-options
     -> 200 { "destinations": [ { "workspace_id", "label",
                                  "path": string|null, "path_is_live": bool } ],
               "presets":      [ { "label" } ] }

POST /api/panes            { "workspace_id" }
     -> 200 { "tab_id", "pane_id" }

POST /api/agents           { "workspace_id", "preset" }
     -> 200 { "tab_id", "pane_id", "name" }
```

Errors on both create routes, per P7 and the CONTEXT error table: 409 when the
destination is gone, 400 for an unknown preset label, 502 `{"error": …}` for
everything else, opaque 404 unauthenticated. `path` is sent to herdr as the
create `cwd` when it resolved, and omitted when it did not — herdr then seeds
the folder the same way the desktop would.

`presets` deliberately carries only `label`: the phone never receives `argv`
either, which keeps the operator's command lines off the wire entirely and makes
P6 impossible to violate by accident from the frontend.

### Risk map

| Component | Risk | Proof required |
|---|---|---|
| Preset lookup by label | **HIGH** — this is the boundary that stops a phone from choosing what runs on the host (P6) | A test proving an unknown label is refused, and that the argv the port receives is the one from config, never anything request-derived |
| Optional `cwd` through the port | **MEDIUM** — three population sites, the exact shape of the known repeat failure in this repo | All three files in one cell; a fake-backed assertion that `None` omits the key rather than sending `""` |
| Provenance flag correctness | **MEDIUM** — a `true` on a stale path is precisely the silent wrong-folder failure D5 forbids | Fixture cases for `foreground_cwd` present, `cwd`-only, both absent, and a join miss |
| Stale-destination mapping | **MEDIUM** — 404 here would collide with the opaque auth answer | A test that a `WorkspaceNotFound` from the port becomes 409, and that an unauthenticated request to the same route becomes 404 |
| `AppState` widening | **LOW** — two construction sites, both compile-checked | The suite |

### Open questions for validating

1. Does `FakeHerdr` need destination-shaped state (workspaces with layouts and
   panes) that it does not already carry, for the create-options test to be
   meaningful? Slice 1 seeded some; confirm before the cell assumes it.
2. Should `POST /api/panes` be `/api/panes` or `/api/tabs`? herdr's verb is
   `tab.create` and it creates a tab *containing* a pane; the response carries
   both ids. The existing route family is `/api/panes/:pane/...`, which argues
   for `/api/panes`. Low stakes, but it is a public contract and slice 5 will
   hard-code it.

## Test Matrix

| Dimension | Case |
|---|---|
| Happy path | Create-options lists every workspace; a shell and an agent both create against `FakeHerdr` and return their ids |
| **Agentless workspace** | A workspace whose panes are all plain shells appears in `destinations` — the case today's `/api/agents` structurally drops |
| Provenance true | Anchor pane has `foreground_cwd` → `path_is_live: true` |
| Provenance false | Anchor pane has only `cwd` → path present, `path_is_live: false` |
| Unresolvable | Join miss → `path: null`, row still present, and the create call omits `cwd` |
| Security | `POST /api/agents` with an unknown preset label → 400, nothing reaches the port |
| Security | A request body carrying `argv` cannot influence what is run — the field is not deserialized at all |
| Stale destination | Port returns `WorkspaceNotFound` → 409, not 404 |
| Error passthrough | Port returns `Remote{code,message}` → 502 with the message |
| Auth | Each of the three new routes, unauthenticated → opaque 404 |
| Backwards compat | Existing `/api/agents`, screen, input and keys tests unchanged and green |

## Cells

Created after Gate 2 approval, four sequential cells:

1. **`web-create-endpoints-1`** — anchor provenance in `wire.rs`.
2. **`web-create-endpoints-2`** — optional `cwd` through the port's three sites.
3. **`web-create-endpoints-3`** — `AppState` carries presets; `GET /api/create-options`. Depends on 1.
4. **`web-create-endpoints-4`** — the two create routes in `src/web/create.rs`. Depends on 2 and 3.

## Handoff

Exits to `bee-validating` (high-risk lane). Slice 5 — the FAB, the sheet, the
`path_is_live: false` caveat, and the duplicate-label disambiguator — is shape
only; its cells do not exist and must not be created here.
