---
topic: adopt-decision-herdr-vs-ntm
date: 2026-07-15
based_on: [herdr@a0678a3, ntm@5840f61]
entries: [herdr:socket-api-control-surface, herdr:workspace-tab-pane-model, herdr:agent-detection-manifests, herdr:native-agent-session-restore, ntm:tmux-swarm-session-model, ntm:robot-mode-operator-loop, ntm:robot-flag-command-surface]
---

# Adopt decision: herdr vs ntm

**Bottom line:** Với 5 yêu cầu (nhiều terminal/agent · layout điều chỉnh được · hiểu+điều khiển loại agent · điều khiển từ xa · AI tự lái), **herdr là tool để adopt & dùng hằng ngày** — nó khớp cả 5 như một sản phẩm liền mạch, cài 1 lệnh, không phụ thuộc ngoài. **ntm giữ làm nguồn ý tưởng/tham chiếu**, không phải tool để chạy: mạnh về safety/planning/memory nhưng tmux-bound, kéo theo 7+ tool ngoài, và tự nhận "early dev, no users, build broken on main". Hai guide chi tiết: `how-to-use-herdr.md`, `how-to-use-ntm.md`. Nếu bạn CHỈ dùng máy local + muốn attach tương tác/điện thoại → herdr rõ ràng. Chỉ nghiêng ntm nếu bạn thực sự cần graph-triage/reservation/policy-engine và chấp nhận dựng cả hệ sinh thái của nó.

## Chấm theo 5 yêu cầu (đã kiểm chứng từ docs+code, không phải marketing)

| # | Yêu cầu | herdr | ntm | Thắng |
|---|---|---|---|---|
| 1 | Điều khiển nhiều terminal/agent | ✓✓ workspace→tab→pane, `agent list`, sidebar rollup blocked/working/done toàn project | ✓✓ tmux swarm, `spawn --cc=3 --cod=2`, mixed-CLI 1 lệnh | **Hòa** |
| 2 | Điều chỉnh layout | ✓✓✓ BSP split native, kéo chuột resize, swap/move/zoom, popup/overlay, **layout export/apply**; mọi thao tác có CLI/socket tương đương | ✗→~ **layout = tmux**, ntm không có layout engine; chỉ thêm dashboard/palette | **herdr** rõ rệt |
| 3 | Hiểu & điều khiển loại agent | ✓✓✓ 19 manifest + **hook-authority** (1 pane 1 nguồn sự thật), 14 integration install/uninstall, `agent explain` (rule+evidence), native resume 14 agent | ✓✓ velocity+regex 6-state, per-CLI classifier, **quản lý sâu**: health/restart/backoff, context-rotation, account-rotation | **herdr** cho "hiểu state sạch"; ntm sâu hơn về "quản vòng đời/tài nguyên" |
| 4 | Điều khiển từ xa | ✓✓✓ `--remote <host>` thin-client (bridge clipboard ảnh), SSH-then-herdr, **detach/reattach sống qua restart**, SSH điện thoại, quản SSH keepalive, auto-install binary remote | ✓✓ `ntm serve` REST/SSE/WS+OpenAPI — remote kiểu **API HTTP**, KHÔNG attach terminal tương tác (vẫn phải `tmux attach`/ssh) | **herdr** cho remote tương tác; ntm cho remote lập trình |
| 5 | AI tự lái tool | ✓✓✓ **1 socket** JSON-RPC tự mô tả (`api schema` sinh từ code), CLI=plugin API, SKILL.md (`HERDR_ENV` gate), `wait output`/`wait agent-status`, agent spawn/read/wait nhau | ✓✓✓ robot-mode ~143 `--robot-*`, operator-loop 8 bước, envelope+`_agent_hints`, SKILL.md; nhưng "never bare `bv`/`cass`", cần Agent Mail để phối hợp | **Hòa** (herdr gọn+tự mô tả; ntm rộng nhưng cồng kềnh hơn) |

## Yếu tố quyết định thật sự: chi phí adopt

