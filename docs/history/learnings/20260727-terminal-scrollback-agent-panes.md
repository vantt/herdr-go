---
date: 2026-07-27
feature: terminal-scrollback-agent-panes
categories: [investigation, validation-process, cell-authoring, dependency-boundary]
severity: medium
tags: [herdr, terminal-detail, hexagonal-port, false-escalation, validation-loop, vendored-dependency]
---

# Terminal scrollback for agent panes — what happened, why, and what to do differently

## What Happened

A prior investigation (`docs/history/learnings/20260718-terminal-scrollback-herdr-limit.md`)
concluded the gateway's terminal-host client had no scrollback capability at
all and recommended filing an upstream feature request. Live re-verification
this session overturned that: the gateway simply never requested more than
the host's default 80-line window. Primary-screen panes (plain shells, and
two of three coding-agent CLIs tested) already had real scrollback available
for free. Only full-screen ("alt-screen") panes are genuinely history-less —
and even that case has a working answer with no dependency change needed:
replaying the agent's own scroll keybinding through a raw-byte input channel,
verified live against one such agent.

Two rejected approaches were seriously investigated before this answer
emerged: reverse-engineering an undocumented binary attach protocol, and
forking the vendored terminal-host dependency to expose scrollback directly.
Both were correctly killed once evidence showed the real limitation was a
fundamental property of the underlying terminal protocol itself (a
full-screen program's mode never retains history, in any implementation) —
neither a different protocol nor a different fork of the same engine can
produce data that was never captured.

The build itself went through 3 validation rounds before shipping. Round 1
caught a false-escalation logic bug (comparing an *approximated* baseline
read instead of the real one). Round 2's own fix for that bug introduced 2
NEW blockers — it assumed viewport-dimension data existed on the gateway's
side that, in fact, does not exist anywhere reachable without repeating the
exact stale-field cross-reference the original design had already rejected.
Round 3 resolved this by dropping the added complexity rather than inventing
a way to source the missing data, accepting the original (low-cost) problem
as a trade-off instead.

## Root Cause

- The stale prior conclusion existed because nobody had tried requesting a
  longer window on the same read the gateway already had — an unverified
  "there is no scrollback" claim went unchallenged for over a week.
- The false-escalation bug (round 1) existed because the comparison baseline
  was approximated instead of using the terminal host's own real "current
  screen" read, which was available all along.
- Round 2's own new blockers existed because a fix was accepted as correct
  by virtue of resolving the finding it targeted, without the same
  evidence-based scrutiny (does this data actually exist on our side?) that
  the original plan itself had already been put through.
- Two cell-authoring mistakes (an incomplete file list omitting 2
  compile-required files, and a `must_haves` internal contradiction left
  over from a dropped mechanism) reached round 3 before being caught,
  because the cell's file list was drawn from the primary implementation
  location rather than a full trait-consumer sweep, and `must_haves` text
  was not re-diffed after removing a mechanism from the plan.

## Recommendation

- When a status/metadata field is ambiguous or possibly stale, compare two
  actual reads taken through the real production code path instead of
  trusting the field — never approximate one side of that comparison, since
  an approximation reintroduces the exact ambiguity the comparison exists to
  remove.
- When validating a fix for a prior finding, apply the same evidence
  standard to the fix's own new claims as to the original plan — a fix is
  not exempt from scrutiny just because it targets a finding; check whether
  any new mechanism it introduces actually has the data it depends on
  available in this codebase before accepting it.
- Before working around a vendored/wrapped dependency (forking it,
  reverse-engineering an internal protocol) to expose missing data, first
  verify whether the underlying protocol or engine actually retains that
  data at all. If the limitation is a fundamental property of the transport
  or protocol itself, no amount of dependency-side work will produce it —
  the fix has to come from a different data path, or the boundary has to be
  accepted.
- When authoring or updating a cell that changes a shared trait method's
  signature, grep the whole repository for every `impl <Trait>` block and
  every call site before finalizing the cell's file list — not just the
  ones already known from the primary implementation. A signature change's
  blast radius is a repo-wide fact, not a guess from familiarity.
- After removing a mechanism from a plan mid-flight (a dropped gate, a
  changed strategy), re-read every remaining `must_haves` truth line by line
  against the new mechanism — leftover wording from the removed mechanism
  creates internally contradictory acceptance criteria that a cold-pickup
  reviewer, not the author, ends up catching.
