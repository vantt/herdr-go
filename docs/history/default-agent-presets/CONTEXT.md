# Default agent presets — Context

**Feature slug:** default-agent-presets
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Quick
**Domain types:** ORGANIZE

## Feature Boundary

A fresh install (no existing `config.json`) ships with three `agent_presets`
entries (Claude, Codex, Agy) already populated, so the create-sheet's "New
agent" flow works with zero manual config editing; any operator who already
has a `config.json` — including one with a deliberately empty
`agent_presets: []` — is never touched, and the existing doctor editor
remains the full override path.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Seed the 3 default presets only inside the default-config template `config::ensure_config` writes when `config.json` does not yet exist (`src/config/mod.rs:846-864`). An existing file — any content, including an intentionally empty `agent_presets` — is never rewritten; `ensure_config` already guarantees this (`if !path.exists()`). | Matches "zero-config for new users, full override for anyone who customized" exactly — no new mechanism needed, only the seeded content changes. |
| D2 | Doctor's unparseable-config recovery path (`default_config_json`, `src/doctor/checks.rs:785-798`) seeds the identical 3 presets, so a repaired config re-lands on the same zero-config baseline as a brand-new install. | Consistency: `ensure_config` and `default_config_json` already duplicate the same default document today (pre-existing, unrelated to this feature); the two must stay in sync when the seeded content changes, or repair silently regresses to no presets. |
| D3 | No PATH/binary-existence probing before seeding or offering a preset. An operator tapping a preset whose binary isn't installed gets the existing inline create-failure UX (create-sheet.md's on-failure behavior) — same as today for any preset that fails to spawn. | YAGNI — matches the existing documented posture that an empty preset list is "a normal, fully-booting gateway"; probing adds cross-platform PATH-resolution complexity for a case already handled by existing error surfacing. |
| D4 | No changes to doctor's guided settings editor (`src/doctor/edit.rs:175` `edit_agent_presets`). It already supports list/add/remove of individual presets — an operator who wants to drop or change one of the 3 defaults already has a path today. | Scout-confirmed: add/remove/list already shipped (doctor-config-surface feature), not append-only. |
| D5 | Default preset **Claude**: `{"label": "Claude", "argv": ["claude", "--dangerously-skip-permissions"]}` — bypasses Claude Code's permission prompts so the agent runs unattended once created from the phone. | User-selected: explicit trade of the permission gate for zero-touch autonomy, consistent with this repo's own bee full-autopilot posture. |
| D6 | Default preset **Codex**: `{"label": "Codex", "argv": ["codex", "--full-auto"]}` — Codex CLI's sandboxed auto-approve mode (auto-approves edits/commands within its own sandbox, does not remove the sandbox itself). | User-selected the `--full-auto` option specifically (not the stronger `--dangerously-bypass-approvals-and-sandbox` flag) — noted as an asymmetry vs D5/D7's full bypass in Outstanding Questions below since the user's own words also said "or an equivalent bypass flag", not necessarily this exact one. |
| D7 | Default preset **Agy**: `{"label": "Agy", "argv": ["agy", "--dangerously-skip-permissions"]}` — user confirmed `agy` is a real CLI they have installed, invoked bare plus the same permission-bypass flag pattern as Claude. | User-supplied; not a tool with any prior reference in this repo, so its exact behavior beyond the confirmed invocation is unverified. |
| D8 | Preset order in the seeded array, and therefore in the create-sheet's action list: Claude, Codex, Agy (Shell always renders first, ahead of all presets, per existing `create-sheet.ts` behavior — unchanged). | Matches the order the user listed them in; no signal favors any other order. |
| D9 | Preset labels are Title-Case (`Claude`, `Codex`, `Agy`), matching the existing test-fixture convention (`src/doctor/edit.rs:524,568`, `src/config/mod.rs:1114,1163`, `src/web/create.rs:168`, `src/web/api.rs:384` already use `"Claude"`/`"Codex"` as example labels) rather than the lowercase binary name. | Consistency with existing convention found in scout; cosmetic, non-blocking if changed later. |

### Agent's Discretion