| | herdr | ntm |
|---|---|---|
| Phụ thuộc bắt buộc | **Chỉ 1 binary** (+ agent CLI bạn muốn chạy) | **tmux** + để có giá trị đầy đủ: `br`+`bv`+Agent Mail+`cass`/`cm`+`ubs`/`rch` = 7+ process ngoài |
| Cài đặt | `brew install herdr` / curl / nix / mise | install.sh + shell-eval + `ntm deps -v`; PATH-gotcha khi chạy ngoài login shell |
| Nền tảng | Linux+macOS ổn định; Windows beta (preview) | Linux+macOS, local-first |
| Độ chín | Sản phẩm công khai: docs site, marketplace, kênh stable/preview, active | Tự nhận "no backwards compat, NO TECH DEBT", self-audit ghi "build broken on main", god-package (`robot.go` 10.7K dòng) |
| Tính năng docs-only | ít | **HyperSync, Web UI (5 bản), docs/schemas/, TOON config-wiring** — quảng cáo nhưng chưa implement |
| Bảo mật | plugin không sandbox (nêu rõ), single-writer ownership | 3 finding chưa vá trong self-audit (pane-input injection, CORS `*`, SLB self-approve) nếu mở `serve` ra ngoài localhost |

## Quyết định theo kịch bản

- **Bạn muốn 1 tool để dùng ngay, local + remote/điện thoại, agent tự lái, layout linh hoạt** → **herdr**. Khớp cả 5, chi phí adopt thấp nhất.
- **Bạn cần graph-aware triage (PageRank/critical-path), file-reservation coordination, policy-engine chặn lệnh nguy hiểm, memory/learning loop** và sẵn sàng dựng cả stack + chấp nhận rủi ro độ chín → cân nhắc **ntm** (hoặc mượn *ý tưởng* của nó, port dần — xem `porting-log.md`).
- **Không chắc** → chạy herdr trước (rẻ để thử, `brew install` + `herdr`), giữ ntm làm nguồn học. herdr không khoá bạn: socket API + CLI-as-plugin cho phép tự thêm phần thiếu.

## Nếu chọn herdr — bắt đầu từ đâu (đường tối thiểu để "dùng sâu")
1. `brew install herdr` (hoặc curl) → `herdr` trong thư mục dự án.
2. Học mental model + 5 phím: `how-to-use-herdr.md` §1, §A2.
3. Chạy vài agent trong pane (`claude`/`codex`), xem sidebar rollup — §A5.
4. Cài integration cho agent bạn dùng để có state chuẩn + native resume — §B1.
5. Remote: quyết `ssh→herdr` (đơn giản) vs `herdr --remote` (bridge clipboard) — §A3.
6. Cho AI tự lái: `npx skills add ogulcancelik/herdr` + đọc SKILL.md contract, thử `pane split --no-focus` → `pane run` → `wait agent-status` — §B3.

## Nếu vẫn muốn thử ntm — điều kiện tiên quyết
`tmux` + ≥1 agent CLI trên PATH của môi trường launch (coi chừng PATH-gotcha) → `ntm spawn <proj> --cc=2`. Bỏ qua `br`/`bv`/mail/cass lúc đầu (degrade graceful); chỉ thêm khi cần triage/coordination. Pin 1 commit vì no-backwards-compat.

## Open questions (cần bạn quyết / kiểm thực địa)
1. **Nền tảng của bạn?** Nếu có Windows trong luồng làm việc → herdr Windows còn beta (nhiều tính năng remote/handoff chưa có); ntm không hỗ trợ Windows. Cần xác nhận.
2. **Remote kiểu nào?** Attach terminal tương tác (herdr) hay điều khiển bằng API/script/máy khác (ntm serve)? Quyết định này gần như tự phân thắng bại.
3. **Có cần graph-triage/coordination/policy của ntm không**, hay agent-multiplexing thuần (herdr) là đủ? Nếu cần → nên *port ý tưởng ntm vào luồng herdr* hơn là adopt cả ntm.
4. herdr `layout export/apply` hiện chỉ có ở socket, chưa có CLI subcommand — nếu bạn cần script hoá layout, đây là khoảng trống nhỏ (guide §C).
