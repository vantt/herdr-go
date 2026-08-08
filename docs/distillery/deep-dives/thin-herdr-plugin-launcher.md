---
topic: thin-herdr-plugin-launcher
date: 2026-08-08
based_on: [collie@f7b692b0]
entries: [collie:config-packaging-thin-plugin-launcher-vs-supervised-daemon, collie:config-packaging-install-vs-link-build-timing, collie:tooling-config-dir-cascading-resolution]
---

# Deep-dive: thin herdr-plugin launcher (marketplace presence)

**Bottom Line:** collie ships a real, listed herdr plugin (`id = "herdr.collie"`) whose manifest is a thin shim — every `[[actions]]` shells out to a control script, and the actual long-lived bridge process runs OUTSIDE the plugin lifecycle, supervised independently (`systemd --user` on Linux, launchd on macOS, unsupervised nohup fallback elsewhere) — exactly because a plugin pane dies when the pane closes or the host restarts, which is precisely when a remote operator needs it most. herdr-gateway already has the harder half of this built: `herdr-go service <start|stop|restart|status>` (README.md, cross-platform dispatch already implemented) is functionally what collie's `collie-ctl.sh` OS-branching exists to provide. So porting this is cheaper for us than it was for them: the plugin manifest can shell straight to our own existing CLI, no new control script needed.

## Câu hỏi
Làm sao có mặt trên marketplace/plugin ecosystem của herdr mà KHÔNG phá vỡ yêu cầu "gateway phải sống độc lập, tự supervise, không phụ thuộc lifecycle của herdr" (`docs/PRD.md:162`, đã khoá)?

## Cách collie giải quyết

**Manifest (`herdr-plugin.toml`, đọc tại `origin/main`):**
```toml
id = "herdr.collie"
name = "Collie"
version = "0.25.0"
min_herdr_version = "0.7.0"
platforms = ["linux", "macos"]

[[build]]
command = ["bash", "scripts/collie-ctl.sh", "build"]
platforms = ["linux", "macos"]

[[actions]]
id = "start" | "stop" | "restart" | "uninstall" | "update" | "url" | "status" | "version"
contexts = ["workspace"]
command = ["bash", "scripts/collie-ctl.sh", "<verb>"]
```
Comment ngay trong file: *"This plugin is a thin launcher. The actual bridge runs as a systemd --user service so it outlives Herdr restarts (see ARCHITECTURE.md §3). The actions below shell out to scripts/collie-ctl.sh."*

**`[[build]]` chỉ chạy khi cài từ GitHub** (`herdr plugin install owner/repo`) — KHÔNG chạy khi dev dùng `herdr plugin link` (local checkout); link-mode build LAZY ở lần `start` đầu tiên qua `ensure_build()` (idempotent check). Một định nghĩa build duy nhất phục vụ cả 2 đường cài, không viết 2 lần.

**`collie-ctl.sh cmd_start` — dispatch theo OS thật** (đọc trực tiếp, dòng 386-438):
```
if have_systemd; then          # Linux: write unit, `systemctl --user enable --now`
elif have_launchd; then        # macOS: write LaunchAgent plist, bootstrap có RETRY 3 lần
                                # (launchd bootstrap có race window thật khi bootout chưa xong)
else start_unsupervised; fi    # fallback: nohup + pidfile, không restart-on-crash
```
`resolve_config_dir()` (dòng 23-33) tự dò `.env` theo thứ tự: env var injected bởi herdr → `herdr plugin config-dir <id>` (hỏi thẳng CLI herdr) → path quy ước của herdr (nếu có `.env` ở đó) → fallback cứng — để script chạy đúng dù được gọi từ herdr action HAY gọi tay trực tiếp, luôn đọc CÙNG 1 file config.

**Service unit tham chiếu** (`systemd/collie.service`): `StartLimitIntervalSec=0` ("never give up restarting — a phone-only operator can't run 'systemctl reset-failed'"), `NoNewPrivileges=yes`, `PrivateTmp=yes`, KHÔNG set `ProtectSystem` (ghi chú rõ lý do: write path do herdr inject, không enum được tĩnh).

**Why:** ARCHITECTURE.md §3 nói thẳng: pane/plugin lifecycle gắn với host (đóng terminal, herdr restart) — dịch vụ network-facing sống lâu, remote operator cần đúng lúc host không ai theo dõi, KHÔNG được sống trong đó. "should this be a plugin/pane or an independent daemon" quy tắc chung: cần sống qua host restart → không thể sống trong lifecycle của host đó.

## herdr-gateway hiện tại (baseline thật)

