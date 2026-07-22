---
date: 2026-07-22
feature: dedupe-default-config-templates
categories: [pattern, decision, failure]
severity: critical
tags: [config, installer, cell-execution, verify-quality, tooling]
---

# Learning: dedupe-default-config-templates

## Pattern: hidden self-exec CLI verb lets a shell installer defer to the binary's own canonical output

**Category:** pattern
**Severity:** standard
**Tags:** [installer, cli-design, cross-language-dedup]
**Applicable-when:** a bash/PowerShell installer script hand-writes content (JSON, env files, config) that the program it installs already owns the canonical version of.

### What Happened

`install.sh`/`install.ps1` each hand-wrote their own copy of the default `config.json`, both incomplete (missing `agent_presets`) and both drifting from the Rust-side canonical `config::default_config_json`. Rather than hand-syncing three copies again, cell -2 added a hidden, undocumented CLI branch (`--internal-print-default-config`) that prints the canonical JSON to stdout, mirroring an existing precedent in the same codebase (`--internal-merge-config`, `src/main.rs:29-32,86,168-171`). Cells -4/-5 then replaced the installers' hand-written literals with a captured call to the just-downloaded binary.

### Root Cause

Both installers already download and extract the binary before writing `config.json`, so the binary was always available at the exact point the content was needed — there was no real reason for a second, independently-maintained representation of the same data to exist. The pattern only worked because a prior feature (`self-update-merge-config`) had already established one Rust-side canonical function and an existing hidden-CLI-verb precedent to copy.

### Recommendation

When a shell installer needs data the program itself already generates canonically, and the program binary is present on disk by the time the installer needs it, add a hidden (`--internal-*`, never in `--help`) CLI verb that emits the canonical value and have the installer capture its stdout — never let the installer re-derive the value in its own language.

---

## Decision: a "pure dedup" can hide a real functional bug — check whether the duplicated content ever actually executes

**Category:** decision
**Severity:** standard
**Tags:** [scope, functional-vs-refactor]
**Applicable-when:** planning a deduplication/refactor with "no observable behavior change" assumed.

### What Happened

D6 (locked during exploring): on every platform, the installer pre-writes `config.json` *before* the binary's first run — so `ensure_config`'s own `agent_presets`-seeding logic never actually fired on any fresh install done through an installer, only via a later `herdr-go update`. What looked like DRY cleanup was closing a real, silent gap in every fresh install to date.

### Root Cause

Two code paths produced "the same" output today only by coincidence of both being kept in sync by hand up to that point (and one, `install.ps1`, was already NOT in sync). Nobody had asked "does path A's logic even get a chance to run, given path B always executes first?"

### Recommendation

