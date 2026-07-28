# Consult: terminal scroll handling (collie)

**Bottom Line:** Collie giải hai bài toán "cuộn" hoàn toàn khác nhau, cả hai đáng biết. (1)
`context-memory-transcript-not-terminal-scrollback` — vì agent TUI chạy alt-screen nên live pane
read KHÔNG BAO GIỜ có scrollback ring thật; "xem lại lịch sử" phải đọc file transcript riêng trên
đĩa, không phải cuộn terminal. (2) `use-auto-scroll.ts` — một hook React ~90 dòng cho "stick to
bottom + tự nhả khi user cuộn lên xem backscroll", đã có test coverage cho case dễ bị bug nhất
(content bên trong resize mà container không đổi kích thước). Hook này CHƯA từng được index vào
`sources/collie.md` (chỉ được nhắc tên ở inventory report dòng 149) — đây là lỗ thật của lưới, đã
vá bằng cách đọc thẳng file upstream trong consult này. Không cái nào thay thế trực tiếp cơ chế
nudge-buttons + escape-injection-restore hiện tại của host (PBI-058/059), nhưng kỹ thuật resize
observer ở (2) đáng học nếu host gặp bug tương tự "content grow bên trong container không resize".

## Chất liệu theo domain

### context-memory (hit)
- **collie:context-memory-transcript-not-terminal-scrollback** — alt-screen (`ESC[?1049h`) ăn hết
  scrollback ring của emulator; live-read API chỉ trả viewport hiện tại. "Xem lại" = đọc transcript
  file của agent, theo dõi continuation chain (resume/fork/promote) tới sibling mới nhất, có guard
  never-show-less. Vì-sao-liên-quan: nếu root cause của PBI-058/059 tương tự "live pane read không
  có scrollback ring thật", đáng đọc kỹ trước khi thiết kế lại. `docs/distillery/porting-log.md`
  dòng 104 đã note liên hệ này.

### ux (hit — chất liệu MỚI, extract trực tiếp từ upstream, chưa có trong index)
- **`web/src/hooks/use-auto-scroll.ts`** (upstream, chưa có slug trong `sources/collie.md`):
  - `atBottom()` dùng ngưỡng `offset` (mặc định 24px), không đòi `scrollTop === scrollHeight -
    clientHeight` tuyệt đối — tránh off-by-1 do subpixel rounding.
  - `autoScroll` là `useRef`, không phải state — giữ "intent đang follow tail" đọc lại trong resize
    handler mà không dính stale closure, tách biệt khỏi `isAtBottom` (state, chỉ dùng để show/hide
    nút "jump to bottom").
  - `useLayoutEffect` re-pin đáy TRƯỚC paint mỗi khi `dep` đổi (dep = giá trị đổi khi có content
    mới) — mở pane / đổi tab phải landing thẳng ở live tail, không flash scrollback cũ trước.
  - `ResizeObserver` quan sát CẢ container VÀ từng child, cộng `MutationObserver` để tự động
    observe node mới thêm vào. Lý do nêu thẳng trong comment: container có thể đã fix height
    (flex), riêng content bên trong (AnsiOutput) grow ra sau — chỉ observe container sẽ bị kẹt ở
    top của scrollback.
  - Khi resize xảy ra lúc đang follow: pin lại theo `scrollHeight` tuyệt đối, KHÔNG gọi lại
    `atBottom()` — vì chính cái resize (vd bàn phím ảo che khuất) đã đẩy tail ra khỏi khung nhìn,
    `atBottom()` lúc đó trả `false` sai ý định.
  - Guard `typeof ResizeObserver === "undefined"` — no-op an toàn cho jsdom/browser cũ thay vì
    throw.
  - Test file (`use-auto-scroll.test.tsx`) mock `ResizeObserver` bằng tay, cover đúng 4 case: resize
    container, resize content-only (pane mở), không yank khi user đã cuộn lên, pin khi `dep` đổi.

### harness / testing-evals (maybe — liên quan xa)
- **collie:harness-fixtures-first-detector-development** — tail-anchored detection (dialog cuộn
  khỏi màn hình phải NGỪNG match). Bài toán khác (detect dialog vs auto-follow scroll) nhưng cùng
  họ ý tưởng "nội dung cuộn khỏi viewport thay đổi hành vi" — không port trực tiếp, chỉ đáng biết
  tồn tại song song trong cùng repo.

