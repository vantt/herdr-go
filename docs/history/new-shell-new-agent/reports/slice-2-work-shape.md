# Slice 2 — create verbs on the herdr port

**Feature:** `new-shell-new-agent` · **Lane:** high-risk · **Scope frozen by** `plan.md:58`
**Product files:** `src/herdr/mod.rs`, `src/herdr/socket.rs`, `src/herdr/fake.rs`

Exit state, verbatim from the frozen plan:

> Both verbs callable and fake-backed; `agent_name_taken`, `workspace_not_found`, and
> `invalid_agent_argv` are distinguishable from a generic request failure

---

## The one open question, decided

`plan.md:88` left slice 2 to choose: one `HerdrError` variant per herdr error code, or a
single `Request { code, message }` carrying the code.

**Decision: neither in pure form — typed variants for exactly the three codes the product
branches on, plus a code-carrying catch-all.**

The frozen exit state already names the three: `agent_name_taken`, `workspace_not_found`,
`invalid_agent_argv`. Each has a caller that must *act* on it, not just display it:

| code | who branches on it | why |
|---|---|---|
| `agent_name_taken` | slice 2's own caller, per D5/D7 | the name is auto-generated; a collision must be retried with a new one, silently |
| `workspace_not_found` | slice 4 | a destination the phone chose has since disappeared — needs its own HTTP status and its own message |
| `invalid_agent_argv` | slice 3 | a malformed preset must fail config validation, not surface as a generic 502 |

The remaining upstream codes — `invalid_agent_name`, `agent_placement_not_found`,
`agent_placement_conflict`, `agent_start_failed`, `tab_create_failed`, `invalid_env` —
have exactly one caller behaviour: show the message. A typed variant each would buy
nothing and would make the enum grow every time herdr adds a code.

Resulting shape:

```rust
pub enum HerdrError {
    Unavailable(String),
    ProtocolMismatch { expected: u32, actual: u32 },
    Request(String),                                // unchanged: local/transport failure
    Malformed(String),
    NoSuchPane(String),
    AgentNameTaken { name: String },                // new
    WorkspaceNotFound { workspace_id: String },     // new
    InvalidAgentArgv(String),                       // new
    Remote { code: String, message: String },       // new: herdr said no, code preserved
}
```

`Request(String)` deliberately keeps its current meaning — *we* failed to make the request
(serialize, connect, write, read). `Remote` means the server answered and refused, with its
code intact. Today `parse_response` (`socket.rs:205-221`) collapses both into `Request` and
**throws `error.code` away entirely**; that is the single line of loss this slice repairs.

Upstream always sends both fields (`ErrorBody { code: String, message: String }`,
`upstreams/herdr/src/api/schema/response.rs:37-40`). A response missing `code` is still
mapped to `Remote` with an empty code — never to `Malformed`, because turning a real
refusal into "malformed response" would hide the server's own message from the operator.

---

## Cells

Sequential — all three touch overlapping files, and the trait change forces the socket and
fake implementations to land with it or nothing compiles.

### Cell 1 — the error surface

**Files:** `src/herdr/mod.rs`, `src/herdr/socket.rs`
**Type:** behavior_change

Add the four variants. Change `parse_response` to read `error.code` alongside
`error.message` and map the three known codes to their typed variants, everything else to
`Remote`.

Touches an existing contract: `parse_response_maps_error` (`socket.rs`) asserts today's
message-only collapse into `Request`. That assertion is deliberately replaced — a coded
refusal is no longer a `Request`. This is the one place in the slice where existing proof
changes, and it changes because the contract it proved is the defect.

**Verify:** `cargo test --quiet --lib herdr::socket`
New tests, each of which must fail before the change (the variants do not exist, so it will
not compile — that is the required red):
- an error envelope with code `agent_name_taken` maps to `AgentNameTaken`
- ditto `workspace_not_found` → `WorkspaceNotFound`, `invalid_agent_argv` → `InvalidAgentArgv`
- an unknown code (`tab_create_failed`) maps to `Remote` with the code string preserved
- an error envelope with no `code` key maps to `Remote` with an empty code and the message intact
- a local IO/serialize failure still maps to `Request`

### Cell 2 — `tab.create`

**Files:** `src/herdr/mod.rs`, `src/herdr/socket.rs`, `src/herdr/fake.rs`
**Type:** behavior_change · **Depends on:** cell 1

Trait method, socket implementation, fake implementation.

Parameters, from `TabCreateParams` (`upstreams/herdr/src/api/schema/tabs.rs:8-19`) narrowed
by the locked decisions:

- `workspace_id` — always sent explicitly. The upstream default ("the active workspace") is
  the desktop's idea of active, which is meaningless for a phone client.
- `cwd` — always sent explicitly, per **D5** (`CONTEXT.md:33`). Never omitted, never left to
  the desktop's `NewTerminalCwdConfig` policy.
- `focus: false` — per **D6** (`CONTEXT.md:36`). Creating from the phone must not steal the
  desktop's focus.
- `label`, `env` — not sent. Nothing in slices 3-5 asks for them.

Result: `tab_created` carrying `tab` and `root_pane`. The port returns the new pane id and
tab id — that is what slice 4 needs to route the phone straight into the new terminal.

The socket side follows the existing hand-rolled extraction idiom (`socket.rs:285-333`),
not a new deserialization layer.

The fake **actually creates**: it appends the tab and its root pane to its snapshot under
lock, so a later `snapshot()` sees it. A stub that returns a plausible id without mutating
state would let slice 4's end-to-end test pass while proving nothing.

**Verify:** `cargo test --quiet --lib herdr`
- fake: creating a tab in a known workspace makes a new tab and pane appear in the next
  snapshot, with the requested cwd