Before scoping a duplication-removal as behavior-neutral, trace execution order between the duplicated call sites, not just their output. If one path always pre-empts another (e.g. an installer writes a file before the program's own first-run logic ever sees it missing), the "unreachable" path's guarantees may already be silently false — the dedup fix may be a bug fix in disguise, and should be evidenced/labelled as such (`behavior_change: true` on the relevant cells), not filed as a pure refactor.

---

## Failure (CRITICAL): this installation's `bee-gather`/`bee-extract` agent types are read-only — never dispatch them for cell execution

**Category:** failure
**Severity:** critical
**Tags:** [bee-swarming, subagent-type, tooling]
**Applicable-when:** dispatching ANY standard/high-risk swarming wave in this installation (Claude Code runtime), or the single-execution-worker path for tiny/small.

### What Happened

The orchestrator's first wave (cells -1, -6) was dispatched per `bee-swarming`'s literal spawn-type guidance — `subagent_type: "bee-gather"` for the generation tier, `"bee-extract"` for extraction. Both workers immediately returned `[BLOCKED]`: in this installation, `bee-gather` and `bee-extract` are rendered with only `Read, Grep, Glob` tools (the Delegation contract's read-only I/O-offload workers) — no `Bash`/`Write`/`Edit`, so they cannot reserve files, run `cargo build`, or write code at all. Both cells had to be re-dispatched under `subagent_type: "claude"` (full tools) before any work happened. This cost one full round-trip per blocked worker and was invisible in the cell traces afterward (see meta-finding below) — only a ~2m38s `acquired_at`→`claimed_at` timing gap on the affected wave hints at it having happened.

### Root Cause

`bee-swarming`'s Runtime Spawn Mechanics table instructs "tier-matched pinned type... `bee-gather` for generation, `bee-extract` for extraction" for cell EXECUTION dispatch. That guidance conflates two distinct worker classes bee's own `AGENTS.md` (critical rule 13) actually documents separately: read-only I/O-offload **gather workers** (Delegation contract — find/digest, never write) versus **execution workers** (AO14 — implement, verify, cap a cell). In this installation, the rendered `bee-gather`/`bee-extract` agents are scoped strictly to the gather role and carry no write tools; the swarming skill's spawn-type table does not hold for them as execution dispatch targets.

### Recommendation

Before the first wave of any swarming session (Claude Code runtime), confirm whether `bee-gather`/`bee-extract` in the current installation actually carry write tools (check `.claude/agents/bee-gather.md`/`bee-extract.md`, or just try one cell first). If they are read-only (as here), dispatch generation/extraction-tier EXECUTION cells under a full-tool agent type (`"claude"` or `"general-purpose"`) instead, reserving `bee-gather`/`bee-extract`/`bee-review` exclusively for actual read-only Delegation-contract gathers (plan-checker, cell-reviewer, orient reads) — which is where they worked correctly throughout this feature.

---

## Failure (CRITICAL): a literal `$` inside a `grep` verify pattern can false-negative under this session's interactive-shell grep wrapper

**Category:** failure
**Severity:** critical
**Tags:** [grep, shell-environment, verify-quality]
**Applicable-when:** authoring or independently re-running ANY verify/goal-check command that greps for a literal `$` character (common in PowerShell literals like `UTF8Encoding($false)`, or any pattern containing a dollar sign not meant as a regex anchor).

### What Happened

Cell -5's verify command included `grep -q 'UTF8Encoding($false)' install.ps1`. The cell's own worker (Jerry) hit a false negative running this interactively and worked around it via `sh -c "..."`, recording the workaround in `verification_evidence`. During the orchestrator's own independent goal-check re-run, the SAME false negative recurred — the workaround Jerry had already found was buried in a prose evidence field, not surfaced anywhere the goal-check step would read before re-running the command. Diagnosis (via `grep -qF`, `grep -n`, and `sh -c`, all agreeing the file was correct) confirmed: this session's interactive zsh has a `grep` shell function that routes to `ugrep -G`, and that wrapper mishandles a literal `$` embedded mid-pattern (treating it as a special character rather than literal), while plain `/usr/bin/grep`, `grep -F`, and any non-interactive `sh -c` invocation all match correctly.

### Root Cause

An environment-local convenience wrapper (`grep` → `ugrep -G` via the Claude Code CLI's scout/smart-grep integration, defined only for interactive zsh sessions) has different BRE semantics for `$` than POSIX `grep`, and nothing marks a verify command as needing to bypass it.

### Recommendation

Any verify/goal-check command containing a literal `$` in a `grep` pattern should either escape it (`\$`) or use `grep -F` (fixed-string) to avoid depending on which shell/grep-flavor re-runs it. When re-running a cell's exact `verify_command` for goal-check purposes and it fails unexpectedly, try `sh -c "<command>"` before concluding the underlying change is wrong — a bypass of interactive-shell wrappers is a legitimate first diagnostic step, not a last resort.

---

## Failure (non-critical): PowerShell `Out-File -Encoding utf8` silently BOMs a config file `serde_json` can't parse

**Category:** failure
**Severity:** standard
**Tags:** [install.ps1, powershell, bom, windows]
**Applicable-when:** any future `install.ps1` edit that writes a file consumed by this program's config/JSON parser.

### What Happened

The drafted cell -5 action initially suggested `Out-File -Encoding utf8` to write the captured default-config content. Cell review (cold-pickup pass, before any cell was claimed) caught that `Out-File -Encoding utf8` writes a UTF-8 byte-order mark under Windows PowerShell 5.1, and `serde_json` does not skip a BOM — silently breaking `config.json` parsing at first service start, unprovable in this dev environment (no `pwsh` available; the real end-to-end proof is `release.yml`'s CI-only Windows install smoke job). Fixed to `[System.IO.File]::WriteAllText($ConfigFile, $defaultConfig, (New-Object System.Text.UTF8Encoding($false)))`, matching a no-BOM pattern the file already used elsewhere (its token-file handling).

### Root Cause

"Capture stdout, write it to a file" reads as encoding-neutral in most languages but is not in Windows PowerShell 5.1, where several common file-write cmdlets default to a BOM-prefixed encoding.

### Recommendation

Any `install.ps1` cell that writes a file consumed by this program must use `[System.IO.File]::WriteAllText(...,(New-Object System.Text.UTF8Encoding($false)))`, never `Out-File`/`Set-Content` with a bare `-Encoding utf8`, and should say so explicitly in the cell's action and a verify-command ban on `Out-File` for that write.

---

## Failure (non-critical, self-critique): the vacuous-test-filter verify pattern recurred despite an existing same-day critical-patterns entry

**Category:** failure
**Severity:** standard
**Tags:** [process, cell-authoring, critical-patterns-adherence]
**Applicable-when:** authoring any cell whose verify includes `cargo test --quiet <filter>` (or an equivalent test-name filter in any test runner).

### What Happened

`critical-patterns.md` already carried "[20260722] A test-runner filter passing is not proof of work done" (from the same day, sibling feature `self-update-merge-config`) at the time this feature's cells -2/-3 were drafted — yet both cells' original verify commands repeated exactly the pattern that entry warns against (`cargo test --quiet <filter>` alone, vacuously passing against the unmodified repo). Caught by plan-checker (WARNING 1) and cell-review (CRITICAL) during validating, before any cell was claimed — the existing rule worked as designed as a *catch*, but did not prevent the mistake at *authoring* time.

### Root Cause

The critical-patterns digest is read once, early in a session (onboarding/exploring), and is easy to not re-apply mechanically several phases later while drafting cell JSON by hand in bee-planning — there's no structural check at cell-creation time cross-referencing a new verify command against the digest's own documented anti-patterns.

### Recommendation

This is a process gap in bee itself, not this host project: bee-planning's cell-authoring step would benefit from a mechanical self-check (e.g. "does this verify command match `cargo test --quiet <filter>` without a preceding `grep -q 'fn <name>'`? if so, add one") rather than relying on validating's plan-checker to catch it every time. Filed as friction (see below) for `bee-grooming`/`bee-evolving` to consider, not something to build in this feature.