## Trade-offs đáng cân nhắc
- Host hiện xử lý va chạm poll-loop / scroll-back load bằng escape-injection restore-to-bottom +
  nudge buttons (critical pattern 2026-07-27, 2026-07-28 — multi-page plateau, false-return-to-live
  trên trang ngắn). Collie's `use-auto-scroll` giải bài hẹp hơn: giữ pin đáy khi content lớn dần lúc
  KHÔNG chủ động xem lịch sử — chưa có khái niệm "load older / phân trang" như host đang làm, nên
  không thay thế 1-1. Nhưng cách tách "container resize" khỏi "content resize" (quan sát cả hai qua
  ResizeObserver + MutationObserver) là kỹ thuật cụ thể, transferable nếu host's `terminal.ts` gặp
  bug dạng flex-height-cố-định-nhưng-nội-dung-bên-trong-grow.
- ref-based `autoScroll` (không state) để logic không kéo re-render mỗi lần scroll fire — chỉ
  `isAtBottom` mới trigger re-render (cho nút nhảy-về-đáy). Tách rõ "giá trị điều khiển logic" khỏi
  "giá trị điều khiển UI".

## Candidate liên quan
- `porting-log.md` dòng 104: `transcript-as-history-source` (collie) — `R2 E1 F2`, candidate, đã
  note liên hệ trực tiếp PBI-058/059.
- Chưa có candidate row cho `use-auto-scroll` hook — nếu host muốn port stick-to-bottom pattern,
  đây là ứng viên mới, chưa được score/đưa vào porting-log (quyết định port thuộc về người, không
  thuộc consult này).

## Coverage ledger
| Domain | Kết quả |
|---|---|
| harness | consulted (5 entries; 1 liên quan xa) |
| skills | ruled out — collie không có entry nào dưới domain này (`domains_covered` không liệt kê) |
| hooks | ruled out — chỉ có git-hooks (versioning), không liên quan UI scroll |
| workflow | ruled out — polling/update/session, không cuộn |
| orchestration | ruled out — multi-session bridge routing, không cuộn |
| context-memory | consulted (1 entry, hit trực tiếp) |
| planning | ruled out — parking-lot doc pattern, không liên quan |
| quality-gates | ruled out — version/build gates |
| docs-style | ruled out — ADR/doc-role convention |
| tooling | ruled out — dev CLI/config resolution |
| config-packaging | ruled out — plugin/daemon lifecycle |
| repo-layout | ruled out — release tagging |
| safety | ruled out — auth/CSRF/traversal, không cuộn |
| self-improvement | ruled out — domain không xuất hiện trong `domains_covered` của collie |
| ux | consulted (4 entries index + 1 entry MỚI extract trực tiếp từ upstream: `use-auto-scroll.ts`) |
| testing-evals | consulted (2 entries; test file của auto-scroll hook đọc trực tiếp, không có slug riêng) |

## Ngoài lưới
- **`use-auto-scroll.ts` chưa có entry riêng trong `sources/collie.md`** — chỉ được nhắc tên (không
  mô tả) ở `docs/distillery/reports/distill-collie-inventory-2026-07-28.md` dòng 149. Đáng backfill
  vào domain `ux` ở lần scan/backfill tiếp theo của collie — đây là gap thật của lưới, không phải
  "ruled out". (Fixing gap này là hành động riêng, cần người quyết — chỉ báo cáo ở đây theo đúng
  nguyên tắc consult read-only.)
- Chỉ **collie** được domain-walk sâu cho feature này. `ntm`/`herdr`/`airemote` chỉ được keyword-sweep
  "scroll": `ntm` có tmux checkpoint scrollback (bài toán "lưu/khôi phục", không phải "auto-follow"),
  `herdr` có RPC `terminal.scroll` + read `--source recent`/`recent-unwrapped` (bài toán "đọc lại",
  gần với collie's transcript approach hơn là với auto-scroll hook). Nếu cần so sánh đầy đủ 4 nguồn
  cho riêng chủ đề "auto-follow scroll khi có nội dung mới", cần domain-walk riêng qua `ux` của cả
  4 index — chưa làm trong consult này.
