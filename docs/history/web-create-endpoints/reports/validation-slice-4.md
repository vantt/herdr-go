# Validation — slice 4 (web create endpoints)

Advisor: **none configured** (`.bee/config.json` carries no `advisor` key;
`models.claude` has `extraction` and `generation` only). Per AO2(b) an
unconfigured advisor is not a hard dependency — recording the fact and
proceeding.

Feasibility was established by an independent read-only validation pass rather
than a spike, and the pass was load-bearing. Seven problems, all folded into the
cells before the execution gate.

## The correctness defect

`agent.start` with `cwd` omitted does **not** resolve the workspace anchor. It
falls back to `std::env::current_dir()` of the herdr **server process**
(`upstreams/herdr/src/app/agents.rs:118-122`), an arbitrary folder. `tab.create`
does resolve the workspace's own anchor
(`upstreams/herdr/src/app/api/tabs.rs:65-67`). The plan assumed symmetry from
the `tab.create` reading and was wrong. Starting an agent in an arbitrary folder
is exactly the silent wrong-repo start parent D5 forbids — and it would have
been invisible in tests, because `FakeHerdr` never modelled the fallback at all.

Fix: **P10** — the shell route omits `cwd`; the agent route refuses with 409.
`plan.md` stays frozen; the correction lives in CONTEXT.md P10, cells 2 and 4,
and a logged decision.

## The fake was kinder and narrower than the live client

Four gaps, each making a headline must-have unprovable while the suite stayed
green:

1. Every seeded pane sets `foreground_cwd == cwd` (`src/herdr/fake.rs:287-295`),
   so `path_is_live: false` was unreachable — though the live capture proves
   `cwd`-only panes are a real shape (`src/herdr/socket.rs:552-566`).
2. Every seeded workspace has agents, so the shell-only workspace this feature
   exists for could not be exercised through the router.
3. The fake returns `WorkspaceNotFound` for an unknown workspace on
   `agent.start`, where the live server returns `agent_placement_not_found`
   (`upstreams/herdr/src/app/agents.rs:152-156,222`).
4. `FakeHerdr` exposes no seed mutator, so a web test cannot shape a snapshot at
   all — every case must come from the static seed.

Fix: the seed work moved into cell 2, which owns `src/herdr/fake.rs`; cell 3
now depends on cell 2. A join **miss** stays proven at cell 1's pure-function
level rather than seeded, so `envelope_fake_seed_joins`
(`src/herdr/fake.rs:442-476`) keeps passing unchanged.

## The third construction site

`AppState::new` has three callers, not two: `src/main.rs:264`, `test_state()`
(`src/web/mod.rs:88-96`), and **`tests/observe_reply_e2e.rs:13`** — outside cell
3's file scope. A signature change would have reddened the cell's own verify on
a file it is forbidden to touch. Fix: the preset list arrives as a field, and
`AppState::new`'s signature is now a stated prohibition.

## Confirmed, no change needed

- **Every verify command fails today.** All four prefixes (`provenance_`,
  `createcwd_`, `createoptions_`, `createroute_`) were executed against the
  current tree: `0 passed`, grep exit 1, zero substring collisions in `src/` or
  `tests/`. A verify that cannot fail cannot tell done from not-started.
- **Error codes exist and map as planned.** `agent_placement_not_found`
  (`agents.rs:222`) and `agent_placement_conflict` (`agents.rs:226`) arrive as
  `HerdrError::Remote{code}` through the default arm at
  `src/herdr/socket.rs:272-290`; `tab.create`'s `workspace_not_found` becomes
  `HerdrError::WorkspaceNotFound`, id filled by `attach_workspace_id`
  (`socket.rs:373-382`).
- **`cwd` is optional on both verbs** —
  `upstreams/herdr/src/api/schema/tabs.rs:11-12` and
  `.../agents.rs:35-36`. Only the *fallback* differs.
- **Route shapes do not collide.** axum 0.7.9 merges two methods on one path
  (`routing/path_router.rs:59-73`), and matchit registers `/api/panes` beside
  `/api/panes/:pane/screen` without conflict. Source-read, not executed — no
  existing repo route currently carries two methods, so cell 4 is the first
  proof.

## Noted, not acted on

- `agent_placement_conflict` is unreachable from this client: upstream raises it
  only when `tab_id` and `workspace_id` disagree (`agents.rs:133-141`), and
  `agent_start_params` never sends `tab_id` (`src/herdr/socket.rs:389-397`).
  Mapping it is harmless defensiveness; a test for it would be fabricated.
- Baseline verify was green before any of this work.
