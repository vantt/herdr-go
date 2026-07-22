# Backlog Auto-Triage Pipeline Split — Context

**Feature slug:** backlog-auto-triage
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Standard
**Domain types:** ORGANIZE

## Feature Boundary

Split today's `bee-exploring` into three cooperating bee-skill stages — an automatic
triage stage, a shared CONTEXT.md-writing stage, and a narrowed human-interactive
exploring stage — so a clear backlog item can flow unattended from submission to
"ready for pickup" while an ambiguous item is parked with a gathered brief instead
of blocking on a synchronous human question. This feature locks the architecture
and behavior only; it does not lock final skill names for the two new stages, and
it does not write code.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Split `bee-exploring` into three stages: (1) an automatic triage stage that runs the moment a backlog item exists, no human involved; (2) a shared CONTEXT.md-writing stage used by both the auto path and the human path; (3) a narrowed `bee-exploring` that only runs when a human explicitly picks up a parked item. | Today `bee-exploring` is the only entry point and always blocks on Socratic questions for ambiguous items — this stops any pipeline (herdr-orchestrating's dispatch loop, or any future orchestrator) from running unattended. |
| D2 | The triage stage first does a real gather step (reads the backlog row plus related code/docs/specs) before assessing anything — never assesses from the raw backlog text alone. | Matches exploring's existing "quick scout" discipline; a decision made without gathering evidence first is not trustworthy. |
| D3 | Clarity/size assessment in triage is an LLM self-assessment over the gathered evidence, explicitly NOT a keyword/regex classifier. | User rejected script-only classification, citing `herdr-orchestrating`'s `classify-lane.mjs` as the anti-pattern to avoid: it fails open on any row whose danger isn't spelled in its keyword list (proven live against both an English and a Vietnamese unsafe row in that feature's own validation). |
| D4 | Clear-item path: triage hands its locked decisions to the shared CONTEXT.md-writing stage, then auto-approves Gate 1, runs `bee-planning`, then auto-approves Gate 2, then marks the item ready/in-flight for an orchestrator to pick up — subject to D7's gate-bypass coupling (no synchronous human step only on lanes `gate_bypass_level` actually covers; otherwise this path still stops and asks, same as today). | Deliberate automation increase for the case triage judges genuinely unambiguous, bounded by D7 so this table can't be implemented divergently from it. |
| D5 | Ambiguous-item path: triage does NOT ask the human synchronously. It writes a brief (what was gathered, what's unclear) into the feature's own `docs/history/<slug>/` via the shared CONTEXT.md-writing stage (reusing CONTEXT.md's existing `Outstanding Questions` section rather than inventing a new brief file format), then stops — the item is "parked". | Reuses existing CONTEXT.md structure (YAGNI/DRY) instead of a second artifact format; headless-mode exploring already writes gray areas into `Outstanding Questions` instead of asking, so this path has direct precedent in the current skill. |
| D6 | Any item carrying a hard-gate risk flag (auth, authorization, data loss, audit/security, external provider, validation removal — same flag set as the mode-gate's risk-flag list) is always parked for a human, regardless of how confidently triage can self-assess it. | Mirrors the precedent this same user already set in `herdr-orchestrating`'s dispatch role (decision D6 of `agent-pane-orchestration`): "when unsure, refuse — refusal is the safe default," extended here to "hard-gate is never auto-cleared, full stop," so an unattended stage never silently walks past auth/data-loss/security work. |
| D7 | Triage's auto-approval of Gate 1 and Gate 2 on the clear-item path is coupled to the existing `gate_bypass_level` switch — it only auto-approves when the level covers that lane (matching the rest of the system), never as an independent bypass mechanism. When `gate_bypass_level` is `off`/`normal` on lanes it doesn't cover, triage's clear-item path still stops and asks at Gate 1/2 like today's `bee-exploring`/`bee-planning` do. | Explicit user answer (this session) to the one genuine open question: avoids a second, parallel safety-control source: turning the global bypass off must also stop triage's auto-approval, not leave a separate always-on channel. |
| D8 | The shared CONTEXT.md-writing stage is the single place that writes `docs/history/<feature>/CONTEXT.md`, regardless of whether the input decisions came from triage's self-assessment or from a human-resolved Socratic session. | Avoids two divergent implementations of "how CONTEXT.md gets written" (DRY) — explicit user requirement. |
| D9 | The narrowed `bee-exploring` (human path) loads the brief triage already gathered when a human picks up a parked item — it does not re-gather from scratch — then runs the interactive Socratic dialogue only over the still-unresolved gray areas, and hands the resolved decisions to the shared CONTEXT.md-writing stage. | Avoids redundant gather work; the brief is exactly the input exploring needs to resume. |
| D10 | The triage/shared-writer logic must live at the bee skill layer, tool-agnostic — never specific to `herdr-orchestrating`. Any orchestrator (herdr, or a future replacement) drives this only by invoking bee skills as sequential, non-overlapping stages. | Explicit user principle: "herdr là một tool orchestrating có thể được thay bởi tool khác, quan trọng là bee skill phải support cho dù dùng tool gì để orchestrate." `herdr-orchestrating`'s dispatch role becomes a caller of this pipeline, not its owner. |

### Agent's Discretion

None delegated in this session — all decisions above were either explicitly stated
by the user or locked as confident recommendations under the gate-bypass
information-vs-approval split (D2 of hive's Socratic Locking step), each with its
rationale cited above.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Triage | The new automatic stage (name not yet locked) that gathers evidence for a backlog item and self-assesses whether it can proceed unattended or must be parked. |
| Parked | An item triage judged too ambiguous/large to auto-resolve: its brief is written, and it waits for a human to pick it up — not blocking, not retried automatically. |
| Ready / in-flight | The state a clear item reaches after triage's auto path completes planning and both gates auto-approve — the same state an orchestrator's dispatch logic (e.g. `herdr-orchestrating`'s D1 dispatchability check) already looks for today. |

## Specific Ideas And References

- User's own prior design in this repo, `herdr-orchestrating`'s dispatch role §6 (Lane-safety filter, D6): a two-key gate (script classifier + the agent's own reading) that refuses on any doubt. Cited directly as the precedent behind D6 above and as the anti-pattern behind D3 (script-only classification is explicitly rejected as the *sole* signal, even though the two-key idea of "never trust a keyword list alone" carries over).

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `.claude/skills/bee-exploring/references/context-template.md` — the exact CONTEXT.md template the shared writing stage (D8) must keep producing; both the auto and human paths write through this same shape.
- `.claude/skills/bee-exploring/SKILL.md` step 5 ("Context Assembly") — today's only implementation of CONTEXT.md-writing; this is the logic D8's shared stage extracts out.
- `.claude/skills/bee-exploring/SKILL.md` Headless section — already writes gray areas into `Outstanding Questions` instead of asking; direct precedent for D5's park-with-brief behavior.
- `docs/backlog.md` — the PBI table triage watches for new/`proposed` rows and flips per the existing `D11a` "backlog flip" convention (`bee-exploring/SKILL.md` step 1).
- `.claude/skills/herdr-orchestrating/SKILL.md` Dispatch role §5-6 — the existing "dispatchable set" + lane-safety two-key gate this feature's D3/D6 draw precedent from, and the consumer that will read triage's "ready/in-flight" output.

### Established Patterns

- Gate-bypass information-vs-approval split (`bee-exploring/SKILL.md` step 4) — the pattern this session itself used to decide which of the 4 candidate gray areas needed asking vs. locking with a cited rationale; the same pattern the new triage stage's Gate 1/2 auto-approval (D7) plugs into.
- `bee.mjs state gate --name <gate> --approved true` + `decisions log` — the existing CLI-driven mechanism today's bypass-driven auto-approval already uses (`bee-exploring/SKILL.md` step 6); D7's coupling means triage reuses this unchanged, not a new mechanism.

### Integration Points

- `.bee/bin/bee.mjs` (`worktree`, `cells`, `decisions`, `state gate` verbs) — triage's clear-item path (D4) drives the same CLI surface `bee-exploring`/`bee-planning` already drive.
- `docs/backlog.md` Status column (`proposed`/`in-flight`/`done`) — triage's clear path and park path both need to read/write this same table today's exploring already flips (D11a).

## Canonical References

- `.claude/skills/bee-hive/SKILL.md` — onboarding text states the `.claude/skills/bee-*` and `.agents/skills/bee-*` trees in this repo are **rendered projections** from bee's own separate source tree ("a rendered projection is never accepted back as an onboarding source"). Planning must locate that source before any skill file is edited — editing the rendered copy in this repo directly is not the real change.
- `.claude/skills/bee-evolving/SKILL.md` description — confirms bee's own self-improvement work runs "in the bee repository only," never in a host repo. Directly relevant to where this feature's implementation actually lands.

## Outstanding Questions

### Resolve Before Planning

- [ ] Final names for the two new skills (candidates so far: "bee-triage", "bee-context" — user explicitly rejected both as unsatisfying and wants a dedicated naming brainstorm session before planning proceeds). Planning cannot create skill files without settled names.

### Deferred To Planning

- [ ] Where does implementation actually happen — bee's own source repo (per the Canonical References above) vs. this repo's rendered copies? Investigate via `.bee/onboarding.json`'s recorded source, or ask the user directly if not resolvable from repo state.
- [ ] Exact gather-step scope for triage (which files/how much to read before self-assessing) — implementation detail, not a product decision.
- [ ] Exact brief format inside CONTEXT.md's `Outstanding Questions` section for a parked item vs. a normal in-progress one — needs to stay distinguishable to whichever process later resumes it.

## Deferred Ideas

- None captured this session — this feature is itself the deferred-idea follow-through from the `agent-pane-orchestration`/`herdr-orchestrating` work; no further spin-off ideas surfaced during locking.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
Validating and reviewing use locked decisions for coverage and UAT.

**This session intentionally stops here, not at `bee-planning`.** Skill naming
(Outstanding Questions → Resolve Before Planning) is explicitly unresolved and the
user has asked for a separate naming brainstorm next — planning should not be
invoked until that lands.
