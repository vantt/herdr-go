---
topic: verify-before-submit-send-guard
date: 2026-08-08
based_on: [collie@f7b692b0]
entries: [collie:harness-race-guard-verify-before-submit]
---

# Deep-dive: verify-before-submit guard (ack ≠ delivered)

**Bottom Line:** herdr's socket API acks the moment it accepts bytes — never when the target TUI has actually rendered/consumed them (`HERDR_API.md`: "An ack means herdr took the bytes, never the TUI acted on them"). collie's reply path used to type text then fire Enter after a fixed 350ms sleep; a real production incident (#34) proved this unsafe — a focused permission dialog swallowed the typed reply and Enter answered the DIALOG instead, silently destroying the message while both RPCs reported success. The fix: type without submitting, poll the pane back until the typed text is verifiably visible, only THEN send the submit key — never blind. herdr-gateway's `replySend` (`web/src/views/terminal.ts:602-612`) has the EXACT same shape collie had before the fix: it calls `sendReply(paneId, text, replyEnter.checked)` in one shot, no readback, no verification. Same API contract (`ack ≠ delivered` is a herdr-wide property, not collie-specific — 3 independent sources converge on it, see `docs/distillery/comparison-matrix.md` harness section, "Hội tụ độc lập lần 3"), same exposure.

## Câu hỏi
Làm sao biết chắc 1 reply thật sự "tới" ô nhập của agent trước khi bấm Enter — khi API chỉ xác nhận "đã nhận byte", không xác nhận "TUI đã xử lý"?

## Cách collie giải quyết

**Kiến trúc guard (đọc trực tiếp `web/src/lib/harness/guard.ts` + `web/src/lib/reply-action.ts` tại `origin/main`, đã refactor nhiều lần từ bản 8c898a0 — đây là bản MỚI NHẤT, trưởng thành hơn":**

1. **Type KHÔNG submit trước** — gọi gửi text với `submit=false`.
2. **Poll bounded** (`POLL_ATTEMPTS=8`, `POLL_DELAY_MS=350` — tổng ~2.8s) đọc lại pane, mỗi lần parse ra "draft" hiện đang nằm trong ô nhập (`adapter.extractInputDraft`).
3. **So khớp** draft vừa đọc với text vừa gửi bằng `draftCarriesSend()` — không phải so bằng nhau tuyệt đối, vì ô nhập có thể: (a) chỉ hiện phần ĐUÔI của text dài (windowed), (b) gộp dòng wrap lại bằng 1 khoảng trắng giả (fold). Thuật toán: tách draft thành các "run" không-khoảng-trắng, build regex cho phép khoảng trắng co giãn CHỈ ở đúng vị trí fold-seam, khớp theo ranh giới ký tự thị giác thật (grapheme cluster, không phải code unit — để không cắt đôi 1 emoji ghép), yêu cầu tối thiểu `MIN_MATCH_CHARS=8` ký tự thấy được khớp liên tục.
4. **Chỉ khi khớp** → gửi tiếp 1 lệnh submit-only: `sendReply(paneId, "", true)` — text rỗng + submit=true nghĩa là bridge/backend CHỈ gửi phím submit, không gõ lại gì (đường "gõ rỗng = chỉ bấm Enter" này bridge của collie đã hỗ trợ sẵn — `sendReplySteps`, `bridge/server.ts:575-606`, `if (txt) {...}`).
5. **Không khớp sau hết vòng poll** → KHÔNG gửi phím submit, trả `status: "stalled"`, giữ nguyên draft cho user, không mất tin nhắn, không trả lời nhầm dialog nào.
6. **Pre-flight thêm 1 lớp trước cả bước 1**: đọc pane 1 lần TRƯỚC KHI gõ, nếu `adapter.composerReady()` báo false (không thấy ô nhập, có thể đang có menu/dialog chiếm toàn màn hình) → không gõ gì cả, trả `status: "blocked"`. Lý do: guard ở bước 2-4 chỉ ngăn Enter trả lời nhầm dialog, KHÔNG ngăn được chính TEXT bị gõ nhầm vào 1 picker không có ô nhập ở cuối màn hình (ví dụ `/model` picker của Claude).
7. **Sanitize text trước khi gõ** (`sanitizeTypedText`): gộp mọi whitespace thành 1 space, strip toàn bộ ký tự điều khiển C0/C1 — vì clipboard paste có thể lén nhét ESC (huỷ dialog), BEL, ETX vào giữa text, và các ký tự này sẽ được gõ THẬT vào input TRƯỚC khi readback kịp kiểm tra.

**Why (bối cảnh):** sự cố #34 là lỗi thật, đo được: ack báo `{ok:true}` cho cả `send_text` VÀ `send_keys`, nhưng dialog đã tiêu thụ cả 2 — message mất, dialog bị trả lời sai (thường "Yes" vì đó là option mặc định highlight). herdr's `revision` field trên `pane.read` được xác nhận sống là **stub luôn = 0** (herdr 0.7.x) — không dùng được làm tín hiệu "màn hình đổi chưa", nên guard KHÔNG được phép dựa vào nó, phải luôn re-derive nội dung thật để so sánh (`entryGuard`'s comment: "the model re-derivation below runs on EVERY path, including 304").

## herdr-gateway hiện tại (baseline thật)

`web/src/views/terminal.ts:602-612`:
```ts
replySend.addEventListener("click", async () => {
  const text = replyText.value;
  if (text.length === 0) return;
  replySend.disabled = true;
  const ok = await sendReply(props.agent.pane_id, text, replyEnter.checked);
  replySend.disabled = false;
  if (ok) { replyText.value = ""; closeReply(); void poll(); }
  else { replyText.setAttribute("aria-invalid", "true"); }
});
```
- 1 lệnh duy nhất, `text` + `submit` cùng lúc — y hệt shape collie TRƯỚC fix #34.
- `sendReply` (`web/src/api.ts:200-211`) gọi `POST /api/panes/:pane/input {text, submit}` — backend (`src/web/screen.rs:95`, `state.herdr.send_input(&pane, &body.text, body.submit)`) đã hỗ trợ sẵn `text=""` (chỉ submit, không gõ) — **API contract giống hệt collie's `sendReplySteps`**, không cần đổi backend/Rust gì cả để port guard này.
- `fetchScreen(pane_id, history?)` đã có sẵn (dùng cho poll()/loadOlder()) — đúng công cụ "đọc lại pane" guard cần, không cần route mới.
- KHÔNG có per-agent adapter/harness (`docs/backlog.md` xác nhận: mình luôn "Tier 0", raw mirror cho mọi agent) — nên không có `extractInputDraft` theo nghĩa collie (đọc đúng NỘI DUNG Ô NHẬP, tách khỏi phần còn lại màn hình). Đây là khác biệt kiến trúc quan trọng nhất cần thiết kế lại, không port thẳng.

## So sánh & trade-offs

| | collie | herdr-gateway |
|---|---|---|
| Có adapter đọc riêng ô nhập | Có (per-agent, Tier 1+) | Không (raw mirror mọi agent) |
| Nguồn "draft" để so khớp | `extractInputDraft()` — đúng vùng ô nhập | Phải dùng toàn bộ text màn hình (hoặc dòng cuối) — thô hơn |
| Fold/windowing ô nhập cần xử lý | Có (ô nhập có thể wrap/cắt) | Chưa biết — terminal.ts hiện show full-screen scroll, không có khái niệm "ô nhập" riêng để windowed |
| API backend cần đổi | Không (bridge đã có empty-text-submit-only) | Không (backend Rust đã có sẵn, xác nhận ở trên) |
| Pre-flight (chặn gõ nhầm vào dialog full-screen) | Có, qua `composerReady` | Không có khái niệm tương đương — CẦN THIẾT KẾ |

## Giải pháp tổng hợp cho host

Vì herdr-gateway không có harness/adapter, KHÔNG port nguyên xi `extractInputDraft`/`draftCarriesSend`'s grapheme-fold logic (được thiết kế cho 1 ô nhập TÁCH BIỆT khỏi màn hình còn lại — mình không có khái niệm đó). Thiết kế rút gọn, đúng bối cảnh Tier-0/raw-mirror:

1. **Bỏ pre-flight `composerReady`** (F thấp, cần adapter mình không có) — chấp nhận rủi ro "gõ nhầm vào 1 dialog full-screen" ở mức thấp hơn collie, NHƯNG vẫn đóng được lỗ hổng chính (#34's Enter-trả-lời-nhầm-dialog) bằng bước 2 dưới đây — đây là phần giá trị lớn nhất, chi phí thấp nhất.
2. **Tách gửi làm 2 bước, y hệt luồng lõi collie**:
   - Gửi `sendReply(paneId, text, false)` (KHÔNG submit).
   - Poll bounded (gợi ý tái dùng đúng số POLL_ATTEMPTS=8/POLL_DELAY_MS=350 collie đã tune qua thực chiến, không cần tự đo lại) gọi `fetchScreen(pane_id)`, kiểm tra text vừa gõ có xuất hiện GẦN CUỐI màn hình hay không (đơn giản hơn collie: match substring liên tục trên vài dòng cuối, KHÔNG cần xử lý fold-seam vì mình show full raw text, không windowed) — 1 hàm nhỏ kiểu `screenTailContains(screenText, sentText)`.
   - Khớp → gửi `sendReply(paneId, "", true)` (chỉ submit).
   - Không khớp sau khi hết vòng poll → giữ nguyên `replyText.value`, KHÔNG đóng panel, báo lỗi rõ ràng ("Tin nhắn có thể chưa tới nơi — có thể 1 dialog khác đang mở"), để user tự quyết định gửi lại hay không (ứng với `status:"stalled"` của collie).
3. **Sanitize text trước khi gõ** — port nguyên `sanitizeTypedText`'s Ý TƯỞNG (gộp whitespace + strip control char C0/C1) trước khi gửi bất kỳ text nào — rẻ, không phụ thuộc adapter, đóng đúng lỗ paste-ESC/BEL mà mình CHƯA có bất kỳ phòng vệ nào hiện tại.
4. **`replyEnter.checked = false`** (Press Enter toggle tắt) vẫn giữ nguyên hành vi cũ — guard chỉ áp dụng khi checkbox bật (đang định gửi kèm Enter); tắt checkbox nghĩa là user chủ động chỉ muốn gõ, không có gì để "verify trước khi submit" cả.

## Portable ideas

- `type-then-verify-then-submit-split` — R2 E3 F2 — tách gửi làm 2 lệnh (type-only rồi submit-only), không bao giờ gửi Enter mù. Evidence tier 3 (3 nguồn độc lập hội tụ trên "ack ≠ delivered": airemote, collie, herdr's own docs — xem `comparison-matrix.md`).
- `sanitize-typed-text-control-chars` — R2 E2 F1 — strip C0/C1 + gộp whitespace trước khi gõ bất kỳ text nào vào TUI, chặn ESC/BEL lén qua clipboard paste.
- `empty-text-submit-only-api-shape` — R1 E2 F1 — không phải port, mà XÁC NHẬN: backend hiện tại (`send_input` với `text=""`) đã đúng shape cần cho pattern này, không cần đổi Rust.

## Open questions

- ~~Ngưỡng "match"...~~ **Quyết 2026-08-08 (implement, PBI-062):** N=3 dòng cuối (`REPLY_GUARD_TAIL_LINES`), match FULL text vừa gửi (không phải substring lỏng) — giảm false-positive so với chỉ so 1 đoạn ngắn. Chưa verify trên fixture Claude Code thật (reply dài hơn 1 dòng có wrap/xuống dòng khác 3-dòng-cuối giả định không — chưa biết).
- ~~Poll 8×350ms...~~ **Quyết 2026-08-08:** giữ nguyên 8×350ms (`REPLY_GUARD_POLL_ATTEMPTS`/`REPLY_GUARD_POLL_DELAY_MS`) làm điểm khởi đầu như khuyến nghị, CHƯA đo lại false-timeout thật trên `POLL_MS=1500` của host — cần quan sát thực chiến trước khi tune.
