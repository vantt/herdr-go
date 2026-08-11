# Đo chi phí render DOM cho phương án đổi renderer (PBI-060)

**Ngày:** 2026-08-11 · **Mục đích:** chốt quyết định mở #5 của
`plans/260728-1135-terminal-ansi-render-lightweight/plan.md` ("render performance" — điểm dừng).
**Kịch bản đo:** `plans/260811-1219-terminal-render-benchmark/parse-and-count.mjs`
**Fixture:** capture thật từ herdr 0.7.4 đang chạy, **không commit** (chứa nội dung làm việc thật —
`plans/260811-1219-terminal-render-benchmark/fixtures/.gitignore` chặn toàn bộ thư mục).

## Câu hỏi cần trả lời

xterm's DOM renderer chỉ vẽ các hàng nằm trong viewport của lưới. Một `<pre>` phẳng vẽ **mọi**
dòng đang giữ. Nếu số node ở 400-1000 dòng lớn, việc đổi renderer phá đúng mục tiêu "mượt" —
đây là điều kiện dừng, không phải chi tiết tối ưu.

## Kết quả

| Fixture | Nguồn | Dòng | Bytes | Spans | Spans/dòng | p50 | p95 | max | Parse |
|---|---|---|---|---|---|---|---|---|---|
| claude-visible | pane Claude idle, `visible` | 69 | 6 986 | 195 | 2.8 | 3 | 7 | 16 | 0.05 ms |
| claude-working | pane Claude đang chạy, `visible` | 69 | 6 271 | 224 | 3.2 | 2 | 9 | 15 | 0.05 ms |
| shell-mixed | `git log --graph` + `ls --color` + `cargo tree`, `recent` | 488 | 46 647 | 2 761 | 5.7 | 6 | 12 | 16 | 0.46 ms |

Số span bị chặn trên bởi số chuỗi SGR: fixture Claude có đúng 230 SGR / 69 dòng = 3.3 — khớp với
2.8 đo được (một số SGR rơi vào buffer rỗng nên không sinh segment). Kết quả không đổi khi chạy
với cả hai mô hình flush (chỉ-khi-đổi-style, và mọi-SGR như collie), nên không nhạy với chi tiết
port parser.

### Chiếu sang kích thước gateway thật render

| | 400 dòng | 1000 dòng |
|---|---|---|
| Claude (3.2 spans/dòng) | ~1 300 spans | ~3 250 spans |
| Shell có màu (5.7 spans/dòng) | ~2 260 spans | ~5 660 spans |

Cộng container mỗi dòng và text node của mỗi span: ca xấu nhất ≈ **12 000 node DOM**.

## Phát hiện quyết định: trạng thái thường trực nhỏ hơn nhiều

Poll sống chỉ xin **80 dòng** (`src/web/screen.rs:58` — `read_pane(&pane, ReadSource::Recent, 80)`).
Mốc 400/1000 dòng chỉ xảy ra khi người dùng chủ động tải lịch sử (`RECENT_LINES_CAP = 1000`,
`src/herdr/pane_scroller.rs:22`).

Nên chi phí lặp lại mỗi 1.5 s là:

- Claude: 80 × 3.2 ≈ **256 spans**
- Shell có màu: 80 × 5.7 ≈ **456 spans**

…và chỉ khi nội dung thực sự đổi (`terminal.ts:272` dedupe bằng `text === lastText`). Render lớn
(~6 000 spans) là **một lần**, theo thao tác người dùng, không phải mỗi khung hình.

Đối chiếu: renderer hiện tại đã làm `term.reset() + term.write()` toàn bộ text mỗi lần nội dung
đổi, cộng `term.resize(cols, rows)` khi kích thước lưới đổi — không rẻ hơn.

## Kết luận

**Rủi ro hiệu năng nhỏ hơn nhiều so với lo ngại ban đầu; không còn là điểm dừng.** Mật độ span
của dữ liệu thật là 3-6 mỗi dòng, không phải hàng chục; parse ~1 ms cho 1000 dòng; và trạng thái
thường trực chỉ ~250-450 span mỗi lần vẽ lại.

Quyết định mở #5 chuyển từ "phải đo trước khi cam kết" sang "đã đo, xanh" — với hai giới hạn dưới.

## Kiểm chứng phụ: `recent-unwrapped` hoạt động đúng như cần

Chạy sống trên pane tự tạo (đã đóng sau khi đo). Một dòng logic 1587 ký tự:

- `--source recent` → bẻ thành **7 dòng vật lý ~231 ký tự** (đúng bề rộng pane).
- `--source recent-unwrapped` → trả về **nguyên 1 dòng 1587 ký tự**.

Đây là mảnh backend cần cho wrap sạch (quyết định mở #7), nay đã kiểm chứng thật chứ không chỉ
đọc tài liệu. Lưu ý: khi không dòng nào vượt bề rộng pane, hai nguồn trả về **giống hệt nhau** —
`recent-unwrapped` không phải một định dạng khác, chỉ là nối lại chỗ bị bẻ.

## Giới hạn của phép đo này

1. **Chưa đo layout/paint trong trình duyệt thật.** Máy này không có chromium hệ thống. Số ở trên
   là số node và thời gian parse, không phải thời gian layout. Với `white-space: pre-wrap` +
   `overflow-wrap: anywhere`, chi phí ngắt dòng cao hơn `pre` — cần đo riêng nếu muốn chắc ở mức
   400-1000 dòng. Ở mức 80 dòng thường trực thì không đáng kể dù thế nào.
2. **Chưa đo trên điện thoại thật.** Số tuyệt đối trên máy dev không chuyển sang phone được; chỉ
   tỷ lệ so sánh mới chuyển được.
3. **Chưa có fixture Codex/Agy** — thời điểm đo không có pane nào thuộc hai loại này đang chạy
   (`herdr pane list`: toàn bộ pane có agent đều là `claude`). Mật độ span của chúng chưa biết.
4. Fixture shell là output do phép đo tự sinh trong pane riêng, không phải scrollback tự nhiên của
   người dùng — chọn lệnh theo hướng nhiều màu (`git log --graph`, `ls --color`, `cargo tree`) để
   lấy cận trên, nên có thể **cao hơn** thực tế trung bình.

## Câu hỏi còn mở

- Có cần đo layout/paint trong trình duyệt thật trước khi bắt đầu port không, hay số node đã đủ để
  cam kết? (Khuyến nghị: đủ để cam kết; đo layout khi đã có prototype thật sẽ chính xác hơn đo
  bằng harness giả.)
- Fixture Codex/Agy lấy khi nào — chờ có pane thật, hay bỏ qua vì Claude/shell đã bao được hai
  thái cực mật độ?
- Corpus fixture dùng cho test correctness (màu khớp xterm) có được commit không? Nội dung thật
  chứa dữ liệu làm việc; cần bản sạch tự sinh hoặc bản đã rà tay trước khi vào repo.
