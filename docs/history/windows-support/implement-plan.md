---
artifact_contract: bee-implement-plan/v1
feature: windows-support
lane: high-risk
status: Approved
updated: 2026-07-19
sources: [CONTEXT.md, discovery.md, approach.md, plan.md, reports/validation-runtime-foundation.md]
decisions: [D1, D2, D3, D4, D5, D6, D7, D8, D9, D10]
---

# Implementation Plan: Windows Support

> Human-layer projection of the truth artifacts. Truth lives in CONTEXT.md
> (decisions), plan.md + cells (work), and the validating report (evidence).

## 1. Goal

Make herdr-gateway build, package, configure, start, and operate truthfully on supported Windows systems. A fake-backed artifact is labeled only as a Windows preview; full Windows support requires live real-herdr evidence on the pinned matrix (D1, D6, D8, D10).

**Success looks like**

- A target-specific x86_64 MSVC ZIP runs foreground on Windows 11 and Windows Server 2022 without Linux-only content (D4, D8, D9).
- Configuration uses roaming application data, local state uses local application data, and another ordinary local user cannot read the token (D3).
- Full-support evidence covers real-herdr start, discovery, observe, input/reply, and restart; otherwise the artifact remains preview-only (D1, D6, D7, D10).
- Existing Linux paths, installation, service, migration, and release behavior remain unchanged except for target-specific packaging (D5).

## 2. Current State

The wire protocol, `Herdr` interface, fake backend, embedded UI, TCP HTTP server, and supervisor seam are portable. Production connection establishment is coupled to an unconditional Unix stream, while default state discovery and token-mode enforcement are Unix-specific.

The release workflow declares an MSVC target and ZIP skeleton but does not prove Windows runtime behavior and currently includes Linux installer/service material. CI runs only on Ubuntu. Upstream herdr v0.7.4 documents a Windows preview using newline-delimited JSON over a named pipe, but the exact default/named-session pipe strings and live gateway interoperability are not yet recorded here.

## 3. Scope

**In scope**

- Prove upstream named-pipe, session-resolution, framing, subscription, and restart behavior before production implementation (D1, D2, D7).
- Add Windows-native connection establishment while preserving the protocol client and Unix behavior (D2, D5).
- Use native roaming/local state and testable owner-only token access (D3).
- Produce a target-specific foreground ZIP and evidence-tiered Windows CI/release checks (D4, D6, D8–D10).
- Keep documentation and support claims aligned with actual evidence (D1, D6, D10).

**Out of scope**

- Windows Service, auto-start, PowerShell installer, uninstall/upgrade, and development deployment (D4, D9).
- ARM64 and 32-bit Windows targets.
- A gateway-owned relay or TCP compatibility endpoint (D2).
- Inferring Windows 11 support from Server 2022 or full support from demo-only evidence.

## 4. Proposed Approach

First prove the selected upstream beta contract. If positive, isolate connection establishment behind platform-specific Unix and Windows transports while retaining request IDs, framing, parsing, subscription semantics, and the `Herdr` interface. Move Windows defaults to native roaming/local data locations and make token protection verifiable. Then make CI and archive assembly target-specific, with preview and full-support labels gated by different evidence requirements. Linux public behavior remains unchanged (D1–D10).

**Why this approach** — Upstream exposes the required named-pipe endpoint, and the gateway's protocol layer is already transport-independent. A thin platform transport minimizes forks.

**Alternatives considered** — CLI subprocesses lose clean long-lived subscription behavior; a gateway relay violates D2; the existing ZIP has no runtime proof; service/installer work is premature; demo-only remains only the D7 fallback.

## 5. Technical Design

The production adapter keeps one portable request pipeline and selects only the connection primitive by operating system:

```text
gateway operation -> portable JSON framing/parser -> Unix stream or Windows named pipe -> herdr -> existing response model
```

Windows endpoint discovery follows the upstream default/named-session contract proven in validation and still permits an explicit low-level override. Startup, doctor, requests, and subscriptions use the same endpoint resolver so diagnostics cannot disagree with runtime behavior. Named-pipe busy and unavailable states become bounded, actionable connection failures rather than platform-specific crashes.

On Windows, configuration resolves beneath roaming application data while the database and runtime state resolve beneath local application data. First-run token creation uses the owner-only protection mechanism selected and proven during validation; repeat runs preserve the token. Unix/XDG discovery and migration remain on their existing compile path.

Release assembly uses an allowlist per target. The Windows ZIP contains the executable and Windows-relevant documentation, never Bash/systemd installation material. The proof workflow extracts that ZIP and exercises the packaged binary, so CI does not pass by testing a different layout than users receive.

**Security / Permissions** — The bearer token is the sensitive asset. Windows acceptance requires effective-permission evidence that another ordinary local user cannot read it; logs and archive output must not print token content. No new network bind, authentication mode, or remote trust boundary is introduced.

## 6. Affected Files

