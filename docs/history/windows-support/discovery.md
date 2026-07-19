# Discovery: Windows Support

## Bottom line

Upstream herdr now ships a native Windows preview and documents newline-delimited JSON over a Windows named pipe. The gateway can therefore preserve its protocol client and add only a platform-native connection layer; a CLI-wrapper rewrite would lose the long-lived subscription behavior the gateway needs.

## Evidence ledger

- **Local:** `src/herdr/socket.rs` couples connection establishment and portable newline-JSON request handling around an unconditional Unix stream. `src/herdr/wire.rs` and the `Herdr` trait are already transport-independent.
- **Local:** `src/config/mod.rs` derives all defaults from Unix/XDG conventions and only enforces token mode on Unix. `.github/workflows/release.yml` creates a Windows ZIP but includes Linux-only installer/service material; `.github/workflows/ci.yml` has no Windows job.
- **Local:** Tokio resolves to 1.53.0 with its network feature enabled; the existing embedded UI, TCP HTTP listener, process command seam, and bundled database are portable.
- **Upstream/Docs:** herdr's current Windows beta uses native ConPTY and ships through the preview channel. Its socket API states that Windows uses a named pipe and that raw clients own the platform-native local socket form. v0.7.4 includes a Windows named-pipe client reliability fix. Sources: <https://herdr.dev/docs/windows-beta/>, <https://herdr.dev/docs/socket-api/>, <https://herdr.dev/docs/install/>.
- **Docs:** Tokio's installed major version exposes `tokio::net::windows::named_pipe::ClientOptions`; its documented client loop handles `ERROR_PIPE_BUSY` with bounded retry. Source: <https://docs.rs/tokio/latest/tokio/net/windows/named_pipe/struct.NamedPipeClient.html>.
- **Docs:** `dirs` 6 uses the Windows Known Folder API and maps configuration to roaming app data and local data to the local per-user area. Source: <https://docs.rs/dirs/latest/dirs/>.
- **Docs:** Windows objects inherit inheritable ACLs from their container; the creator owns new objects. Machine-specific app data belongs under LocalAppData. Sources: <https://learn.microsoft.com/en-us/windows/security/identity-protection/access-control/access-control>, <https://learn.microsoft.com/en-us/windows/apps/develop/windows-app-restore>.
- **Docs:** GitHub warns that `windows-latest` is a moving alias; a named image such as `windows-2022` avoids surprise migration. Source: <https://github.com/actions/runner-images>.

## Candidate approaches

### A. Reuse protocol + add platform transport — recommended

Keep request IDs, framing, response parsing, subscription semantics, and the `Herdr` interface. Compile Unix connection code only on Unix and named-pipe connection code only on Windows. Resolve endpoint overrides as platform-native values and validate the upstream default pipe name during feasibility work.

Why it wins: it reuses almost all production behavior and matches upstream's documented raw-client contract. The only unresolved fact is exact default/named-session pipe resolution.

### B. Invoke herdr CLI wrappers for every operation — rejected

The upstream docs recommend CLI wrappers for simple portable automation, but this gateway needs direct request/response control and long-lived event or observation behavior. Replacing the adapter with subprocess calls would add parsing, process, latency, and cancellation semantics while still requiring a separate solution for subscriptions.

### C. Gateway-owned TCP or relay bridge — rejected

This contradicts D2 and duplicates an endpoint that upstream already supplies. It creates a new security boundary and lifecycle component without a product need.

### D. Demo-only Windows artifact — fallback only

This remains the D7 stop branch if the exact upstream endpoint cannot be connected reliably on the pinned Windows matrix. Current upstream evidence makes it a fallback, not the recommended outcome.

## Version caveats and proof obligations

- **Inference:** the gateway can derive the exact default pipe name from upstream source or a live `HERDR_SOCKET_PATH`/session contract, but the public socket page does not spell out the pipe string. Validation must prove default and named-session resolution against the selected preview build.
- **Inference:** creating the token under the user's AppData subtree will inherit a non-world-readable ACL. Validation must inspect effective ACLs and prove that an ordinary second local user lacks read access; inheritance alone is not sufficient evidence.
- **Inference:** the current process supervisor can restart `herdr.exe` without shell mediation. Validation must run the real command on Windows and observe server recovery.
- **Inference:** GitHub-hosted Windows Server 2022 proves the server half of D8. Windows 11 needs a separate real machine/self-hosted runner or an explicitly recorded preview gap; it cannot be inferred from Server 2022.

## Recommendation ladder

1. **Reuse:** keep `Herdr`, wire types, framing/parser, fake backend, HTTP/UI, and supervisor seam.
2. **Built-in:** use Tokio's Windows named-pipe client and Rust/platform directory APIs rather than inventing I/O primitives.
3. **Adapt upstream:** mirror herdr's documented endpoint/session resolution once verified from its current source/live beta.
4. **Build:** add only the thin platform adapter, native-state mapping, target-specific packaging, and Windows proof workflow missing locally.