Exact JSON formatting/whitespace of the seeded `agent_presets` array inside
the two default-document string templates (`ensure_config`,
`default_config_json`) — planning/execution may format however keeps the
existing `format!` string readable, as long as the parsed result matches
D5-D9 exactly.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Default preset | An `AgentPreset` entry present in a config.json only because it was seeded by `ensure_config`/`default_config_json` at creation time — indistinguishable in the file from one the operator typed by hand once written; "default" describes provenance, not a marked/protected status. |

## Specific Ideas And References

- User confirmed via `--dangerously-skip-permissions` (Claude) / `--full-auto`
  (Codex) / `--dangerously-skip-permissions` (Agy) that these presets should
  run unattended, not require interactive approval per action once launched
  from the phone.

## Existing Code Context

### Reusable Assets

- `src/config/mod.rs:41-63` — `Config.agent_presets: Vec<AgentPreset>` and the `AgentPreset { label, argv }` shape; already exactly what's needed, no shape change.
- `src/config/mod.rs:846-864` — `ensure_config`, the only place a default `config.json` is written for a genuinely fresh install.
- `src/doctor/checks.rs:785-798` — `default_config_json`, the doctor unparseable-config recovery path's equivalent default-document generator.
- `src/doctor/edit.rs:164-264` — `edit_agent_presets`, the existing full add/remove/list editor; confirmed no change needed (D4).
- `src/web/create.rs:77-115` — `create_agent`, which looks up a preset by label and passes its `argv` straight to `agent.start`; unaffected, presets are just data to it.

### Established Patterns

- `RawAgentPreset`/`AgentPreset` round-trip via `Config::load_str` (`src/config/mod.rs:139-144, 275-278`) — any seeded JSON must satisfy the same validation already enforced for hand-written presets (non-empty label, unique label, non-empty argv, non-empty argv[0]) with zero special-casing for "default" entries.
- Existing test fixtures already use `"Claude"`/`"Codex"` as sample labels (`src/doctor/edit.rs`, `src/config/mod.rs`, `src/web/create.rs`, `src/web/api.rs` tests) — informed D9's Title-Case convention, though those fixtures are arbitrary test data, not prior product decisions.

### Integration Points

- `web/src/views/create-sheet.ts` — renders one action row per preset returned by `GET /api/create-options`; no change needed, it already renders whatever the backend returns.

## Canonical References

- `src/config/mod.rs` — `Config`, `AgentPreset`, `ensure_config`.
- `src/doctor/checks.rs` — `default_config_json`.
- `src/doctor/edit.rs` — `edit_agent_presets`.
- `docs/specs/create-sheet.md` — action-row/preset behavior contract (unchanged by this feature).

## Outstanding Questions

### Resolve Before Planning

*(none — all decisions locked)*

### Deferred To Planning

- [ ] Whether `codex --full-auto`'s sandboxed auto-approve is the behavior the user actually wants long-term, or whether they'd prefer symmetry with D5/D7's full permission bypass (`--dangerously-bypass-approvals-and-sandbox`) — the user picked `--full-auto` explicitly (D6) but also said "or an equivalent bypass flag," leaving room to revisit if `--full-auto` turns out to still prompt for anything in practice. Planning should confirm the exact current Codex CLI flag spelling against Codex's own `--help`/docs before writing the cell, since a wrong flag spelling breaks the preset on every fresh install silently.
- [ ] Whether `agy --dangerously-skip-permissions` is actually a valid invocation for whatever `agy` is — unverified beyond the user's confirmation; if `agy` turns out not to support that flag, the preset would need a plain fallback.

## Deferred Ideas

- Preset icons / visual distinction between Shell and each preset row in create-sheet — out of scope, cosmetic, no signal this was wanted.
- A "recommended presets" catalog the operator can pick from during `doctor` setup (rather than 3 always-seeded presets) — deferred; the locked scope here is exactly "3 defaults present unless the operator has already customized," not a picker UI.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads
locked decisions, code context, canonical references, and the
deferred-to-planning questions above (confirm exact Codex/Agy flag spelling
before cutting cells).