| Action | File / component | Purpose |
|---|---|---|
| Validate | `.bee/spikes/windows-support/` | Disposable proof of upstream pipe/session/framing/restart behavior |
| Modify | `src/herdr/socket.rs` | Upstream-compatible cross-platform local-socket connection establishment |
| Modify | `src/herdr/mod.rs` | Select platform transport without changing the portable interface |
| Modify | `src/config/mod.rs` | Windows roaming/local defaults and token protection |
| Modify | `src/main.rs` | Platform endpoint composition, help, and doctor behavior |
| Modify | `.github/workflows/ci.yml` | Pinned Windows runtime and security checks |
| Create | `scripts/windows-runtime-smoke.ps1` | Real herdr, restart, state, and second-user ACL proof |
| Modify | `.github/workflows/release.yml` | Target-specific archive and smoke behavior |
| Modify | `Cargo.toml`, `Cargo.lock` | Smallest justified platform directory/security dependency surface |
| Modify | README, install/usage docs, and `docs/specs/` | Evidence-tiered Windows truthfulness |

## 7. Implementation Steps

- [ ] Add platform-native herdr transport and endpoint resolution (`windows-support-1`); validation must first prove default/named-session pipe resolution, requests, subscriptions, and restart against real herdr Windows beta.
- [ ] Use native Windows state roots and owner-only token protection (`windows-support-2`, depends on `windows-support-1`).
- [ ] Prove Windows compile, real herdr runtime, and token isolation (`windows-support-3`, depends on cells 1 and 2; blocked until a Windows-capable Git-writable execution surface exists).
- [ ] Later slice: add target-specific ZIP assembly, extract/run release smoke, and Linux regression coverage.
- [ ] Later slice: publish only the evidence tier reached and synchronize documentation and specifications.

## 8. Validation Plan

**Automated**

- Existing full Linux verification.
- x86_64 MSVC compile and portable tests.
- Named-pipe busy/unavailable/reconnect, explicit override, default/named session, request, and subscription behavior.
- Roaming/local paths, first-run/repeat-run preservation, Unicode/spaced paths, and no Windows XDG migration.
- Target-specific ZIP allowlist and extracted help, doctor, foreground, HTTP/UI, and preview path.
- Effective token ACL plus negative read by another ordinary local user.
- For full support, real-herdr start, discovery, observe, input/reply, restart, and recovery.
- Recorded Server 2022 runner image plus separate Windows 11 evidence.

**Manual**

- [ ] Published labels and installation guidance match the evidence tier.
- [ ] No Linux installer/systemd/Bash material appears in the Windows ZIP.
- [ ] Doctor failures are actionable and do not reveal sensitive content.

**Cell verification** — `windows-support-1`: `cargo fmt --all --check && cargo test --quiet herdr::socket && cargo test --quiet main_migration_seam_obeys_the_cli_mode_matrix && cargo clippy --quiet -- -D warnings`; `windows-support-2`: `cargo fmt --all --check && cargo test --quiet config && cargo test --quiet main_migration_seam_obeys_the_cli_mode_matrix && cargo clippy --quiet -- -D warnings`.

**Evidence** — `reports/validation-runtime-foundation.md` records source-level upstream compatibility as proven and the Windows compile/runtime/ACL surface as unavailable. Gate 3 remains unapproved; no Windows behavior is claimed green.

## 9. Risks & Mitigation

| Risk | Impact | Mitigation |
|---|---|---|
| Exact pipe/session resolution is under-documented | High | Prove live default/named sessions and anchor upstream source |
| Named-pipe connection semantics differ | High | Bounded Tokio retry tests and real round trips |
| Native defaults alter Unix behavior | Medium | Platform-scope the change and keep Linux contracts green |
| Token ACL silently fails | High | Inspect effective ACL and prove second-user denial |
| `herdr.exe` restart differs | Medium | Real stop/restart/recovery smoke |
| Windows ZIP retains Linux content | Medium | Per-target allowlist and extract/run smoke |
| Server evidence is mistaken for desktop evidence | Medium | Require a separate Windows 11 run |

## 10. Rollback Plan

Each implementation cell is reverted as one commit in reverse dependency order. Transport rollback restores the Unix-only adapter and removes Windows support claims/artifacts; state rollback removes only Windows default-path selection and never moves or deletes user state already created. Workflow rollback disables Windows publication before reverting archive logic, preventing an older unverified ZIP from being advertised. No database schema migration is planned, so rollback does not rewrite stored records.

If a post-merge Windows proof fails, the immediate safe action is to remove the general-support label and publication path while retaining Linux behavior; a demo artifact may remain only if it still passes D6/D10 preview evidence.

## 11. Open Questions

- Exact default and named-session pipe strings for the selected herdr preview.
- Whether request-per-connection and long-lived subscription patterns work unchanged.
- The smallest official ACL API and test location that proves owner-only access.
- Whether Server 2022 CI can create a meaningful second-user denial test.
- The available Windows 11 execution surface.
- Exact smoke-script paths and per-cell verification commands after validation.
