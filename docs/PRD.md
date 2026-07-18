# PRD — herdr-go

- **Status:** Draft
- **Ngày:** 2026-07-17
- **Binary/CLI:** `herdctl`
- **Stack:** Backend **Rust** · Frontend **TypeScript + xterm.js** (rationale ở mục *Tech stack & rationale*).
- **App chính (không thuộc scope build):** [`herdr`](https://github.com/ogulcancelik/herdr) — terminal multiplexer chuyên chạy/điều phối AI coding agent.
- **Nguồn tham chiếu thiết kế:** `airemote`/AgentBridge (@ `5667667`) — một remote gateway thực chiến trên herdr; bài học trích xuất trong `docs/distillery/sources/airemote.md`.

## 1. Một câu

`herdr-go` là **remote gateway + supervisor 2 kênh** cho herdr: **web mobile** để quan sát/gõ trực tiếp với AI coding agent đang chạy trong herdr (terminal live — thứ chat không render được), **Telegram** cho mọi tương tác có cấu trúc (notify, provision project mới qua wizard, verb có guard), và tự giám sát để herdr luôn sống — **không** quản lý session/agent lifecycle nội bộ (herdr đã lo).

## 2. Bối cảnh & vấn đề

herdr đã quản lý trọn vẹn session/pane/agent (nhận diện `idle`/`working`/`blocked`/`done`, rollup sidebar, worktree, snapshot-restore) và **chủ động từ chối làm web dashboard/mobile app** (docs herdr: *"You do not need a Herdr mobile app or a web dashboard"* — câu trả lời của họ là SSH + TUI responsive). herdr cũng đang dịch chuyển sang **server-owned runtime, TUI chỉ là 1 client, mọi thứ lộ qua JSON API/socket** — đúng substrate cho một gateway bên thứ 3.

Khoảng trống herdr để mở (và không có ý định lấp): giao diện **web tiện trên mobile**, tương tác agent không cần mở SSH client + gõ TUI trên màn hình hẹp, và **giữ herdr luôn sống** mà không phải tự tay quản lý process. `herdr-go` lấp đúng chỗ đó, tận dụng substrate herdr cung cấp, không xây lại thứ herdr đã có.

## 3. Mục tiêu / Không mục tiêu

**Mục tiêu:**
- **Web-UI mobile-first**: agent switcher → chọn agent → mở màn hình terminal ngang (landscape) → gõ trực tiếp với agent đó (live, full fidelity).
- **Supervisor**: herdr chết → gateway tự bật lại. Gateway là watchdog của herdr.
- **Provision project mới** qua **Telegram wizard `/new`** (kiểu airemote): tạo GitHub repo → checkout về máy → tạo workspace herdr → bật agent.
- **Notify qua Telegram**: báo khi agent `blocked`/`done` — push chat có tiếng, miễn phí, tức thì (web push/PWA hạ xuống tùy chọn sau).
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
| **herdr-go (`herdctl`)** | Web server + relay + supervisor. **Giám sát herdr** (bật lại nếu chết), làm client của herdr. |
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

**Resize khi xoay phone (hybrid — decision `82eff9f7`):** chế độ xem (observe) resize theo **observer** — chỉ đổi viewport phone, không đụng PTY thật; chế độ gõ (control) resize theo **controller** — PTY thật reflow đúng cỡ phone ("như ngồi tại máy"). Cả hai nhận `full=true` frame kế tiếp.

**Đánh đổi:** Tier 2 **bỏ qua** safety guard §8 — pipe trong suốt, không diễn giải readiness. An toàn vì **người** lái live có nhìn màn hình. Không dùng cho automation gõ mù.

### 5.2 Provision project mới (kênh: Telegram wizard `/new`)

Flow wizard kiểu airemote: `/new` → chọn qua button (project/visibility → agent Claude/Codex) → tin nhắn text tiếp theo làm mô tả/tên. `/new` giữa chừng thay thế wizard đang dở, không mở song song. **Mọi payload button re-validate server-side tại thời điểm build request** — button chỉ chọn handler, không phải giá trị đã tin được (airemote D66).

| Bước | Việc | Công cụ |
|---|---|---|
| 1 | Tạo GitHub repo (nhập tên/visibility từ wizard) | `gh repo create` / GitHub API (cần token — §7) |
| 2 | Checkout về máy trong allowed-root | host `git clone` (no-follow-create + re-validate — §10) |
| 3 | Tạo workspace herdr trỏ vào checkout | `workspace.create --cwd <path>` |
| 4 | Bật agent (chọn Claude/Codex) | `agent start <name> --workspace <id> --cwd <path> -- <agent>` |

Cũng hỗ trợ clone repo **có sẵn** (nhập URL) thay vì tạo mới — cùng bước 2→4. Tên repo/branch từ chat đều qua slug sanitizer + path-allowlist trước khi thành path/argv.

### 5.3 Supervisor (giám sát herdr)

- Health-check herdr định kỳ (`ping`/`herdr status server`). Chết/không phản hồi → bật lại (`herdr --session <name> server` headless).
- **Lưu ý herdr behavior:** herdr server restart **mất process agent đang chạy** (chỉ detach/reattach mới giữ process; full restart chỉ khôi phục layout từ `session.json` + resume hội thoại agent qua native session restore nếu integration hỗ trợ). Supervisor bật lại herdr = lấy lại cấu trúc workspace/tab/pane + relaunch agent qua native resume, **không** cứu được work dở của agent đã chết cùng herdr. Đây là giới hạn của herdr, không phải gateway.
- Nâng cấp herdr (không phải crash): dùng `herdr server live-handoff` để giữ PTY sống, không stop+relaunch.

### 5.4 Notify (kênh: Telegram)

- **Telegram message** khi agent `blocked`/`done` — push chat có tiếng, miễn phí, không cần PWA/subscription.
- Delivery **at-least-once**: notify là durable event có marker `delivered_at`, send trước — ghi sau; crash giữa chừng thì resend chứ không nuốt (pattern airemote D47/D57).
- Nguồn sự kiện: **poll 500ms mặc định** (kiểu airemote) + de-dup (§8) — ship ngay. Verify PBI-001 đã đạt: nâng lên subscribe **được phép**, làm sau PBI-003 (đo `pane.agent_status_changed` với agent thật) như một optimization, không phải điều kiện ship.
- Web push / PWA: tùy chọn sau, không thuộc giai đoạn đầu.

### 5.5 Tier 1 — verb có cấu trúc (kênh: Telegram, CÓ guard)

Kênh chat = **Telegram**, cùng bot với notify/provision. Core layer vẫn channel-agnostic; các verb: `list`, `launch`, `say` (type→confirm→submit), `read` (redacted), `stop` (2-step interrupt). Giữ trọn guard §8. Không phải trục chính giai đoạn đầu nhưng core phải để mở cho nó.

## Tech stack & rationale

**Backend: Rust. Frontend: TypeScript + xterm.js.** (Frontend là split cố định — xterm.js là thư viện JS, bắt buộc render terminal ANSI trên web bất kể backend.)

Vì sao Rust cho backend (quyết định sau khi bỏ yếu tố công-sức-viết ra khỏi cân nhắc):
- App là **hạ tầng luôn-bật**: supervisor của herdr + **security boundary DUY NHẤT** giữ cửa vào socket herdr (quyền sudo) + streaming relay nhiều kết nối. Profile này tối ưu cho độ tin cậy / an toàn tại biên / footprint nhỏ / không GC-jitter trong relay — đúng thế mạnh Rust.
- "Đồng bộ herdr" **không** phải lý do share code (biên gateway↔herdr là JSON-over-socket + CLI, language-agnostic — không import herdr như crate). Lý do thật: **cùng động cơ khiến herdr chọn Rust** (runtime terminal luôn-bật, tin cậy) áp dụng y hệt cho gateway giám sát + canh cửa nó.
- Biên tái-sử-dụng thực tế là **airemote (Go)** — không copy code được sang Rust, nhưng các *pattern* đã dogfood (herdr client, send-confirm-submit, path-allowlist, redaction, slug — §7/§8) là bản thiết kế port sang Rust được.

**Fallback không hối hận:** Go — concurrency model (goroutine) hợp shape "daemon nhiều kết nối + subprocess" nhất, precedent daemon mạnh, khớp airemote. Chọn Go nếu async Rust (tokio) tỏ ra quá rườm cho phần relay. Không chọn: TypeScript cho backend (yếu ở vai trò supervisor daemon luôn-bật, dù full-stack 1 ngôn ngữ là điểm cộng).

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
- **Auth gate fail-closed (web)** — chỉ user allowlist. Session token/cookie sau đăng nhập, endpoint không auth trả về **không tiết lộ gì** (nguyên tắc im lặng airemote D46/D56 áp cho HTTP: 404/không mô tả, không xác nhận app tồn tại). Scope web giờ chỉ còn switcher + terminal.
- **Auth gate fail-closed (Telegram)** — chỉ nhận event từ đúng 1 group cấu hình VÀ sender allowlist; sai group/sender/không định danh được → drop **im lặng tuyệt đối** + 1 audit record content-free (airemote D46/D56). Button press authenticate theo chính người bấm, độc lập người mở menu. Bot **poll-only, không inbound webhook** (`DeleteWebhook` phòng thủ lúc startup).
- **Redaction** — output Tier 1 (`read`, error) qua 1 redactor trước khi ra web. *(Tier 2 raw stream KHÔNG redact — người dùng nhìn màn thật, redact ANSI vừa vô nghĩa vừa phá render.)*
- **Path-allowlist + slug** — provision (§5.2) nhận tên repo/URL từ web → validate path có thứ tự (deny-list trước+sau resolve symlink, containment theo component, fail-closed) + slug sanitizer (allowlist charset, byte-level, rỗng=error).
- **GitHub token + Telegram bot token** — đọc từ env/secret file (mode 600), mỗi token đúng 1 reader, không log, không serialize vào response/config. *(pattern `bot-token-env-only` — strict config decoding khiến việc đặt token vào settings thành lỗi vì field không tồn tại.)*
- **Transport** — web endpoint bind vào **tailnet Tailscale**, không public Internet (chốt 2026-07-17). Tailscale/WireGuard đã mã hoá transport; TLS thêm là tùy chọn. Auth gate vẫn fail-closed như trên — defense-in-depth, không coi tailnet là auth. Không expose herdr socket trực tiếp ra ngoài bao giờ.

**Ngoài tầm (giới hạn trung thực):** agent có terminal gõ được bất kỳ lệnh nào account chạy được; path-allowlist chỉ kiểm soát *chỗ gateway trỏ agent tới*, không phải *agent làm gì sau đó*. Không phải sandbox.

## 8. Ràng buộc kỹ thuật — HÀNH VI THẬT CỦA HERDR (bắt buộc tôn trọng)

Hành vi đã verify sống trên herdr 0.7.3–0.7.4 — bất kỳ ai lái herdr đều dính. **Áp cho Tier 1**; Tier 2 relay trong suốt nên phần lớn không liên quan (người dùng tự xử lý như ngồi tại máy), trừ các dòng ghi "(cả 2)"/"(Tier 2)". Các ràng buộc từ spike PBI-001/002 chi tiết ở `docs/DISCOVERY.md` — **DISCOVERY thắng khi vênh với bảng này.**

| Ràng buộc | Chi tiết | Hệ quả |
|---|---|---|
| **`--session` tường minh, LUÔN** | `HERDR_SESSION` env var bị herdr CLI **lờ đi**; chỉ `--session <name>` mới cô lập. | Mọi invocation (kể cả observe/control, supervisor start) prepend `--session`. |
| **Send ≠ Submit** (Tier 1) | `send`/`send_text` chỉ gõ chữ, không submit; phải gửi Enter riêng sau khi poll xác nhận chữ đã lên (~3s deadline). Enter sớm = mất. | `say` = type → confirm-landed → submit. |
| **`idle` ≠ ready** (Tier 1) | herdr báo `idle` cả khi ready lẫn khi kẹt prompt lạ; gửi nhầm có thể **giết agent**. | Trước `say`, xác nhận readiness bằng composer shape. |
| **Readiness = composer shape** (Tier 1) | Nhận ready theo hình dạng màn (composer glyph, không enumerate, không activity line), KHÔNG enumerate menu. | Copy `default-deny-readiness`. Màn lạ → 0 keystroke. |
| **Pin protocol number** (cả 2) | Compat **exact-match** (không phải `>=`) theo **wire protocol number** (hiện `16`), số bump theo từng release herdr — không phải version string. | Pin theo vendored herdr version; startup handshake check; mismatch = lỗi có kiểu. |
| **1 request / 1 connection** (cả 2) | Socket API chỉ trả lời request **đầu tiên** trên mỗi connection; request sau bị lờ im lặng. | Subscription = connection riêng sống lâu (chỉ chứa `events.subscribe`); mọi call khác = connection ngắn riêng (như `herdr api` CLI). Client herdr phải xây trên model này. |
| **Subscribe replay + de-dup** (watcher) | Mỗi connect replay ring buffer event gần nhất (kể cả entity đã đóng); 3 event dạng `pane.*` cần `pane_id` per-pane; error response **không mang `id`**; `pane.agent_status_changed` từng fire 2 lần/1ms. | Client bắt buộc idempotent: cursor de-dup; kỷ luật reconnect `snapshot → subscribe → catch-up-de-dup → re-snapshot`; correlate lỗi theo FIFO; per-pane re-subscribe khi pane mới xuất hiện. |
| **EOF ≠ `terminal.closed`** (Tier 2) | IPC đứt đột ngột không phát `terminal.closed`. | Relay xử lý raw EOF y hệt closed: kết thúc WS stream, reconnect + chờ frame `full=true`. |
| **`seq` ordering-only** (Tier 2) | herdr không bao giờ phát gap `seq`; client chậm bị coalesce về state mới nhất, không bị skip. | Không cần gap-recovery/backfill; reconnect = reset xterm.js bằng frame `full=true` đầu tiên. |

## 9. Vòng đời & supervision

**Mô hình: systemd canh gateway; gateway canh herdr.** (Đây là lựa chọn đề xuất — giải quyết mong muốn "khỏi tự cài nhiều thứ" ở mức tối thiểu tuyệt đối.)

- **Tại sao không né được systemd hoàn toàn:** một process đã chết **không thể tự bật lại** — luôn cần thứ bên ngoài. Gateway bật lại được herdr, nhưng nếu gateway chết (hoặc máy reboot) thì không có gì bật gateway. Né systemd thường dẫn tới tự chế lại systemd tệ hơn (cron @reboot + vòng `while true`).
- **Giải pháp:** **đúng 1 systemd user unit cho gateway** (`Restart=always`, `WantedBy=default.target` + lingering). Gateway lo toàn bộ lifecycle herdr (health-check + restart — §5.3). Kết quả: cài 1 unit duy nhất, tự hồi phục qua cả crash lẫn reboot, herdr do gateway quản.
- **Gateway KHÔNG phải herdr plugin** — herdr không có autostart-plugin-khi-boot, và plugin sẽ khoá vòng đời gateway vào herdr (ngược hoàn toàn: ở đây gateway phải *trên* herdr để giám sát nó).

**State:** gần như stateless về session. herdr là nguồn sự thật.
- Routing/deep-link map: stamp vào herdr qua `pane report-metadata --token` (§6). Gateway restart → đọc `session.snapshot` dựng lại.
- State thật sự durable duy nhất của gateway: allowlist (web user + Telegram group/sender) + auth credential + GitHub/bot token (config/secret file), cộng 1 store nhỏ cho kênh Telegram: **poll offset** (restart resume đúng-1-lần, act→persist→fetch — airemote D53/D57) + marker `delivered_at` của notify. Store không bao giờ chứa output terminal hay credential.
- Reconcile khi gateway khởi động: đảm bảo herdr sống (bật nếu chưa) → snapshot → dựng lại switcher + routing.

## 10. Rủi ro & phải verify sống trước khi khoá kiến trúc

- ~~[CAO] `events.subscribe` có đáng tin không?~~ — **ĐÃ VERIFY 2026-07-17 (PBI-001, sống trên herdr 0.7.4):** subscribe **đáng tin** — watcher được phép nâng poll → subscribe ("not implemented" là stance của airemote, không phải herdr). Kèm 4 ràng buộc mới đã nhập vào §8 (1-request/connection, replay ring buffer + de-dup bắt buộc, per-pane `pane_id`, error không id). Notify vẫn ship trên poll (Telegram-B); chuyển subscribe chỉ sau khi PBI-003 đo `pane.agent_status_changed` với agent thật. Bằng chứng: `.bee/spikes/pbi-001-events-subscribe/`, `docs/DISCOVERY.md`.
- **[TB↓ từ CAO] Tier 2 observe/control — protocol layer ĐÃ VERIFY (PBI-002): GO.** Frame schema/single-writer/`--takeover`/full-frame resync đúng như §5.1; `seq` không bao giờ gap (miễn backfill); floor latency ~16ms server tick. **Còn lại:** số latency/UX thật trên cellular/tailnet — đo bằng live test khi build bước 4. Resize khi xoay phone **đã chốt hybrid** (decision `82eff9f7`): xem = observer resize (viewport riêng), gõ = controller resize (PTY thật reflow). Bằng chứng: `.bee/spikes/tier2-observe-control/`, `docs/DISCOVERY.md`.
- **[TB] herdr server restart mất process agent** (§5.3) — supervisor bật lại herdr chỉ khôi phục layout + native resume, không cứu work dở. Cần xác nhận native-resume hoạt động cho Claude/Codex như mong đợi.
- **[TB] herdr tự restart** làm gì với client đang observe/control + subscribe? airemote ghi nhận **chưa probe** — cần test reconnect.
- **[THẤP] Provision TOCTOU** khi checkout tạo dir rồi mới validate (herdr worktree-create follow symlink) — no-follow-create + re-validate (copy `safe-create-point-of-use`).
- **[THẤP] Tier 2 không stream kitty-graphics** — observer bỏ qua message `Graphics`; ảnh inline không hiện trên web. Không vấn đề với coding-agent TUI (toàn text/ANSI).

## 11. Lộ trình (2 kênh: web terminal + Telegram cấu trúc)

1. **Supervisor + herdr client nền** — gateway đảm bảo herdr sống, kết nối, pin protocol. 1 systemd unit.
2. **Telegram bot nền + Notify** — auth gate im lặng, poll-offset resume, notify `blocked`/`done` qua poll 500ms + de-dup, delivery at-least-once. **Giá trị ngay, không đợi verify events.**
3. **Web auth + agent switcher (read-only)** — đăng nhập, list agent + trạng thái từ snapshot. Chưa tương tác.
4. **Tier 2 landscape terminal** — xterm.js + WebSocket relay sang observe/control; gõ trực tiếp với agent. **Trục chính.**
5. **Provision qua Telegram wizard `/new`** — tạo GitHub repo + checkout + workspace + agent; path-allowlist + slug + token + server-side revalidation.
6. **Tier 1 verbs qua Telegram** — `say`/`read`/`stop` với trọn guard §8; nudge có kiểm soát, automation.
7. **(Tùy chọn sau) Web push/PWA** — nếu cần notify không qua Telegram.

## Open questions (cần operator chốt)

1. ~~Provision "tạo GitHub"~~ — **ĐÃ CHỐT 2026-07-17:** hỗ trợ cả tạo-mới lẫn clone-URL; repo private mặc định.
2. ~~Web auth cơ chế gì~~ — **ĐÃ CHỐT 2026-07-17:** token tĩnh + cookie phiên, trước mắt 1 operator; nâng cấp sau nếu thêm người.
3. ~~Notify web push làm ngay hay để sau?~~ — **ĐÃ CHỐT 2026-07-17 (quyết định Telegram-B):** notify qua Telegram, ship sớm ở bước 2 trên poll; web push thành tùy chọn cuối lộ trình.
4. ~~Phạm vi expose web~~ — **ĐÃ CHỐT 2026-07-17:** chỉ trong tailnet **Tailscale**, không public Internet (§7 Transport).

*(Hết open question — kiến trúc module xem quyết định "hexagonal tại nút thắt" trong decision log.)*

---

*File tự chứa, thiết kế cho repo `herdr-go`. Chi tiết bài học airemote: `docs/distillery/sources/airemote.md` trong project research/multiplexer.*
