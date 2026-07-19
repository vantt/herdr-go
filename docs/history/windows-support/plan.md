---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: high-risk
---

# Plan: Windows Support

Mode: `high-risk` — 6 risk flags: audit/security, external systems, public contracts, cross-platform, existing covered behavior, weak proof around the area.

Why this is the least workflow that protects the work: endpoint interoperability, credential ACLs, release claims, and two operating-system families all require independent feasibility evidence before source execution.

## Requirements (from CONTEXT.md)

- D1/D6/D10: publish and document only the Windows evidence tier actually proven; preview and full support remain distinct.
- D2/D7: use upstream herdr's real named-pipe contract or stop at preview; never invent a production relay.
- D3: map config to roaming data, database/runtime state to local data, and protect the token from another ordinary user.
- D4/D9: ship a target-specific foreground ZIP first; service, auto-start, installer, and dev deploy are separate future work.
- D5: preserve all Linux defaults, migration, installer, systemd, and release behavior.
- D8: target x86_64 MSVC on Windows 11 and Windows Server 2022 with named-version evidence.

## Discovery

See `discovery.md`. Upstream now provides a Windows preview over named pipes; Tokio and Known Folder support cover the missing local primitives, while exact pipe resolution and ACL proof remain validation obligations.

## Approach

See `approach.md` for the platform-transport reuse path, rejected alternatives, risk map, and ordered file surface.

## Shape

Feature outcome: a target-specific Windows executable and ZIP whose foreground gateway behavior, native state, token protection, web UI, and real-herdr control path are proven on the pinned support matrix, without changing Linux behavior.

| Epic | Capability / risk area | Why it exists | Slices | Proof needed |
|---|---|---|---|---|
| E1 | Upstream feasibility | The exact pipe/session/restart contract is the only fact that can invalidate full support | Live-contract spike | Real v0.7.4+ Windows beta evidence |
| E2 | Platform runtime | Production currently cannot compile or connect on Windows | Transport, endpoint config, supervisor | Named-pipe unit + real-herdr E2E |
| E3 | Native state security | Current defaults and token protection are Unix-only | Known folders, first-run/repeat-run, ACL | Path matrix + negative second-user read |
| E4 | Artifact truthfulness | Current Windows ZIP is Linux-shaped and untested | CI matrix, archive allowlist, extract/run | Pinned runner logs and archive smoke |
| E5 | Documentation/spec | Support labels must match evidence exactly | preview/full docs and state-layer sync | Contract assertions + scribing |

### Slice queue

1. **Feasibility: real herdr Windows contract** — source compatibility is YES and pinned in `.bee/spikes/windows-support/upstream-contract.md`; live request/subscription/restart remains part of the blocking proof cell.
2. **Portable runtime + native state** — implement upstream-compatible local-socket transport, native directories, endpoint/doctor behavior, and race-free token ACL protection.
3. **Blocking Windows proof** — compile both Windows production branches and run named-pipe, state, ACL second-user, and real-herdr smoke; the runtime slice cannot close without it.
4. **Packaging** — after runtime proof, add target-specific ZIP and extract/run release evidence.
5. **Truthful docs and closeout** — publish only the evidence tier reached and sync specs.

Current slice to prepare after Gate 2: validation-led feasibility of the real upstream Windows beta contract. No production source edit is allowed until the proof is positive and Gate 3 is approved.

## Current slice

**Slice:** Windows foreground runtime foundation plus its blocking Windows proof.

**Entry state:** upstream v0.7.4+ documents a Windows named pipe, while the gateway is Unix-stream-only and Windows state/token semantics are undefined.

**Exit state:** source evidence has pinned upstream's cross-platform local-socket conversion; the gateway uses it, selects native Windows state, protects both new and existing tokens without an exposure window, reports redacted diagnostics, and passes a real Windows proof covering compile, endpoint behavior, ACL denial, first-run roots, and real-herdr restart. Release publication and support claims remain unchanged.

**Bounded files:** `Cargo.toml`, `Cargo.lock`, `src/herdr/mod.rs`, `src/herdr/socket.rs`, `src/config/mod.rs`, `src/main.rs`, `src/supervisor.rs`, `.github/workflows/ci.yml`, and `scripts/windows-runtime-smoke.ps1`.

**Verification:** formatting, Rust tests and clippy on Linux; a pinned Windows Server 2022 job compiles/tests both Windows branches and runs the production smoke script with a real herdr beta and a second ordinary user. Windows 11 remains required before a general support claim, but does not block implementation evidence on Server 2022.

## Cells

- `windows-support-1` — platform-native herdr transport and endpoint resolution.
- `windows-support-2` — native Windows state roots, token protection, and doctor reporting (serialized after cell 1 because both wire `src/main.rs`).
- `windows-support-3` — blocking Server 2022 compile/runtime/security proof for both implementation cells.

## Test matrix

| Dimension | Probe |
|---|---|
| User types | First-run operator, repeat operator, and a second ordinary local user attempting token access |
| Input extremes | Paths with spaces/Unicode; empty/invalid pipe override; named session with safe boundary characters |
| Timing | Pipe busy/not-yet-created; herdr restart during request and during subscription |
| Scale | Empty session, one pane, multiple workspaces/panes; subscription remains bounded |
| State transitions | first run → repeat run; herdr down → up; default → named session; preview → full evidence tier |
| Environment | Windows 11, Server 2022, Linux; PowerShell/cmd-neutral foreground invocation; no Bash assumptions in Windows archive |
| Error cascades | Missing herdr, stale pipe, access denied, malformed response, restart failure; doctor reports actionable failure without crashing |
| Authorization | Bearer token remains owner-readable only; HTTP auth behavior is unchanged |
| Data integrity | Existing config/data survive repeat start; no Unix/XDG migration on Windows; Linux migration unchanged |
| Integration | v0.7.4+ named-pipe framing, default/named session resolution, snapshot/observe/reply/restart contract |
| Compliance | Token and sensitive paths do not leak through logs or archive contents |
| Business logic | Preview label cannot unlock general support docs; full label requires every D10 proof |

## Out of scope

- Windows Service, auto-start, PowerShell installer, uninstall/upgrade, and dev deployment (PBI-012).
- Windows ARM64 or 32-bit targets.
- A gateway-owned compatibility relay or TCP endpoint.
- Claiming Windows 11 from Server 2022 evidence or full support from demo-only evidence.
