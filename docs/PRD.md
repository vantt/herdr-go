# PRD — herdr-gateway

- **Status:** Draft
- **Ngày:** 2026-07-17
- **Binary/CLI:** `herdctl`
- **App chính (không thuộc scope build):** [`herdr`](https://github.com/ogulcancelik/herdr) — terminal multiplexer chuyên chạy/điều phối AI coding agent.
- **Nguồn tham chiếu thiết kế:** `airemote`/AgentBridge (@ `5667667`) — một remote gateway thực chiến trên herdr; bài học trích xuất trong `docs/distillery/sources/airemote.md`.

## 1. Một câu

`herdr-gateway` là **web-first remote gateway + supervisor** cho herdr: cho phép từ điện thoại (web mobile) quan sát/tương tác trực tiếp với AI coding agent đang chạy trong herdr, provision project mới, và tự giám sát để herdr luôn sống — **không** quản lý session/agent lifecycle nội bộ (herdr đã lo).

## 2. Bối cảnh & vấn đề

herdr đã quản lý trọn vẹn session/pane/agent (nhận diện `idle`/`working`/`blocked`/`done`, rollup sidebar, worktree, snapshot-restore) và **chủ động từ chối làm web dashboard/mobile app** (docs herdr: *"You do not need a Herdr mobile app or a web dashboard"* — câu trả lời của họ là SSH + TUI responsive). herdr cũng đang dịch chuyển sang **server-owned runtime, TUI chỉ là 1 client, mọi thứ lộ qua JSON API/socket** — đúng substrate cho một gateway bên thứ 3.

Khoảng trống herdr để mở (và không có ý định lấp): giao diện **web tiện trên mobile**, tương tác agent không cần mở SSH client + gõ TUI trên màn hình hẹp, và **giữ herdr luôn sống** mà không phải tự tay quản lý process. `herdr-gateway` lấp đúng chỗ đó, tận dụng substrate herdr cung cấp, không xây lại thứ herdr đã có.

## 3. Mục tiêu / Không mục tiêu

**Mục tiêu:**
- **Web-UI mobile-first**: agent switcher → chọn agent → mở màn hình terminal ngang (landscape) → gõ trực tiếp với agent đó (live, full fidelity).
- **Supervisor**: herdr chết → gateway tự bật lại. Gateway là watchdog của herdr.
- **Provision project mới**: tạo GitHub repo → checkout về máy → tạo workspace herdr → bật agent.
- **Notify**: báo (có tiếng, qua web push/PWA) khi agent `blocked`/`done`.
- Gateway gần như **stateless về session** — herdr là nguồn sự thật duy nhất.

**Không mục tiêu (herdr đã làm / cố tình không lặp):**
- Quản lý lifecycle session/agent (create/track/recover/orphan/cleanup của herdr).
- Sổ sách DB session song song với herdr.
- Làm **window manager** cho herdr (split/swap/zoom/move pane tuỳ ý, quản layout/tab như bản TUI). Gateway *trỏ tới* 1 pane/agent rồi quan sát/tương tác, không tái tạo layout engine. (Xem full màn + gõ live 1 pane thì **có** trong scope — đó là "attach 1 terminal", khác "lái toàn bộ layout".)
- Sandbox agent (ngoài tầm — §7).

## 4. Actors & quan hệ process

| Actor | Vai trò |
|---|---|
| **Operator (bạn)** | Chủ máy dev, đã allowlist. Người duy nhất được điều khiển. Dùng web trên phone. |
| **herdr-gateway (`herdctl`)** | Web server + relay + supervisor. **Giám sát herdr** (bật lại nếu chết), làm client của herdr. |
| **herdr server** | Runtime sở hữu mọi terminal + agent. Gateway là client + supervisor của nó. |
| **Coding agent** | Claude Code / Codex chạy trong pane do herdr quản lý. |
| **Kẻ lạ** | Ai chạm được web endpoint nhưng không allowlist → drop im lặng (§7). |

Chuỗi supervision: **systemd → canh gateway → canh herdr → chạy agent** (chi tiết §9).

## 5. Yêu cầu chức năng

### 5.1 Web-UI mobile-first (kênh chính — Tier 2 raw terminal)

Đây là trục chính của app. Dùng giao thức stream terminal hạng nhất của herdr (chính là cơ chế `agent attach`/`terminal attach` của thin client).

**Agent switcher → landscape terminal flow:**
1. Màn danh sách (portrait): liệt kê mọi agent qua các workspace/tab, kèm trạng thái (`working`/`blocked`/`done`/`idle`) — đọc từ `session.snapshot`.
2. Tap 1 agent → chuyển sang **màn terminal ngang (landscape)** full-screen của pane đó.
3. Web render terminal bằng **xterm.js**; gõ trực tiếp → đẩy vào agent live.

**Wire (đã verify trong source herdr):**

| Hướng | Lệnh herdr | Format |
|---|---|---|
| **Output ra** | `terminal session observe <target> [--cols N] [--rows N]` (read-only) | newline-delimited JSON, mỗi dòng 1 frame: `{"type":"terminal.frame","seq":<u64>,"encoding":"ansi","width":<u16>,"height":<u16>,"full":<bool>,"bytes":"<base64 ANSI>"}`. `full=true`=redraw toàn màn, `false`=diff. Kết thúc `{"type":"terminal.closed","reason":...}`. |
| **Input vào** | `terminal session control <target> [--takeover] [--cols N] [--rows N]` (writable) | JSON: `terminal.input` (`text` HOẶC `bytes` base64 — gồm mũi tên/Enter/Ctrl-C/Tab), `terminal.resize`, `terminal.scroll`, `terminal.release`. |

Câu hỏi interactive của Claude/Codex (menu arrow-select, y/n, nhập text) render thành ANSI → hiện trên web; trả lời bằng cách gửi keystroke tương ứng. Gateway = **ống relay trong suốt**.

**Single-writer (từ source):** `observe` read-only → nhiều tab/thiết bị xem cùng lúc OK. `control` writable → herdr ép **1 writer/terminal**, `--takeover` để giành. Reconnect → server gửi `full=true` frame đầu rồi diff theo `seq` (web phải apply đúng thứ tự).

**Đánh đổi:** Tier 2 **bỏ qua** safety guard §8 — pipe trong suốt, không diễn giải readiness. An toàn vì **người** lái live có nhìn màn hình. Không dùng cho automation gõ mù.

### 5.2 Provision project mới

| Bước | Việc | Công cụ |
|---|---|---|
| 1 | Tạo GitHub repo (nhập tên/visibility từ web) | `gh repo create` / GitHub API (cần token — §7) |
| 2 | Checkout về máy trong allowed-root | host `git clone` (no-follow-create + re-validate — §10) |
| 3 | Tạo workspace herdr trỏ vào checkout | `workspace.create --cwd <path>` |
| 4 | Bật agent (chọn Claude/Codex) | `agent start <name> --workspace <id> --cwd <path> -- <agent>` |

Cũng hỗ trợ clone repo **có sẵn** (nhập URL) thay vì tạo mới — cùng bước 2→4. Tên repo/branch từ web đều qua slug sanitizer + path-allowlist trước khi thành path/argv.

### 5.3 Supervisor (giám sát herdr)

- Health-check herdr định kỳ (`ping`/`herdr status server`). Chết/không phản hồi → bật lại (`herdr --session <name> server` headless).
- **Lưu ý herdr behavior:** herdr server restart **mất process agent đang chạy** (chỉ detach/reattach mới giữ process; full restart chỉ khôi phục layout từ `session.json` + resume hội thoại agent qua native session restore nếu integration hỗ trợ). Supervisor bật lại herdr = lấy lại cấu trúc workspace/tab/pane + relaunch agent qua native resume, **không** cứu được work dở của agent đã chết cùng herdr. Đây là giới hạn của herdr, không phải gateway.
- Nâng cấp herdr (không phải crash): dùng `herdr server live-handoff` để giữ PTY sống, không stop+relaunch.

### 5.4 Notify

- Web push / PWA notification (có tiếng) khi agent `blocked`/`done`. (Web-first nên **không** miễn phí như push của chat — cần PWA + web push subscription.)
- Nguồn sự kiện: xem §6 Event watcher + rủi ro §10.

### 5.5 Tier 1 — verb có cấu trúc (kênh phụ, CÓ guard)

Cho automation / chat (nếu thêm sau) / thao tác cần kiểm soát. Core layer channel-agnostic; các verb: `list`, `launch`, `say` (type→confirm→submit), `read` (redacted), `stop` (2-step interrupt). Giữ trọn guard §8. Không phải trục chính giai đoạn đầu nhưng core phải để mở cho nó.

## 6. Addressing scheme — cách gọi tên agent để truy xuất

herdr phân cấp `workspace → tab → pane → agent`; ID **opaque, KHÔNG được tự ghép, phải đọc lại từ snapshot** (hợp đồng herdr). Scheme 3 lớp:

| Lớp | Dùng cho | Giá trị |
|---|---|---|
| **Tên hiển thị** | switcher, cho người đọc | `<workspace-label> › <tab-label> › <agent-name/kind>` — quét nhanh, không dùng làm khoá. |
| **Khoá địa chỉ nội bộ** | gateway trỏ observe/control | `pane_id`/agent-target **opaque của herdr**, luôn resolve tươi từ `session.snapshot` mỗi lần, không cache/ghép chuỗi. |
| **Deep-link ổn định** (tùy chọn) | bookmark 1 agent trên phone | gateway stamp `gw_id=<uuid>` vào pane qua `pane report-metadata --token`; web deep-link theo `gw_id` → resolve ra `pane_id` sống. Giữ pattern "nhét state vào herdr, gateway stateless". |

Nguyên tắc: **hiển thị theo path người-đọc-được, địa chỉ theo opaque ID herdr, không bao giờ tự dựng ID từ số thứ tự.**

## 7. Mô hình bảo mật (threat model)

**Gateway là security boundary DUY NHẤT** giữa web (Internet/LAN) và herdr — herdr socket **không có auth**, ai chạm được là điều khiển được tất cả. Mọi thứ chạy dưới login account operator (có sudo), không sandbox.

**Kiểm soát bắt buộc:**
- **Auth gate fail-closed** — chỉ user allowlist. Vì là web (không phải chat riêng tư), cần cơ chế auth thật: session token/cookie sau đăng nhập, endpoint không auth trả về **không tiết lộ gì** (giống nguyên tắc im lặng của airemote D46/D56, áp cho HTTP: 404/không mô tả, không xác nhận app tồn tại).
- **Redaction** — output Tier 1 (`read`, error) qua 1 redactor trước khi ra web. *(Tier 2 raw stream KHÔNG redact — người dùng nhìn màn thật, redact ANSI vừa vô nghĩa vừa phá render.)*
- **Path-allowlist + slug** — provision (§5.2) nhận tên repo/URL từ web → validate path có thứ tự (deny-list trước+sau resolve symlink, containment theo component, fail-closed) + slug sanitizer (allowlist charset, byte-level, rỗng=error).
- **GitHub token** — provision cần token tạo repo. Đọc từ env/secret file (mode 600), 1 reader duy nhất, không log, không đưa vào response. *(pattern `bot-token-env-only`.)*
- **Transport** — web endpoint phải sau TLS nếu ra khỏi localhost (reverse proxy / tunnel). Không expose herdr socket trực tiếp ra ngoài bao giờ.

**Ngoài tầm (giới hạn trung thực):** agent có terminal gõ được bất kỳ lệnh nào account chạy được; path-allowlist chỉ kiểm soát *chỗ gateway trỏ agent tới*, không phải *agent làm gì sau đó*. Không phải sandbox.

## 8. Ràng buộc kỹ thuật — HÀNH VI THẬT CỦA HERDR (bắt buộc tôn trọng)

Hành vi đã verify sống trên herdr 0.7.3 — bất kỳ ai lái herdr đều dính. **Áp cho Tier 1**; Tier 2 relay trong suốt nên phần lớn không liên quan (người dùng tự xử lý như ngồi tại máy), trừ 2 dòng cuối áp cho cả hai.

| Ràng buộc | Chi tiết | Hệ quả |
|---|---|---|
| **`--session` tường minh, LUÔN** | `HERDR_SESSION` env var bị herdr CLI **lờ đi**; chỉ `--session <name>` mới cô lập. | Mọi invocation (kể cả observe/control, supervisor start) prepend `--session`. |
| **Send ≠ Submit** (Tier 1) | `send`/`send_text` chỉ gõ chữ, không submit; phải gửi Enter riêng sau khi poll xác nhận chữ đã lên (~3s deadline). Enter sớm = mất. | `say` = type → confirm-landed → submit. |
| **`idle` ≠ ready** (Tier 1) | herdr báo `idle` cả khi ready lẫn khi kẹt prompt lạ; gửi nhầm có thể **giết agent**. | Trước `say`, xác nhận readiness bằng composer shape. |
| **Readiness = composer shape** (Tier 1) | Nhận ready theo hình dạng màn (composer glyph, không enumerate, không activity line), KHÔNG enumerate menu. | Copy `default-deny-readiness`. Màn lạ → 0 keystroke. |
| **Pin protocol number** (cả 2) | Compat theo **wire protocol number** (hiện `16`), không phải version string. | Startup check; mismatch = lỗi có kiểu. |
| **Duplicate events** (cả 2) | `pane.agent_status_changed` từng fire 2 lần cùng 1ms. | Event watcher de-dup, tránh double-notify. |

## 9. Vòng đời & supervision

**Mô hình: systemd canh gateway; gateway canh herdr.** (Đây là lựa chọn đề xuất — giải quyết mong muốn "khỏi tự cài nhiều thứ" ở mức tối thiểu tuyệt đối.)

- **Tại sao không né được systemd hoàn toàn:** một process đã chết **không thể tự bật lại** — luôn cần thứ bên ngoài. Gateway bật lại được herdr, nhưng nếu gateway chết (hoặc máy reboot) thì không có gì bật gateway. Né systemd thường dẫn tới tự chế lại systemd tệ hơn (cron @reboot + vòng `while true`).
- **Giải pháp:** **đúng 1 systemd user unit cho gateway** (`Restart=always`, `WantedBy=default.target` + lingering). Gateway lo toàn bộ lifecycle herdr (health-check + restart — §5.3). Kết quả: cài 1 unit duy nhất, tự hồi phục qua cả crash lẫn reboot, herdr do gateway quản.
- **Gateway KHÔNG phải herdr plugin** — herdr không có autostart-plugin-khi-boot, và plugin sẽ khoá vòng đời gateway vào herdr (ngược hoàn toàn: ở đây gateway phải *trên* herdr để giám sát nó).

**State:** gần như stateless về session. herdr là nguồn sự thật.
- Routing/deep-link map: stamp vào herdr qua `pane report-metadata --token` (§6). Gateway restart → đọc `session.snapshot` dựng lại.
- State thật sự durable duy nhất của gateway: allowlist user + auth credential + GitHub token (config/secret file), và web-push subscription (nếu làm PWA notify).
- Reconcile khi gateway khởi động: đảm bảo herdr sống (bật nếu chưa) → snapshot → dựng lại switcher + routing.

## 10. Rủi ro & phải verify sống trước khi khoá kiến trúc

- **[CAO] `events.subscribe` có đáng tin không?** herdr có push-event channel nhưng airemote **cố tình không dùng** (poll 500ms, `Subscribe` trả "not implemented"). Cần test: subscribe giao đủ, không miss, không drop khi reconnect? Không tin được → fallback poll. Quyết định toàn bộ Event watcher + notify.
- **[CAO] Tier 2 observe/control ổn định & đủ nhanh trên mobile?** Cần test thực: độ trễ frame qua mạng di động, reconnect (full-frame resync), `--takeover` khi cùng agent mở ở nhiều nơi, resize khi xoay ngang/dọc phone. Đây là trục chính nên rủi ro cao.
- **[TB] herdr server restart mất process agent** (§5.3) — supervisor bật lại herdr chỉ khôi phục layout + native resume, không cứu work dở. Cần xác nhận native-resume hoạt động cho Claude/Codex như mong đợi.
- **[TB] herdr tự restart** làm gì với client đang observe/control + subscribe? airemote ghi nhận **chưa probe** — cần test reconnect.
- **[THẤP] Provision TOCTOU** khi checkout tạo dir rồi mới validate (herdr worktree-create follow symlink) — no-follow-create + re-validate (copy `safe-create-point-of-use`).
- **[THẤP] Tier 2 không stream kitty-graphics** — observer bỏ qua message `Graphics`; ảnh inline không hiện trên web. Không vấn đề với coding-agent TUI (toàn text/ANSI).

## 11. Lộ trình (web-first)

1. **Supervisor + herdr client nền** — gateway đảm bảo herdr sống, kết nối, pin protocol. 1 systemd unit.
2. **Web auth + agent switcher (read-only)** — đăng nhập, list agent + trạng thái từ snapshot. Chưa tương tác.
3. **Tier 2 landscape terminal** — xterm.js + WebSocket relay sang observe/control; gõ trực tiếp với agent. **Trục chính, có giá trị ngay.**
4. **Notify** — web push/PWA blocked/done (sau khi verify §10 events).
5. **Provision** — tạo GitHub repo + checkout + workspace + agent; path-allowlist + slug + token.
6. **(Tùy chọn sau) Tier 1 verbs / chat adapter** — nudge có kiểm soát, automation.

## Open questions (cần operator chốt)

1. **Provision "tạo GitHub"**: luôn tạo repo mới trên GitHub, hay cho phép cả clone repo có sẵn (URL)? Có cần repo private mặc định? — *Giả định: hỗ trợ cả tạo-mới lẫn clone-URL; private mặc định.*
2. **Web auth cơ chế gì** cho single-operator: token tĩnh trong config + cookie phiên là đủ, hay cần login provider (OAuth GitHub…)? — *Giả định: token tĩnh + cookie phiên, đủ cho 1 operator; nâng cấp sau.*
3. **Notify web push** làm ngay ở giai đoạn đầu hay để sau khi Tier 2 chạy ổn? — *Giả định: sau (bước 4), vì cần verify events trước.*
4. **Phạm vi expose web**: chỉ trong LAN/VPN/tunnel cá nhân, hay ra Internet công khai (đổi độ gắt của auth/TLS)? — *Giả định: sau tunnel/VPN cá nhân, không public Internet giai đoạn đầu.*

---

*File tự chứa, thiết kế để move sang repo `herdr-gateway`. Chi tiết bài học airemote: `docs/distillery/sources/airemote.md` trong project research/multiplexer.*
