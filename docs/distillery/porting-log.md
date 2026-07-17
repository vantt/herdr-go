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
