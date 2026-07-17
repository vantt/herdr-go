# Feature Comparison Matrix

So sánh tính năng giữa các learning sources. Mỗi domain một bảng; ô có ✓ link về entry trong `sources/<name>.md#<slug>`. Ký hiệu: ✓ có | ~ một phần | ✗ không | ? chưa khảo sát. Matrix là curated view — chỉ hàng có đối chiếu đáng giá, không exhaustive.

> **Bối cảnh:** cả `ntm` (Go, control-plane TRÊN tmux) và `herdr` (Rust, tự thay tmux) đều là agent multiplexer — cùng loại với host project. Chỗ hai nguồn **hội tụ độc lập** = tín hiệu học mạnh nhất (Evidence tier 3). Chỗ **rẽ nhánh** = ứng viên deep-dive.

## harness — nhận diện & điều khiển agent

| Khả năng | ntm | herdr | Best-in-class |
|---|---|---|---|
| Screen-scrape agent state (idle/working/blocked) | ✓ [→](sources/ntm.md#agent-activity-detection) | ✓ [→](sources/herdr.md#agent-detection-manifests) | **Hội tụ** — cả hai coi đây là lõi. ntm = output-velocity + regex + hysteresis 2s + confidence score; herdr = TOML manifest trên bottom-of-buffer snapshot. Cách tiếp cận khác, kết luận giống → deep-dive. |
| State machine có "done vs seen" | ~ (GENERATING/WAITING/THINKING/ERROR/STALLED) | ✓ [→](sources/herdr.md#agent-state-machine) | **herdr** — tách `done`/`idle` chỉ bằng "đã xem chưa" (tab focus) là mô hình gọn cho UI rollup. |
| Hook-authority > screen fallback | ✗ | ✓ [→](sources/herdr.md#status-authority-arbitration) | **herdr** — mỗi pane đúng 1 status authority; hook lifecycle đè screen-manifest, tránh 2 nguồn sự thật. |
| Per-CLI classifier riêng từng agent | ✓ [→](sources/ntm.md#per-agent-cli-classifiers) | ✓ [→](sources/herdr.md#agent-detection-manifests) | **Hội tụ** — cả hai chấp nhận "mỗi CLI cần adapter riêng". |
| Health monitor + auto restart/backoff | ✓ [→](sources/ntm.md#agent-health-resilience) | ✗ | **ntm** — soft/hard restart, PID liveness, rate-limit backoff. |
| Context-window rotation + handoff | ✓ [→](sources/ntm.md#context-window-rotation) | ~ (native resume, khác cơ chế) | **ntm** cho rotation-on-fill; herdr giải quyết "khôi phục" chứ không "xoay". |

> **Xác minh thực chiến (dogfooding) từ airemote** — không phải hàng so sánh (airemote không tự làm harness, nó *dùng* herdr's) nhưng là bằng chứng độc lập, thực đo trên herdr 0.7.3 mà cả `herdr.md` lẫn matrix nên biết: `idle` status của herdr **không phân biệt** "blocked chờ prompt chưa nhận diện" với "thực sự sẵn sàng" ([→](sources/airemote.md#idle-status-ambiguity)) — gửi lệnh nhầm lúc đó **giết chết agent thật**, quan sát trực tiếp. `agent send` chỉ gõ chữ, **không submit** ([→](sources/airemote.md#sending-is-not-submitting)) — phải tự gửi Enter riêng, gửi sớm quá thì mất submission. `HERDR_SESSION` env var bị **lờ đi hoàn toàn** bởi CLI thật, chỉ `--session` mới cô lập session ([→](sources/airemote.md#session-isolation-dedicated-session)) — sự cố này từng xảy ra thật trong lúc airemote tự dogfood. Đáng để herdr's own docs/tests đối chiếu.

## orchestration — mô hình session & phối hợp

| Khả năng | ntm | herdr | Best-in-class |
|---|---|---|---|
| Mô hình session/pane | ~ [→](sources/ntm.md#tmux-swarm-session-model) (uỷ cho tmux) | ✓ [→](sources/herdr.md#workspace-tab-pane-model) (BSP riêng, opaque ID) | **Rẽ nhánh nền tảng** — ntm xây TRÊN tmux; herdr thay tmux. Quyết định kiến trúc lớn nhất của host. |
| Wait primitives (block đến khi idle/match) | ✓ [→](sources/ntm.md#agent-activity-detection) (`wait --until=idle`) | ✓ [→](sources/herdr.md#wait-primitives) | **Hội tụ** — cả hai tách "wait output" vs "wait agent-status". |
| Git worktree isolation | ✓ [→](sources/ntm.md#git-worktree-isolation) | ✓ [→](sources/herdr.md#worktree-as-workspace) | **Hội tụ** — herdr thêm provenance + branch-slug; ntm thêm `worktrees merge`. |
| Layout export/apply (declarative topology) | ✗ | ✓ [→](sources/herdr.md#layout-export-apply) | **herdr** — serialize BSP tree ra JSON, tái tạo cấu trúc (không PTY). |
| File-reservation coordination | ✓ [→](sources/ntm.md#agent-mail-coordination) | ✗ (dùng worktree isolation thay thế) | **ntm** — Agent Mail + reservation leases + simulator. |
| Active coordinator (conflict negotiation) | ✓ [→](sources/ntm.md#coordinator-active-session-management) | ✗ | **ntm** — digest/conflicts/assignment chủ động. |
| Backpressure / pressure governor | ✓ [→](sources/ntm.md#swarm-pressure-governor) | ✗ | **ntm** — resource_pressure normalize theo core count. |
| Spawn stagger chống thundering-herd | ✓ [→](sources/ntm.md#thundering-herd-prevention) | ✗ | **ntm** — 90s stagger, soft-claim protocol. |

## tooling — bề mặt máy đọc (automation surface)

| Khả năng | ntm | herdr | Best-in-class |
|---|---|---|---|
| Machine control surface first-class | ✓ [→](sources/ntm.md#robot-flag-command-surface) (~100 `--robot-*` + REST/SSE/WS) | ✓ [→](sources/herdr.md#socket-api-control-surface) (1 socket, JSON-RPC) | **Hội tụ triết lý, rẽ nhánh hình thức** — ntm maximalist (flags + REST + OpenAPI); herdr minimalist (một socket). Deep-dive. |
| Schema tự sinh từ source-of-truth | ✓ [→](sources/ntm.md#unified-robot-command-registry) ("declared once, derived everywhere") | ✓ [→](sources/herdr.md#self-describing-protocol-schema) (schemars → JSON Schema) | **Hội tụ** — cả hai chống drift bằng cách sinh schema từ code, không viết tay. Tín hiệu mạnh. |
| Output envelope + error contract chuẩn hoá | ✓ [→](sources/ntm.md#robot-output-envelope-and-error-contract) | ~ (JSON-RPC error, reason codes) | **ntm** — `_agent_hints`, `next_actions`, `safer_alternative`, exit-code 2 = unavailable. |
| Idempotency / request identity | ✓ [→](sources/ntm.md#request-identity-and-idempotency) | ✗ | **ntm** — `req_<ts>_<rand>`, poll `robot-action-status`, crash-recovery. |
| Payload budget (token/byte/latency) | ✓ [→](sources/ntm.md#ordering-pagination-and-payload-budgets) | ✗ | **ntm** — per-surface budgets + detail-level matrix + transport multiplier. |
| Bootstrap snapshot + subscribe-to-events | ~ (`--robot-snapshot` + SSE) | ✓ [→](sources/herdr.md#session-snapshot-bootstrap-rpc) | **herdr** — hợp đồng client-cache rõ: snapshot 1 lần rồi `events.subscribe`. |
| Plugin/extensibility | ✓ [→](sources/ntm.md#plugin-sdk-for-external-commands) (SDK subcommand) | ✓ [→](sources/herdr.md#cli-as-plugin-api) (CLI = API, marketplace) | **herdr** — "toàn bộ CLI là plugin API", marketplace piggyback GitHub topic. |

## context-memory — trạng thái bền & khôi phục

| Khả năng | ntm | herdr | Best-in-class |
|---|---|---|---|
| Session snapshot restore (structure, không process) | ✓ [→](sources/ntm.md#checkpoint-restore-system) | ✓ [→](sources/herdr.md#session-snapshot-restore) | **Hội tụ** — cả hai versioned snapshot; ntm giữ cả scrollback. |
| Native agent-session resume (`claude --resume`) | ✗ | ✓ [→](sources/herdr.md#native-agent-session-restore) | **herdr** — relaunch chính lệnh resume của agent, 14 agent, có version gate. |
| Live handoff (fd passing qua đổi binary) | ✗ | ✓ [→](sources/herdr.md#live-handoff) | **herdr** — chuyển PTY sống sang process mới, không serialize-rồi-relaunch. |
| Timeline replay + history search | ✓ [→](sources/ntm.md#timeline-history-audit-trail) | ✗ | **ntm** — replay session, full-text prompt search, delta snapshot. |
| SQLite projection layer (recomputable cache) | ✓ [→](sources/ntm.md#robot-durable-vs-recomputable-store) | ~ (session.json + history.json) | **ntm** — tách 4 tầng Source-Truth/Projection/Durable-Events/Watermark. |
| Cross-agent conversation search / memory learning | ✓ [→](sources/ntm.md#cass-cross-agent-search) | ✗ | **ntm** — CASS index + `cm` reflective playbook. |

## safety

> **airemote** không phải multiplexer ngang hàng ntm/herdr — nó là *consumer* chạy trên herdr, đưa agent ra remote (Telegram) nên đối mặt lớp tấn công khác (path traversal từ chat text, secret trong chat). Chỉ đối chiếu ở hàng có tương đồng thật; hàng mới (path-allowlist, sandbox-escape-hatch-refusal) là "notable absence" ở cả ntm và herdr, không phải hội tụ.

| Khả năng | ntm | herdr | airemote | Best-in-class |
|---|---|---|---|---|
| Destructive-command policy engine | ✓ [→](sources/ntm.md#destructive-command-policy-engine) | ✗ | ✗ | **ntm** — 3-tier pattern + PATH-shadow wrapper + PreToolUse hook (từ sự cố thật). |
| Two-person approval workflow | ✓ [→](sources/ntm.md#two-person-approval-workflow) | ✗ | ✗ | **ntm** (có ghi nhận bypass finding). |
| Redaction / secret scrubbing | ✓ [→](sources/ntm.md#redaction-engine) | ~ (cảnh báo pane-history chứa secret) | ✓ [→](sources/airemote.md#secret-redaction) | **Hội tụ 2/3 độc lập** — ntm dùng hash-tag category; airemote idempotent + tuned chống over-redaction (drop bare "password" khỏi prose) + đúng 1 implementation toàn hệ thống. Deep-dive đáng làm. |
| Encryption at rest | ✓ [→](sources/ntm.md#encryption-at-rest) | ✗ | ✗ | **ntm** — AES-256-GCM keyring rotation. |
| Destructive-action double-confirm | ✓ [→](sources/ntm.md#foundational-agent-safety-meta-rules) | ✓ [→](sources/herdr.md#destructive-worktree-remove-confirmation) | ✗ (dùng path-allowlist thay vì confirm) | **Hội tụ ntm+herdr** — escalation "thử an toàn → mới ép buộc". |
| Untrusted-code/plugin gate | ~ | ✓ [→](sources/herdr.md#plugin-trust-and-preview-gate) | ✗ | **herdr** — trust preview + `min_version` từ chối cứng; thẳng thắn "không sandbox". |
| Single-writer ownership | ✗ | ✓ [→](sources/herdr.md#direct-attach-single-writer-ownership) | ✗ | **herdr** — chỉ 1 writer/terminal, `--takeover` để evict. |
| Path allowlist với deny-list cứng, không config được | ✗ | ✗ | ✓ [→](sources/airemote.md#path-allowlist-validation) | **airemote** — 7-bước có thứ tự (deny trước+sau resolve symlink, containment theo component không phải text-prefix), TOCTOU đóng ở point-of-use (`safe-create-point-of-use`). Cả ntm/herdr không cần vì không expose agent ra remote-untrusted input. |
| Sandbox-disabling config bị từ chối tại load, không chỉ khuyến cáo | ✗ | ✗ | ✓ [→](sources/airemote.md#sandbox-escape-hatch-refusal) | **airemote** — `danger-full-access`/`bypassPermissions`/`sudo:true` là startup error, cố tình khó đảo ngược. |
| Auth gate im lặng tuyệt đối (không leak tồn tại) | ✗ | ✗ | ✓ [→](sources/airemote.md#auth-gate-fail-closed-silent) | **airemote** — người lạ dò kênh nhận zero signal; audit content-free. Chỉ áp dụng khi có remote control-surface (chat/API công khai). |

## planning — triage công việc

| Khả năng | ntm | herdr | Best-in-class |
|---|---|---|---|
| Graph-aware work triage | ✓ [→](sources/ntm.md#bv-graph-aware-triage) (PageRank/critical-path) | ~ [→](sources/herdr.md#issue-triage-skill) (skill light) | **ntm** áp đảo — bv engine + beads dependency graph. |
| Smart work distribution / scoring | ✓ [→](sources/ntm.md#smart-work-distribution) | ✗ | **ntm** — score 0-100, least-loaded/round-robin. |
| Ideation khi queue cạn (guarded) | ✓ [→](sources/ntm.md#ideation-roadmap-generation) | ✗ | **ntm**. |

## skills & docs-style — hợp đồng cho agent

| Khả năng | ntm | herdr | Best-in-class |
|---|---|---|---|
| Ship SKILL.md để agent tự lái tool | ✓ [→](sources/ntm.md#ntm-ships-its-own-skill-file) | ✓ [→](sources/herdr.md#shipped-agent-skill-file) | **Hội tụ mạnh** — cả hai độc lập ship skill file (herdr còn có `HERDR_ENV` hard-gate). Bài học tier-3. |
| AGENTS.md làm hợp đồng ràng buộc | ✓ [→](sources/ntm.md#agents-md-as-binding-contract) (RULE 0 override) | ✓ [→](sources/herdr.md#layered-agents-md-scoping) (audience-gated) | **Hội tụ** — herdr thêm scope-gating (maintainer/local/contributor); ntm thêm precedence RULE 0. |
| Dual-audience agent docs (operate vs teach) | ✗ | ✓ [→](sources/herdr.md#dual-audience-agent-docs) | **herdr** — SKILL.md (vận hành) vs agent-guide.md (dạy người). |
| Command-palette prompt library | ✓ [→](sources/ntm.md#command-palette-prompt-library) | ✗ | **ntm**. |
| Docs staging mirror + release gate | ~ (upgrade-log decision record) | ✓ [→](sources/herdr.md#docs-next-staging-mirror) | **herdr** — `docs/next/` + `release-docs-check` diff cứng + translation parity. |

## quality-gates & testing — kiểm chứng

| Khả năng | ntm | herdr | Best-in-class |
|---|---|---|---|
| Layered gates (fast local → full pre-release) | ✓ [→](sources/ntm.md#mandatory-compiler-and-lint-checks) | ✓ [→](sources/herdr.md#just-recipe-composed-gates) | **Hội tụ** — herdr thêm windows cross-lint từ Linux. |
| Doc/code drift check tự động | ✓ [→](sources/ntm.md#robot-cli-drift-audit) | ✓ [→](sources/herdr.md#config-reference-consistency-check) | **Hội tụ** — cả hai sinh/verify docs từ type thật (chống drift). |
| Test bằng process/binary thật, không mock | ✓ [→](sources/ntm.md#e2e-live-agent-test-suite) | ✓ [→](sources/herdr.md#real-binary-pty-integration-tests) | **Hội tụ** — herdr spawn binary thật trong PTY + watchdog dọn orphan; ntm có e2e live-agent. |
| No-real-model / determinism cho CI | ✓ [→](sources/ntm.md#deterministic-fault-injection-harness) | ~ (evidence-capture fixtures) | **ntm** — "no-real-model mandate" + fault harness. |
| Test chính script tooling của mình | ~ | ✓ [→](sources/herdr.md#python-maintenance-test-suite) | **herdr** — script release/docs/vendor có test riêng trong `just test`. |
| Bug-scanner làm pre-commit gate bắt buộc | ✓ [→](sources/ntm.md#ubs-bug-scanner-integration) | ✗ | **ntm**. |

## config-packaging

| Khả năng | ntm | herdr | Best-in-class |
|---|---|---|---|
| Config layering user + project | ✓ [→](sources/ntm.md#user-vs-project-config-layering) | ~ [→](sources/herdr.md#layered-config-with-safe-fallback) | **Hội tụ** — herdr thêm safe-fallback + live reload; ntm thêm project override. |
| Multi-channel install + checksum verify | ✓ [→](sources/ntm.md#install-script-with-checksum-verification) | ✓ [→](sources/herdr.md#multi-channel-install) | **Hội tụ** — cả hai curl+checksum; herdr thêm update channels (stable/preview). |
| Self-update có checksum + verify | ✓ [→](sources/ntm.md#upgrade-protection-checksum-verification) | ✓ [→](sources/herdr.md#update-channels) | **Hội tụ**. |
| Vendored dep + patch-lifecycle kỷ luật | ~ [→](sources/ntm.md#vendored-bubbletea-fork) (warn note) | ✓ [→](sources/herdr.md#vendored-native-dependency-with-tracked-patches) | **herdr** — patch phải khai removal-condition, `just check` verify cơ học. |

## ux

| Khả năng | ntm | herdr | Best-in-class |
|---|---|---|---|
| TUI operator surface | ✓ [→](sources/ntm.md#tui-dashboard-and-palette) (dashboard over tmux) | ✓ [→](sources/herdr.md#mouse-first-with-optional-prefix-keyboard) (là chính terminal) | **herdr** cho tương tác trực tiếp; ntm cho dashboard giám sát. |
| Human-vs-robot surface split | ✓ [→](sources/ntm.md#tui-dashboard-and-palette) | ✓ [→](sources/herdr.md#agent-vs-pane-cli-distinction) | **Hội tụ** — tách rõ bề mặt cho người vs cho máy/agent. |
| Keybinding an toàn qua nhiều terminal | ✗ | ✓ [→](sources/herdr.md#prefix-free-chord-safety-guidance) | **herdr** — bảng chord test qua 10 emulator. |
| Sidebar token-row cấu hình | ✗ | ✓ [→](sources/herdr.md#sidebar-token-row-system) | **herdr** — row = mảng token, tự collapse khi thiếu giá trị. |
| Theme auto light/dark | ✓ [→](sources/ntm.md#catppuccin-theme-system) (NO_COLOR) | ✓ [→](sources/herdr.md#theme-auto-switching) | **herdr** — follow terminal appearance; ntm — NO_COLOR + Catppuccin. |

## self-improvement

| Khả năng | ntm | herdr | Best-in-class |
|---|---|---|---|
| Reflective memory / learning loop | ✓ [→](sources/ntm.md#cass-memory-reflective-learning-loop) | ✗ | **ntm** — `cm` playbook + graded feedback tự nạp cuối phiên. |
| Multi-persona self-audit | ✓ [→](sources/ntm.md#multi-persona-self-audit-methodology) | ✗ | **ntm** — 10 reasoning-mode audit kiến trúc/bảo mật. |
| Skill tự kiểm release readiness | ✗ | ✓ [→](sources/herdr.md#pre-release-audit-skill) | **herdr** — skill maintainer diff commit↔changelog, read-only mặc định. |
</content>