- fake: creating in an unknown workspace returns `WorkspaceNotFound`
- socket: the params builder emits exactly `workspace_id`, `cwd`, `focus: false` — asserted
  as a pure function against a `Value`, no socket needed (the `parse_snapshot` seam
  precedent, `plan.md:69`)

### Cell 3 — `agent.start`

**Files:** `src/herdr/mod.rs`, `src/herdr/socket.rs`, `src/herdr/fake.rs`
**Type:** behavior_change · **Depends on:** cell 2

Parameters, from `AgentStartParams` (`upstreams/herdr/src/api/schema/agents.rs:33-48`):

- `name` — required, caller-generated per **D7** (`CONTEXT.md:37`).
- `argv` — required, non-empty. Empty is the only upstream validation, and it is what
  slice 3's preset check will lean on.
- `cwd` — always sent explicitly, per D5.
- `workspace_id` — sent. `tab_id` and `split` are **not** sent: sending both a tab and a
  workspace opens `agent_placement_conflict` for no product gain, and the phone has no
  concept of split direction. The upstream default (`Right`, off the workspace's active
  tab's focused pane) is accepted.
- `focus: false` — per D6.

**The retry belongs here, not to the caller.** D7 says the name is auto-generated and
retried on collision; slice 4 should not have to know that. The port owns generate → call →
on `AgentNameTaken` regenerate → call again, with a bounded number of attempts and a
distinguishable error when it gives up. Bound: 5. A sixth collision means something is
generating names wrongly, and looping harder would only hide it.

**Verify:** `cargo test --quiet --lib herdr`
- fake: starting an agent adds a pane whose agent name is the requested one, visible in the
  next snapshot
- fake: a name already in use returns `AgentNameTaken` carrying the name
- fake: empty argv returns `InvalidAgentArgv`
- port: a caller hitting one collision then succeeding ends with the second name; a caller
  colliding 5 times gets a terminal error rather than an infinite loop
- socket: the params builder emits `name`, `argv`, `cwd`, `workspace_id`, `focus: false`
  and **no** `tab_id`/`split` key

---

## Deliberately out of scope

- **No read timeout on the socket loop.** `call` (`socket.rs:183-198`) can block forever;
  create calls spawn a process and are the slowest requests this client will ever make, so
  this slice makes an existing hazard more reachable. It is still a pre-existing defect of
  the shared transport, not of the create verbs, and fixing it inside a scoped slice would
  change every existing call path. Filed as friction instead.
- **Whether `agent.start` succeeded from the agent's point of view.** The upstream response
  confirms the pane and terminal exist, not that the agent process finished starting; its
  `agent_status` at that instant is typically not yet meaningful. Callers learn readiness
  from the existing snapshot/poll path. Nothing in this slice should pretend otherwise.
- `label` and `env` params on either verb.

## Feasibility findings (validation pass)

Four things checked against the code, three of which changed the plan.

**1. The fake can genuinely create.** `Snapshot`, `Pane`, `Tab`, `Workspace` and `PaneLayout`
are all plain public-field structs deriving `Clone` + `Default` (`wire.rs:98-174`), and
`set_status` (`fake.rs:144-153`) already establishes mutate-snapshot-under-lock. Appending
is free. No blocker.

**2. `wire::Agent` has no `name` field — plan widened.** Fields are `pane_id`,
`workspace_id`, `tab_id`, `kind`, `status`, `title` (`wire.rs:45-58`). herdr does send
`name` on `AgentInfo`; this port simply never parsed it. Consequences, both real:
the fake cannot detect a name collision from its own state, so `agent_name_taken` would
have to be faked from side-state that no snapshot can corroborate; and cell 3's stated
verify — "the agent's name is visible in the next snapshot" — is unwritable today.

So slice 2 also adds `name` to `wire::Agent` and its three population sites. `wire.rs`
becomes a fourth product file. It is one field, `#[serde(default)]` so every existing
JSON-fixture test stays green, and only `fake.rs`'s `agent()` helper constructs an `Agent`
literally. Note `Agent` derives `Serialize` but not `Deserialize` — parsing is hand-rolled,
so the new field must be added to the hand-written parse, not just the struct.

**3. The fake must seed a screen for every pane it creates.** `read_pane`
(`fake.rs:205-215`) returns `NoSuchPane` when a pane has no `screens` entry. Slice 4 routes
the phone straight from a successful create into that terminal, so a created pane with no
screen entry would make the very next call fail. Both create verbs must insert a screen
entry — same as `FakeHerdr::new` already does for its seeded shell (`fake.rs:97`).

**4. `tab_create` must append a `PaneLayout` too.** The D10 anchor join
(`anchor_cwd_for_workspace`, `wire.rs:216-246`) walks workspace → `active_tab_id` →
`layouts[]` → `focused_pane_id` → pane. A created tab with no layout row is invisible to
that join. It does not break today's callers — `focus: false` means `active_tab_id` does
not move — but leaving the row out would make the fake's snapshot a shape real herdr never
produces, which is exactly how a fake stops being evidence.

## Risk register

| risk | why it is contained |
|---|---|
| Changing `parse_response` touches every existing call path (snapshot, ping, read_pane, send_input, send_keys) | `Request` keeps its meaning for local failures; only *server refusals* move to new variants, and each existing path is covered by the current test suite, which stays green apart from the one deliberately updated assertion |
| The fake diverges from real herdr | The params builders are asserted against the captured upstream schema shape, and slice 1's tracked live capture remains the fixture of record |
| Name-collision retry masking a generator bug | Bounded at 5 with a distinct terminal error |