- **Không có manifest/plugin nào** — đã research trước đó (`docs/history/research/herdr-plugin-feasibility.md`), chốt hướng "Show and tell" thay vì plugin, companion-plugin bị PARK (`docs/backlog.md` PBI-029).
- **ĐÃ CÓ sẵn đúng thứ collie phải tự xây (`collie-ctl.sh`'s OS-dispatch)**: `herdr-go service start|stop|restart|status` — README.md xác nhận "auto-detects your platform's service manager (systemd user unit on Linux, launchd on macOS, Scheduled Task on Windows) and does the right thing". Đây LÀ `collie-ctl.sh cmd_start/cmd_stop/...`'s OS-branch, chỉ khác: đã build sẵn TRONG chính binary Rust, không phải bash script rời.
- Windows: herdr-gateway đã hỗ trợ (Scheduled Task) — collie chỉ có `platforms = ["linux", "macos"]`, KHÔNG có nhánh Windows nào trong `collie-ctl.sh` (không đọc thấy `Get-ScheduledTask`/PowerShell branch trong file đã đọc). Nếu port, herdr-gateway's manifest có thể khai `platforms = ["linux", "macos", "windows"]` — rộng hơn collie, đúng năng lực thật đã có.
- `install.sh`/`install.ps1` đã có logic tải+cài binary released — không cần `[[build]]` step kiểu collie (compile từ source lúc cài) vì mình ship binary sẵn.

## So sánh & trade-offs

| | collie | herdr-gateway (nếu port) |
|---|---|---|
| OS-dispatch cho start/stop/restart/status | Tự viết trong `collie-ctl.sh` (~500 dòng) | ĐÃ CÓ, trong chính binary (`herdr-go service <verb>`) |
| `[[build]]` cần thiết | Có (Bun/TS cần compile lúc cài) | Không (binary release sẵn, `install.sh` lo việc đó) |
| Config-dir cascading resolution | Cần tự viết (`resolve_config_dir`) | Có thể không cần — gateway tự biết config dir của chính nó, action chỉ gọi `herdr-go service <verb>`, không cần đi vòng qua config dir |
| Actions cung cấp | start/stop/restart/uninstall/update/url/status/version (mutate + đọc) | Cần TỰ QUYẾT — xem risk note dưới |
| Windows | Không hỗ trợ | Có thể hỗ trợ (mình đã có Scheduled Task) |

**Risk khác biệt quan trọng, đã bàn trước trong phiên làm việc này (không lặp lại chi tiết ở đây):** collie KHÔNG supervise herdr — chỉ đọc/ghi vào nó. herdr-gateway THÌ CÓ (`src/supervisor.rs`, PRD §9 khoá cứng "gateway canh herdr, không bao giờ ngược lại"). Thêm action `restart`/`stop` gọi được từ TRONG herdr's UI tạo 1 đường tương tác MỚI từ herdr vào lifecycle gateway — không phá invariant kỹ thuật, nhưng là 1 coupling không cần thiết đổi lấy tiện ích nhỏ (user vốn đã có SSH/terminal). Quyết định trước đó trong phiên: ưu tiên bản CHỈ action đọc (`status`, `open`/`url`), KHÔNG có `start`/`stop`/`restart`/`uninstall` — khác với collie (họ có đủ mutate actions vì không có risk này).

## Giải pháp tổng hợp cho host

1. **`herdr-plugin.toml` tối giản, không cần control script riêng** — action `command` gọi THẲNG `herdr-go service <verb>` (binary đã cài trong PATH qua `install.sh`), không cần file bash trung gian như collie (họ cần vì daemon của họ không tự có CLI dispatch, mình đã có sẵn trong Rust binary).
2. **Actions đề xuất, chỉ đọc** (khác quyết định collie, vì lý do coupling nêu trên):
   - `status` → `herdr-go service status` (hoặc gọi thẳng `/api/health` nếu muốn tránh phụ thuộc service-manager cụ thể).
   - `url`/`open` → in ra URL web UI hiện tại (bind address + port từ config), zero side-effect.
3. **KHÔNG `[[build]]`** — binary release sẵn, không cần compile lúc `herdr plugin install`.
4. **`min_herdr_version`**: dùng version herdr tương ứng protocol 16 đã pin cứng trong `src/herdr/wire.rs` (`HERDR_PROTOCOL: u32 = 16`) — tái dùng kỷ luật pin-exact sẵn có, không tự đặt ngưỡng mới.
5. **`platforms = ["linux", "macos", "windows"]`** — rộng hơn collie, đúng năng lực herdr-gateway đã chứng minh (README's service table có đủ 3 OS).
6. **Version field của manifest**: sinh từ `Cargo.toml` lúc release (1 dòng trong `release.yml`), không viết tay song song — tránh chính vấn đề collie phải xây 3-lớp lockstep để chống (xem deep-dive `version-lockstep-release-discipline`).
7. **Không cần `resolve_config_dir` cascading** — gateway tự biết config dir của nó qua cơ chế hiện có (`src/config/`), action chỉ gọi CLI, không cần đọc `.env` qua đường vòng nào khác.

## Portable ideas

- `thin-manifest-shells-to-existing-cli` — R2 E2 F1 — action trong manifest gọi thẳng CLI sản phẩm đã có, không viết control script riêng khi CLI cross-platform đã tồn tại (khác collie — họ phải viết vì chưa có CLI dispatch sẵn, mình thì có).
- `read-only-actions-when-daemon-supervises-host` — R2 E2 F1 — nguyên tắc riêng cho host này (không phải port trực tiếp từ collie, mà LÀ điểm khác biệt cần giữ): khi service của mình supervise ngược lại chính hệ thống plugin đang host nó, chỉ expose action đọc, không expose mutate, để giữ invariant lifecycle 1 chiều.
- `build-step-only-on-hosted-install-not-link` — R1 E2 F1 — ghi chú tham khảo, KHÔNG áp dụng (mình không cần `[[build]]` — chỉ giữ lại hiểu biết cho trường hợp tương lai nếu đổi sang model build-from-source).

## Open questions

- Muốn thật sự publish plugin này không, hay giữ nguyên quyết định park trước đó (Show and tell only)? Deep-dive này chỉ chuẩn bị SẴN thiết kế nếu quyết định làm — không tự ý coi là đã quyết đi làm.
- `min_herdr_version` chính xác tương ứng protocol 16 là bản herdr nào — cần tra lại herdr's release notes/CHANGELOG (chưa tra trong phiên này).
