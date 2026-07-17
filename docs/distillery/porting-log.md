# Porting Log

Nguồn sự thật duy nhất về trạng thái porting. Tính năng bị từ chối vẫn ghi lại kèm lý do.

- Status: `candidate` → `planned` → `in-progress` → `ported` / `adapted` / `rejected`
- Score `R# E# F#` chấm một lần lúc tạo candidate; xếp hạng bằng `distill.mjs rank`.
- Local = tên trong project ta sau khi port (bắt buộc khi ported/adapted); tra hai chiều bằng `distill.mjs map`.

| Feature | Nguồn | Status | Score | Local | Đích (path) | Commit | Ghi chú / Lý do |
|---|---|---|---|---|---|---|---|
| ship-agent-skill-file | ntm+herdr | candidate | R3 E3 F1 | | | | Hội tụ độc lập: cả hai ship `SKILL.md` để agent tự lái tool; herdr thêm `HERDR_ENV` hard-gate. Convention rẻ, tác động toàn platform. |
| agents-md-binding-contract | ntm+herdr | candidate | R3 E3 F1 | | | | Cả hai coi AGENTS.md là hợp đồng ràng buộc; ntm=RULE 0 precedence, herdr=audience-scope gating. Nên hợp nhất cả hai ý. |
| human-vs-robot-surface-split | ntm+herdr | candidate | R3 E3 F1 | | | | Nguyên tắc: tách bề mặt cho người (TUI) vs máy/agent (robot/socket). Cả hai độc lập áp dụng → convention nền tảng. |
| schema-from-source-of-truth | ntm+herdr | candidate | R2 E3 F2 | | | | Sinh schema/help từ code (herdr schemars→JSON Schema; ntm "declared once, derived everywhere") để chống drift. |
| wait-primitives | ntm+herdr | candidate | R2 E3 F2 | | | | Tách `wait output` vs `wait agent-status`; match-ngay-hoặc-block. Lõi scripting cho agent điều phối. |
| agent-state-detection | ntm+herdr | candidate | R3 E3 F3 | | | | Screen-scrape idle/working/blocked là lõi. So kè: ntm velocity+regex+hysteresis vs herdr manifest+hook-authority → xem deep-dive. |
| doc-code-drift-check | ntm+herdr | candidate | R2 E3 F1 | | | | Verify docs khớp type thật bằng script CI (herdr config_reference_check; ntm drift/parity audit). Rẻ, hội tụ. |
| real-process-no-mock-testing | ntm+herdr | candidate | R2 E3 F2 | | | | Test bằng binary/PTY thật + dọn orphan (herdr watchdog) / e2e live-agent (ntm). Triết lý test hội tụ. |
| multi-channel-install-checksum | ntm+herdr | candidate | R2 E3 F1 | | | | curl+checksum-verify + self-update verify; herdr thêm stable/preview channels. |
| git-worktree-isolation | ntm+herdr | candidate | R2 E3 F2 | | | | Worktree per-agent như coordination-alt; herdr thêm provenance/branch-slug, ntm thêm `worktrees merge`. |
| session-snapshot-restore | ntm+herdr | candidate | R2 E3 F2 | | | | Versioned snapshot khôi phục cấu trúc (không process). Nền cho detach/restart. |
| layered-quality-gates | ntm+herdr | candidate | R2 E3 F1 | | | | fast-local → full pre-release; herdr windows cross-lint từ Linux. |
| destructive-command-policy-engine | ntm | candidate | R3 E2 F3 | | | | 3-tier pattern + PATH-shadow wrapper + PreToolUse hook; sinh từ sự cố thật (dogfood). Safety cross-cutting nhưng nặng. |
| machine-operator-loop-contract | ntm | candidate | R3 E2 F3 | | | | "LLM là driver, tool là nervous system" + 8-step loop + lane Must/Must-NOT. Định hướng thiết kế robot surface. |
| hook-authority-arbitration | herdr | candidate | R2 E1 F2 | | | | Mỗi pane đúng 1 status authority; hook lifecycle đè screen-manifest. Giải bài "2 nguồn sự thật". |
| native-agent-session-restore | herdr | candidate | R2 E1 F2 | | | | Relaunch chính lệnh resume của agent (`claude --resume <id>`), 14 agent + version gate, typed ref. |
| session-snapshot-bootstrap-rpc | herdr | candidate | R2 E1 F2 | | | | snapshot 1 lần rồi `events.subscribe` giữ local cache — hợp đồng client rõ ràng. |
| docs-staging-mirror-release-gate | herdr | candidate | R2 E1 F1 | | | | `docs/next/` staging + `release-docs-check` diff cứng + translation parity. |
| redaction-engine | ntm | candidate | R2 E1 F2 | | | | `[REDACTED:CAT:hash8]` deterministic-non-reversible, RE2 budget, 5-state disclosure matrix. |
| checkpoint-cadence-guidance | ntm | candidate | R2 E1 F1 | | | | Checkpoint như nhịp thường quy (trước edit rủi ro, sau verify xanh, trước handoff), không chỉ khi thảm hoạ. |
| single-writer-ownership | herdr | candidate | R1 E2 F1 | | | | 1 writer/terminal, `--takeover` để evict; observer read-only không giới hạn. |
| prefix-free-chord-safety | herdr | candidate | R1 E1 F1 | | | | Bảng chord an toàn test qua 10 terminal emulator — `ctrl+alt` là họ modifier gần như trống. |
| command-palette-prompt-library | ntm | candidate | R1 E1 F1 | | | | `command_palette.md` thư viện prompt phân loại, surface qua picker. |
| sidebar-token-row-system | herdr | candidate | R1 E1 F2 | | | | Row = mảng token, tự collapse khi thiếu value; custom `$token` qua report-metadata. |
| discovery-supersedes-written-spec | airemote | candidate | R3 E2 F1 | | | | Milestone-0 spike chạy tool thật trước khi code, ghi delta vs spec (`DISCOVERY.md`), discovery thắng vĩnh viễn khi mâu thuẫn. Dogfood-proven: bắt được 1 lỗi HIGH (Codex trust-prompt đọc thành `idle`) đáng lẽ làm design sai hoàn toàn. |
| locked-decision-id-log | airemote | candidate | R2 E2 F1 | | | | Mọi quyết định thiết kế (không riêng security) có `D<n>` ổn định, cross-reference qua nhiều spec file thay vì lặp prose — chống drift "tại sao lại làm thế này". 66+ decision đã chứng minh scale được. |
| default-deny-readiness-shape-match | airemote | candidate | R3 E2 F2 | | | | Match "sẵn sàng" theo HÌNH DẠNG màn hình (composer glyph + không enumerate + không activity), không enumerate menu đã biết — vì enumerate-menu từng để lọt 1 incident thật (chọn nhầm menu chạy pipe-to-shell). Áp dụng được cho bất kỳ ai lái agent qua terminal. |
| send-confirm-submit-pattern | airemote | candidate | R2 E2 F1 | | | | Gõ chữ ≠ submit khi lái agent qua herdr: phải type → poll màn hình xác nhận đã lên → mới gửi Enter. Gửi Enter sớm mất submission (quan sát 2 lần thật). |
| path-allowlist-ordered-validation | airemote | candidate | R3 E2 F2 | | | | Thuật toán 7 bước có THỨ TỰ: deny-list trước VÀ sau resolve symlink, containment theo component (không phải text-prefix), fail-closed mặc định. Component tái dùng được cho bất kỳ service nào cấp quyền filesystem cho agent từ input không tin cậy. |
| toctou-safe-create-point-of-use | airemote | candidate | R2 E2 F2 | | | | No-follow create + re-validate toàn bộ path + teardown-nếu-escape — đóng TOCTOU vì herdr's own worktree-create được xác minh là follow symlink. Liên quan trực tiếp nếu host project tạo thư mục qua herdr worktree API. |
| unreachable-vs-empty-recovery-evidence | airemote | candidate | R2 E2 F2 | | | | Phân biệt rạch ròi "không gọi được runtime" vs "runtime trả về rỗng" khi quyết định orphan — chỉ snapshot THÀNH CÔNG mới được kết luận "đã mất". Nguyên tắc chung cho mọi recovery logic dựa trên snapshot. |
| single-verify-target-is-the-done-bar | airemote | candidate | R2 E2 F1 | | | | 1 target `verify` duy nhất làm done-bar (Rust: build+clippy+test). Gap (coverage/lint) được nêu tên + hoãn có chủ đích, không giấu. §11 bước 0. |
| strict-config-decoding | airemote | candidate | R3 E2 F1 | | | | Key lạ/gõ sai = lỗi startup nêu tên key (serde deny_unknown_fields); gom mọi lỗi validation báo 1 lần. "Security setting rơi về default im lặng là setting operator tưởng có mà không có." §11 bước 0. |
| empty-allowlist-fail-closed | airemote | candidate | R3 E2 F1 | | | | Allowed-roots rỗng = lỗi startup cứng, không bao giờ hiểu là "cho tất". §11 bước 0. |
| bot-token-env-only | airemote | candidate | R3 E2 F1 | | | | Token (Telegram bot + GitHub) không bao giờ là field config — env/secret file mode-600, đúng 1 reader, không log/serialize. Strict decoding biến việc nhét token vào settings thành lỗi. PRD §7. |
| session-isolation-dedicated-session | airemote | candidate | R3 E2 F1 | | | | Mọi invocation herdr prepend `--session` tường minh, enforce nhiều lớp — `HERDR_SESSION` env bị herdr 0.7.3 lờ im lặng (phát hiện sống). PRD §8. |
| protocol-version-compatibility-check | airemote | candidate | R3 E2 F1 | | | | Pin wire protocol number (16) check lúc startup, không tin version string; mismatch = lỗi có kiểu. PRD §8. |
| no-event-subscription-status-polling | airemote | candidate | R3 E2 F1 | | | | Poll 500ms là default, `Subscribe` trả "not implemented" cho tới khi M0 verify events đạt. Kèm de-dup: `agent_status_changed` từng fire 2 lần/1ms → chống double-notify. PRD §5.4/§8/§10. |
| restart-recovery-single-snapshot-evidence | airemote | candidate | R2 E2 F2 | | | | Reconcile khởi động bằng ĐÚNG 1 snapshot quyết định tất cả, retry backoff 30s, read-only tuyệt đối. Cặp với unreachable-vs-empty (đã có row riêng). PRD §9. |
| agent-outlives-service | airemote | candidate | R2 E2 F1 | | | | Bất biến: gateway chết/restart không đụng agent trong herdr; chứng minh bằng test kill-9 thật (pane diff byte-identical sau re-attach). §11 bước 1. |
| single-service-durable-record-lock | airemote | candidate | R2 E2 F1 | | | | flock 1 gateway instance duy nhất — 2 supervisor cùng canh herdr sẽ đá nhau khi restart; crash tự nhả lock. §11 bước 1. |
| auth-gate-fail-closed-silent | airemote | candidate | R3 E2 F1 | | | | TG: chỉ 1 group + sender allowlist, sai → drop im lặng tuyệt đối + audit content-free; button auth theo người bấm. Web: silent-404 cùng nguyên tắc. PRD §7 (quyết định Telegram-B). |
| no-inbound-webhook-poll-only | airemote | candidate | R2 E2 F1 | | | | Kênh TG chỉ long-poll, `DeleteWebhook` phòng thủ startup. Trước bị loại nhầm (tưởng ngược posture web) — chỉ áp cho bot, web server vẫn là web. PRD §7. |
| durable-poll-offset-resume | airemote | candidate | R3 E2 F2 | | | | Offset caller-owned, thứ tự act → persist → fetch: restart resume đúng-1-lần, không nuốt lệnh, không trả lời đôi. PRD §9, §11 bước 2. |
| deferred-obligation-as-durable-event | airemote | candidate | R2 E2 F2 | | | | Notify = durable event + `delivered_at`, at-least-once (send trước ghi sau) — nghĩa vụ mà restart quên được thì không phải nghĩa vụ. PRD §5.4. |
| migrations-all-or-nothing-repeatable | airemote | candidate | R2 E2 F1 | | | | Store nhỏ TG (offset+delivered): mọi migration trong 1 transaction, embed trong binary; nửa-migrate không bao giờ là state quan sát được. PRD §9. |
| never-store-output-or-credentials | airemote | candidate | R2 E2 F1 | | | | Store không bao giờ chứa output terminal (chỉ hash) hay credential. PRD §9. |
| server-side-payload-revalidation | airemote | candidate | R3 E2 F1 | | | | Payload button/form (TG lẫn web) không bao giờ là giá trị đã tin — re-validate tươi tại thời điểm build request; "sender được auth, payload chỉ chọn handler" (D66). PRD §5.2. |
| new-wizard-guided-task-creation | airemote | candidate | R2 E2 F2 | | | | `/new` 3 bước button + text mô tả; `/new` giữa chừng thay wizard dở, không mở song song; cap check trước button đầu (bounded race chấp nhận có ghi chú). PRD §5.2, §11 bước 5. |
| slug-sanitization | airemote | candidate | R3 E2 F1 | | | | Allowlist charset byte-level (không decode) cho tên repo/branch từ chat; rỗng = error, không bao giờ fallback raw. PRD §7. |
| hard-deny-list | airemote | candidate | R3 E2 F1 | | | | Deny cứng không config được (`/etc`, `~/.ssh`, `~/.aws`…); boundary refuse start nếu allowed-root nằm trong deny. Tự nhận known-incomplete — nới là quyết định operator. §11 bước 5. |
| branch-slug-last-gate-refusal | airemote | candidate | R2 E2 F1 | | | | Gate cuối độc lập caller trước khi giá trị rời process: branch phải khớp đúng output sanitizer, không thì herdr tạo `../../../.ssh` verbatim. §11 bước 5. |
| worktree-double-validated-target-directory | airemote | candidate | R2 E2 F2 | | | | Validate 2 lần: no-follow create của mình + validate lại directory herdr checkout thật sự trả về (herdr follow symlink — verify sống). §11 bước 5. |
| cap-tightening-only-permission-ceiling | airemote | candidate | R2 E2 F1 | | | | Convention 1 chiều xuyên codebase: override/mode chỉ được siết, không được nới — project read-only không thể bị chat ép write kể cả payload giả. §11 bước 5. |
| sandbox-safe-launch-flags-only | airemote | candidate | R2 E2 F1 | | | | Launch agent chỉ chọn được 2 mức an toàn; giá trị disable-sandbox không tồn tại trong code + refuse lúc load config (refused-not-discouraged). §11 bước 5. |
| adversarial-and-mutation-testing | airemote | candidate | R2 E2 F2 | | | | Module security: suite adversarial độc lập + mutation test chống implementation biết-là-sai (HasPrefix, resolve-sau-containment) — chứng minh bắt đúng bug class. §11 bước 5. |
| topic-first-fail-closed-task-creation | airemote | candidate | R1 E2 F2 | | | | CÂN NHẮC: topic-per-task (mint id → topic → record, fail = dọn sạch, không orphan topic). Chỉ lấy nếu muốn notify/chat theo thread từng project. |
| ansi-stripping-scanner | airemote | candidate | R2 E2 F1 | | | | Scanner thuần (không regex — OSC có 2 terminator hợp lệ) strip ANSI; nền cho confirm-landed + readiness Tier 1. §11 bước 6. |
| per-agent-launch-adapters | airemote | candidate | R2 E2 F2 | | | | 1 adapter/agent (Codex `›`, Claude `❯` trong box) giấu khác biệt launch/glyph/cancel sau interface chung. §11 bước 6. |
| directory-trust-flag-preemption | airemote | candidate | R2 E2 F1 | | | | Pre-clear Codex trust prompt qua `-c` flag: trust theo repo root (không phải worktree), path phải nằm value-half (D44 — cơ chế cũ D23/D31 ship mà không chạy). §11 bước 6. |
| keystroke-state-guard | airemote | candidate | R2 E2 F1 | | | | Refuse mọi keystroke khi state ≠ ready — "sequencing is not a guard; state is"; bảo vệ session needs_attention khỏi mọi caller tương lai. §11 bước 6. |
| two-step-cancellation | airemote | candidate | R2 E2 F1 | | | | `stop` = interrupt → grace → interrupt 2, không bao giờ force-kill; refuse trên session không ready. PRD §5.5. |
| secret-redaction | airemote | candidate | R2 E2 F2 | | | | 1 redactor duy nhất hệ thống, idempotent, tuned chống over-redact, trước mọi output Tier 1. So kè với redaction-engine (ntm, row trên) khi implement — chọn 1. PRD §7. |
| composer-testdata-ground-truth | airemote | candidate | R2 E2 F2 | | | | Fixture màn hình thật + completeness assertion + 10 synthetic unseen-shape chứng minh default-deny; gap không backfill giả. §11 bước 6. |
| short-handle-resolution | airemote | candidate | R1 E2 F1 | | | | `/status <prefix>` resolve như short-SHA; mơ hồ → liệt kê hỏi lại, không bao giờ đoán. §11 bước 6. |
