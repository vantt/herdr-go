# Product Backlog

PBI cho session khác nhận làm. Status: `proposed` → `in-flight` → `done`.

| ID | PBI | Status | Ghi chú |
|---|---|---|---|
| PBI-001 | **M0 spike — verify `events.subscribe`** (PRD §10 [CAO] #1): chạy herdr thật, test subscribe giao đủ event, không miss, không drop khi reconnect, xác nhận duplicate-event (`agent_status_changed` 2 lần/1ms) và cách de-dup. Kết quả quyết định Event watcher giữ poll 500ms hay nâng lên subscribe. Bằng chứng vào `.bee/spikes/`, delta ghi `docs/DISCOVERY.md` (discovery thắng spec). | proposed | Chặn khoá kiến trúc watcher. Không chặn notify (đã chốt ship trên poll — quyết định Telegram-B 302c0544). |
| PBI-002 | **M0 spike — Tier 2 `observe`/`control` trên mobile** (PRD §10 [CAO] #2): đo độ trễ frame qua mạng di động, reconnect full-frame resync theo `seq`, `--takeover` khi cùng agent mở nhiều thiết bị, resize khi xoay ngang/dọc phone. Trục chính của app — kết quả quyết định kiến trúc relay WebSocket↔observe/control. Bằng chứng vào `.bee/spikes/`, delta ghi `docs/DISCOVERY.md`. | proposed | Chặn khoá kiến trúc relay (lộ trình §11 bước 4). |
