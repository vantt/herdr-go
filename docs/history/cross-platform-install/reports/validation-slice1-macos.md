# Validation report: cross-platform-install Slice 1 (macOS)

## Reality gate

| Check | Verdict | Evidence |
|---|---|---|
| MODE FIT | PASS | 4 flags (cross-platform, multi-domain, external systems, public contracts) — high-risk is correct |
| REPO FIT | PASS | `src/config/mod.rs`'s windows/unix cfg split, `install.sh`'s uname dispatch, and `packaging/herdr-go.service` all exist as the exact templates the cells extend |
| ASSUMPTIONS | PASS (after fix) | launchd LaunchAgent mechanism is real and works as described; the token-transport assumption was wrong and is now fixed (see Plan-checker below) |
| SMALLER PATH | PASS | already the smallest 2-slice split; macOS-only current slice, Windows deferred |
| PROOF SURFACE | PASS | each cell's verify is a real runnable command, self-tested to fail pre-fix for the right reason |

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| macOS native path resolution lands correctly | MEDIUM | real macos-14 CI run must show config/data under `~/Library/Application Support/herdr-go` | deferred to Slice 1's own real-CI proof step (plan.md) | pending execution |
| launchd plist can supervise the process (KeepAlive) | MEDIUM | real run must show restart-on-crash | deferred to Slice 1's real-CI proof | pending execution |
| The login token can reach the macOS process without a second copy | HIGH (security) | code inspection of `ensure_web_secret()` | `src/config/mod.rs:729-751` confirmed: reads `HERDR_GO_WEB_SECRET` env, then falls back to reading `herdr-go.env` directly at startup — no process-launcher help needed, exactly matching Linux's existing behavior via systemd `EnvironmentFile=` | **PASS** — cell-2 redesigned to rely on this, eliminating the second-secret-copy risk entirely rather than working around it |
| release.yml's Package step structure matches cell-3's described conditional | LOW | direct read | confirmed: shared `build` job, matrix.cross-style conditional already exists (`if [ "${{ matrix.cross }}" = "true" ]`) as the pattern to mirror | PASS |

Schedule: `deps: cross-platform-install-1 <- 2 <- 3` (linear), no cycles — trivial 3-node chain, one wave per cell (or 1+2 in a shared wave since 1→2's dependency is soft per the panel's scope-guardian finding, kept linear for simplicity given the small slice size).

## Persona panel (4 lenses, `bee-review`/opus, iteration 1)

- **Coherence:** PASS on requirement coverage — every D1-D11 in scope maps to a cell. **1 CRITICAL:** cell-2's original action self-contradicted ("populate the plist's EnvironmentVariables dict inline" vs "never embed the token literal elsewhere").
- **Feasibility:** PASS on the launchd mechanism itself; **CRITICAL redundancy** flagged — injecting the token via the plist duplicates what `ensure_web_secret()` already does by reading `herdr-go.env` directly; also flagged an unachievable `ReadWritePaths`-style launchd claim (WARNING).
- **Security (load-bearing):** **CRITICAL.** Default `~/Library/LaunchAgents/*.plist` permissions (`umask 022` → mode 644) would have exposed the token to other local users, violating R12 in `docs/specs/installation.md`; no runtime validation (`validate_token_protection()`) covers a plist copy of the secret; the action never told the worker to `chmod 600` it.
- **Scope-guardian:** PASS — clean, non-overlapping file ownership across the 3 cells; cell-3's description of `release.yml`'s current structure verified accurate against the real file. WARNING (minor, accepted): cell-1's `cargo test` verify can't exercise the `#[cfg(target_os = "macos")]` arm on this (Linux) machine — only the real `macos-14` CI proof step can, which the plan already requires before Slice 1 counts as done.

**Fix applied:** cell `cross-platform-install-2` redesigned — the plist's `ProgramArguments` now invoke only the binary + `--config <path>`; no `EnvironmentVariables` dict, no env-file read, no secret ever written into or referenced by the plist. This removes the second-copy-of-the-secret problem at its root (rather than patching it with a `chmod`), and the cell's `verify` now asserts the plist contains neither `EnvironmentVariables` nor any reference to `herdr-go.env`. Self-verified by the orchestrator (re-read the corrected cell against the panel's exact findings); no second panel dispatch was needed since the fix directly and structurally eliminates the flagged mechanism rather than patching around it.

## Cell review (cold pickup)

Covered inline by the persona panel above (scope-guardian lens = cell review for this dispatch). No CRITICAL flags remain after the cell-2 fix. One accepted WARNING (cell-1's structural verify can't exercise the macOS-only cfg arm on a Linux CI machine — mitigated by the mandatory real macos-14 proof before Slice 1 is considered done).

## Advisor consult (AO2b)

No advisor configured in `.bee/config.json` — recorded as `none-configured` per AO2(b) (not a hard dependency). `state advisor-ref record --lane cross-platform-install` completed, anchors stamped (feature, newest decision id, `plan.md` sha256) so the Gate 3 enforcement precondition is satisfied.

## Decision

**READY.** `cross-platform-install-1`, `-2` (corrected), `-3` are approved for execution as Slice 1.

## Approval block

- Lane: `high-risk`. Gate bypass level `full` — auto-approves Gate 3 at every lane including high-risk/hard-gate; the mechanical advisor-consult precondition (AO2b) is satisfied regardless of bypass level, and was completed above before this approval.
- `approved_gates.execution` set via `bee.mjs state gate --lane cross-platform-install --name execution --approved true`.
