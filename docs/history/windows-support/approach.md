# Approach: Windows Support

## Recommended path

Source validation pins upstream's compatibility mechanism: `interprocess::local_socket` maps the same logical endpoint path to `GenericFilePath` on Unix and `GenericNamespaced` on Windows. Implement that shared mechanism first while reusing the existing protocol client (D1, D2, D7). Then implement native per-user state and fail-closed token protection, including existing-token validation and race-free creation (D3). A dependent Windows proof cell must compile and exercise both production paths before the slice can close; packaging/release work follows only after that proof (D4, D6, D8–D10). Preserve Linux behavior at its public boundaries (D5).

## Rejected alternatives

- CLI subprocess adapter — cannot cleanly preserve direct streaming/subscription behavior and adds a second operational model.
- Gateway-owned relay/TCP bridge — violates D2 and creates an unnecessary security/lifecycle surface.
- Publish the existing ZIP as support — violates D1, D6, and D10 because no Windows runtime proof exists.
- Build service/installer now — violates D4 and D9; foreground proof comes first.

## Risk map

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| Upstream pipe/session resolution | HIGH | Public docs name the transport but not the exact default/named-session pipe strings | Live Windows beta ping/snapshot/subscribe against default and named sessions; source anchor |
| Async named-pipe behavior | HIGH | Busy/not-found/reconnect semantics differ from Unix streams | Tokio client retry and reconnect tests plus real herdr round trips |
| Native state paths | MEDIUM | Roaming/local split changes defaults only on Windows | Windows tests with isolated known-folder roots; repeat-run preservation |
| Token ACL | HIGH | A silent no-op would expose bearer credentials | Effective ACL inspection and ordinary second-user read denial |
| Supervisor restart | MEDIUM | executable suffix/process semantics differ | Real `herdr.exe` stop/restart and recovery smoke |
| Release archive | MEDIUM | Existing Windows ZIP includes Linux-only content | Extract/run smoke and archive allowlist per target |
| CI matrix | MEDIUM | Server 2022 is available; Windows 11 hosted proof is not guaranteed | Named Server 2022 job plus explicit Windows 11 evidence path |
| Linux regression | HIGH | Existing behavior and release contracts are already covered | Existing full verification and Linux installer/rename contract stay green |

## Files and order

1. Read the pinned upstream source result at `.bee/spikes/windows-support/upstream-contract.md`; live beta proof remains a dependent execution cell.
2. `src/herdr/socket.rs` or a split platform transport module; `src/herdr/mod.rs`; focused transport tests.
3. `src/config/mod.rs`, `src/main.rs`, and configuration/doctor tests for native path and endpoint reporting.
4. Windows-only token protection implementation/tests with the smallest official API surface required.
5. `.github/workflows/ci.yml`, `.github/workflows/release.yml`, Windows smoke scripts, and target-specific archive assertions.
6. README/install/usage and specs, with preview/full labels driven by actual evidence.

## Relevant learnings

- `docs/history/learnings/20260719-rename-mode-isolation-preflight.md` — route platform decisions through production seams and prove preflight/mutation behavior, not detached helpers.
- `docs/history/learnings/critical-patterns.md` — use `cargo check`/tests and the `bundle` script; do not use blocked output-directory terms in shell commands.

## Questions for validating

- What exact pipe string and session-name transform does herdr preview v0.7.4 use?
- Does its named pipe support the gateway's one-request-per-connection and long-lived subscription patterns unchanged?
- Can GitHub's Windows Server runner create a second ordinary user or otherwise prove negative token access without a privileged false positive?
- Which Windows 11 execution surface is available for D8, and must that half remain an explicit pre-release gap?
