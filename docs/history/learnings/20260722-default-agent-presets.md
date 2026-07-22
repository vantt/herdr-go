---
date: 2026-07-22
feature: default-agent-presets
categories: [pattern, decision, failure]
severity: critical
tags: [config-defaults, cell-scoping, security-defaults, verification-evidence, decision-reopening]
---

# Learning: Seeding security-relevant CLI defaults needs CLI-verified evidence, not search-verified evidence

**Category:** failure
**Severity:** critical
**Tags:** [verification-evidence, security-defaults, external-cli]
**Applicable-when:** locking any config default, preset, or template that hardcodes a flag/argument for an external CLI the target machine may or may not have — especially when the flag changes a permission/sandbox/approval posture.

## What Happened

Planning locked `codex --sandbox danger-full-access --ask-for-approval never` (D10, superseding a deprecated `--full-auto` choice) using a WebSearch against `developers.openai.com` as evidence. Compounding's failure analyst later ran `codex --help`, `claude --help`, and `agy --help` directly on the same machine and confirmed all three seeded flags (`--dangerously-skip-permissions` x2, `--sandbox`/`--ask-for-approval`) are exact matches — the WebSearch-based decision turned out correct. But the CLIs were reachable and free to query the whole time; nothing about the plan required accepting a lower evidentiary bar for a decision this security-relevant (it ships permission/sandbox bypass as the *default*, unconditionally, on every fresh install).

## Root Cause

The planning workflow's L1 discovery step accepted WebSearch as sufficient without asking whether stronger evidence (`--help`, `--version`) was cheaply available. This project already has a stated evidentiary bar for cell-level verification (`verification_evidence`, `red_failure_evidence`, real command output) — that bar simply wasn't extended to the planning-time factual claim underlying the decision itself.

## Recommendation

When a decision hardcodes an external CLI's flag/argument into a shipped default, and the CLI binary is reachable on the machine doing the planning, run `<cli> --help` (or equivalent) as confirming evidence before locking the decision — treat WebSearch/docs as a starting point, not the final source, whenever the binary itself is one command away. When the binary is *not* reachable, say so explicitly in the decision's rationale ("unverified against the real CLI, WebSearch/docs only") rather than presenting search-derived confidence as if it were CLI-verified.

---

# Learning: A "no mitigation needed" decision must be checked against precedent already in the file it names as the fallback

**Category:** failure
**Severity:** critical
**Tags:** [decision-quality, scope-discipline, precedent-check]
**Applicable-when:** locking a decision that declines a mitigation (probing, validation, a guard) by citing "existing X already handles this" — before locking, grep X for whether it actually does.

## What Happened

D3 locked "no PATH/binary-existence probing before seeding agent presets," reasoning that probing "adds cross-platform PATH-resolution complexity for a case already handled by existing error surfacing." Compounding's failure analyst found `src/doctor/checks.rs:262-272`'s `herdr_version()` already implements exactly this pattern for the `herdr` binary — `Command::new(x).output().ok()?`, zero PATH-walking code, the OS resolves it. The "complexity" cited as the reason to skip a doctor-level check was never real; a same-shape check for `claude`/`codex`/`agy` would have been the same order of effort as the existing precedent.

## Root Cause

The decision was locked from first-principles reasoning ("probing sounds complex") without grepping the very module (`src/doctor/checks.rs`) the decision's own rationale named as the existing fallback. YAGNI reasoning is only as good as the "already handled" claim it rests on.

## Recommendation

Before locking a decision that declines a mitigation by name-dropping an existing fallback ("X already handles this," "Y already exists for this case"), grep the named file/module for a literal precedent of that exact mitigation shape. If the precedent exists and is cheap to extend, the "adds complexity" framing is false and the decision needs to be re-evaluated on its actual (not assumed) cost.

---

# Learning: A universal default seeded from one user's personal/local tool is a portability risk the decision record must flag explicitly

**Category:** decision
**Severity:** standard
**Tags:** [portability, single-operator-scope, user-supplied-fact]
**Applicable-when:** a user asks for a default/preset backed by a tool that turns out to have no public distribution — before shipping it as unconditional/universal, confirm whether "universal" actually means "this one operator's own machines."

## What Happened

D7 seeded an `agy` preset (`agy --dangerously-skip-permissions`) as a universal default in every freshly created `config.json` — including doctor's config-repair path — based solely on the user confirming "agy is a real CLI I have installed." Compounding's failure analyst traced `agy` to `~/.local/bin/agy`, a stripped ELF binary wrapping Gemini models with no npm/pip/package-manager presence and zero other reference in the repo — a personal tool, not a broadly distributed CLI. Because D3 (above) declined any PATH-probing, a fresh install on a *different* machine (not the one where this decision was made) seeds a preset that will permanently fail to spawn, surfaced only as a raw OS spawn-error string on the phone's create-sheet.

Given this project's own PRD frames it as a single-operator system typically run on one machine the operator controls, this is a low-urgency finding in practice — but it was never stated as an assumption in CONTEXT.md, so a future reinstall-on-a-new-machine or any move toward multi-operator use would hit it silently.

## Root Cause

CONTEXT.md's own "Deferred To Planning" section correctly flagged `agy`'s invocation as "unverified beyond the user's confirmation," but nothing turned that flag into a blocking gate item or a scoped decision ("seed only if this machine currently resolves the binary" vs. "seed unconditionally everywhere"). It shipped as an open question, not a closed one.

## Recommendation

When a CONTEXT.md "Deferred To Planning" item concerns a fact only the current machine/session can verify (an installed binary, a local path, a personal credential), planning must either (a) close it with a concrete check before cutting the cell, or (b) explicitly scope the resulting decision to acknowledge the limitation ("this default assumes the tool is present on every machine this config seeds onto — true today, not guaranteed on a future reinstall"). A deferred item left open in CONTEXT.md and never revisited is a silently dropped decision, not a resolved one.

---

# Learning: Reopening a locked decision on factual (not preference) grounds, with concrete options, is the correct handling

**Category:** pattern
**Severity:** standard
**Tags:** [decision-reopening, gate-bypass, audit-vs-decision]
**Applicable-when:** planning-stage evidence contradicts an exploring-stage locked decision's factual premise (a flag is deprecated, a version is unsupported) — as opposed to evidence merely suggesting a different preference.

## What Happened

D6 (`codex --full-auto`, locked during exploring) was found deprecated during planning's L1 discovery. Rather than silently keeping the stale flag or silently swapping in a guessed replacement, the new evidence was surfaced back to the user with 3 concrete options (keep deprecated / modern equivalent of the old meaning / full bypass matching the other two presets); the user picked full bypass, recorded as D10 with D6 struck through (not deleted) and a rationale pointing to the supersession.

## Root Cause

This matches the project's own standing rule (`review-audit-self-decision.md`: a locked/user decision is only reopened when new evidence exists, presented as original-decision + concern + trade-off + concrete-options, then the user decides) — applied correctly under live pressure rather than left as an abstract policy.

## Recommendation

When planning-stage verification contradicts a locked decision's factual premise, do not silently resolve it either direction. Re-present the original decision, the new evidence with its source, and concrete options; log the new decision as a supersession (never an edit) so the audit trail shows both the original call and why it changed. Watch for scope creep in the reopening itself: if fixing the factual problem also invites a wider value tradeoff (as it did here — deprecation-fix became a sandbox-strength choice), surface that expansion explicitly as new option content, not folded silently into "just fixing the flag."
