# PBI-027: Bàn phím OS che đáy terminal — Context

**Feature slug:** pbi-027-visual-viewport-keyboard
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Standard
**Domain types:** SEE

## Feature Boundary

Sửa `applySheetInset`/`openReply` trong `web/src/views/terminal.ts` để reserve đủ khoảng trống ở đáy terminal (`#term-viewport`) khi bàn phím ảo OS bật lên sau `replyText.focus()`, bằng cách lắng nghe `window.visualViewport` để biết chiều cao thật của bàn phím và cộng vào khoảng trống hiện có (`sheet.offsetHeight`). Kết thúc ở: reply-sheet trên terminal-detail, không mở rộng sang keys-pad, create-sheet, hay login.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Fix chỉ áp dụng cho `.reply-sheet` (qua `openReply`/`applySheetInset(replySheet)`). `.keys-pad` không đổi — không có input focusable nào trong keys-pad nên bàn phím OS không bao giờ bật khi nó mở. | `.keys-pad` chỉ chứa nút bấm (markup `web/src/views/terminal.ts:85-102`: `.sheet-x`, `.key-btn`, `.sheet-switch`; handlers dòng 244-251) — không có `<input>`/`<textarea>` nào để `.focus()`. |
| D2 | Graceful degradation: nếu `window.visualViewport` không tồn tại (browser cũ/không hỗ trợ), giữ nguyên hành vi hiện tại (không thêm padding cho bàn phím, không crash, không regression). | Đúng với mô tả gốc trong backlog — chỉ "một số browser" bị che, browser khác coi như không bị ảnh hưởng bởi bug này. |
| D3 | Thứ tự hiển thị đúng (từ trên xuống): đáy terminal → reply-sheet (typing form) → bàn phím OS — không phần tử nào bị phần dưới nó che khuất. Nghĩa là bản thân reply-sheet (kể cả nút Send) cũng phải nằm trọn phía trên bàn phím, không chỉ riêng nội dung terminal phía trên sheet. | Yêu cầu gốc từ backlog (`docs/backlog.md` PBI-027), người dùng đã confirm trực tiếp mô tả này — không phải suy diễn. |
| D4 | `viewport.scrollTop = viewport.scrollHeight` (ép cuộn xuống cuối) chỉ chạy 1 lần lúc mở sheet (`openReply`), như hành vi hiện tại. Khi `visualViewport` resize sau đó (bàn phím đổi chiều cao trong lúc sheet vẫn mở — vd thanh gợi ý/autocomplete bật lên), listener mới CHỈ cập nhật `paddingBottom`, KHÔNG ép cuộn lại. | User chọn trực tiếp (không phải approval-default): tránh giật cuộn ngoài ý muốn khi user đang cố tình xem lại output cũ trong lúc gõ. |

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| bàn phím OS / bàn phím ảo | Virtual keyboard hệ điều hành mobile bật lên khi focus vào `<textarea>`/`<input>`, làm co lại `window.visualViewport.height` mà không nhất thiết co `window.innerHeight` (layout viewport) — chính là gap mà code hiện tại không tính tới. |
| khoảng trống bàn phím | Phần chiều cao bổ sung cần cộng vào `viewport.style.paddingBottom`, tính từ `window.innerHeight - window.visualViewport.height` (khi API tồn tại), cộng thêm vào công thức `overlap` hiện có trong `applySheetInset`. |

## Existing Code Context

### Reusable Assets

- `web/src/views/terminal.ts:209-214` (`applySheetInset`) — công thức hiện tại: `overlap = sheet.offsetHeight - termBar.offsetHeight`, set `paddingBottom` trên `#term-viewport`, rồi ép cuộn xuống cuối. Đây là điểm sửa chính.
- `web/src/views/terminal.ts:219-224` (`openReply`) — thứ tự gọi hiện tại: `applySheetInset(replySheet)` rồi mới `replyText.focus()`. Vì `applySheetInset` chạy TRƯỚC focus, nó không thể biết trước chiều cao bàn phím ngay lần gọi đầu — bàn phím chỉ bật lên (và visualViewport resize) SAU focus.
- `web/src/views/terminal.ts:268-272` (`closeReply`) — gọi `clearSheetInset()`, nơi cần tháo listener `visualViewport` nếu có đăng ký khi mở.

### Established Patterns

- `.reply-sheet`/`.keys-pad` đều `position: absolute; bottom: 0` bên trong `.view-terminal` (`height: 100%`, không dùng `position: fixed` — comment tại `web/src/styles.css:640-648` giải thích cố tình tránh fixed vì từng gây lỗi `-webkit-overflow-scrolling` trên mobile). Fix cho PBI này phải giữ nguyên constraint "không dùng position:fixed".
- Không có nơi nào khác trong `web/src` hiện dùng `window.visualViewport` — đây là lần đầu tiên API này được đưa vào codebase.

### Integration Points

- `web/src/views/terminal.ts` — toàn bộ thay đổi nằm trong file này, không cần sửa `web/src/styles.css` (padding vẫn set qua inline style như hiện tại) trừ khi implementation cần thêm class/state mới (để planning quyết).

## Canonical References

- `docs/backlog.md` PBI-027 — mô tả gốc, đã được user confirm, là nguồn của D3.
- `docs/specs/terminal-detail.md` — spec hiện có cho màn hình này (chưa đọc chi tiết trong exploring; planning nên rà lại xem có đoạn nào mô tả bottom-sheet layout cần cập nhật theo D3 không).

## Outstanding Questions

### Deferred To Planning

- [ ] Thời điểm chính xác để đăng ký/huỷ đăng ký `visualViewport.resize`/`scroll` listener (lúc `openReply`/`closeReply`, hay 1 listener sống suốt vòng đời view) — implementation choice, planning quyết dựa trên cấu trúc hiện có của `terminal.ts`.
- [ ] Công thức cộng dồn chính xác giữa `overlap` (sheet vs term-bar) và khoảng trống bàn phím (`innerHeight - visualViewport.height`) — có cộng thêm `SHEET_GAP` một lần nữa hay dùng chung — planning/implementation tự quyết, không phải quyết định sản phẩm.
- [ ] Cách đảm bảo D3 (reply-sheet không bị bàn phím che) — có thể cần dịch chuyển `bottom` của `.reply-sheet` theo `visualViewport` offset thay vì chỉ set `paddingBottom` trên `#term-viewport` — planning cần đọc kỹ hành vi `visualViewport.offsetTop` trên iOS Safari trước khi quyết định approach.

## Deferred Ideas

- Áp dụng pattern `visualViewport`-aware inset cho các bottom-sheet khác có input text trong tương lai (hiện tại không có sheet nào khác cần, theo scout D1) — giữ làm tham khảo nếu sau này có sheet mới kèm input. Không tạo backlog row mới vì chưa có nhu cầu cụ thể (YAGNI).

## Handoff Note

CONTEXT.md là nguồn sự thật. D1-D4 là quyết định cố định; planning đọc code context, canonical references, và outstanding questions ở trên trước khi lên plan.
