---
topic: version-lockstep-release-discipline
date: 2026-08-08
based_on: [collie@f7b692b0]
entries: [collie:quality-gates-version-triple-lockstep-enforcement, collie:repo-layout-tag-per-release-drives-github-release]
---

# Deep-dive: version bump discipline (release lockstep)

**Bottom Line:** collie enforces version agreement across N files (manifest + 2 package.json + CHANGELOG) with 3 redundant, independently-triggerable checks (build script, pre-commit hook, CI) plus a `sort -V` monotonicity guard so a bump can never go backwards. herdr-gateway has exactly ONE version file (`Cargo.toml`, `web/package.json` is untracked/unrelated — confirmed stale at `0.1.0` regardless of product version) and ZERO automated enforcement — the last two bumps (`0.1.9`, `0.1.10`) were both manual, unchecked edits. Recommendation: port the *mechanism* (single check script, called from a git pre-commit hook) but not the *problem* (we have one canonical file, not three, so there is nothing to keep "in lockstep" — only "did you forget to bump it at all" and "did you bump it backwards").

## Câu hỏi
Làm sao đảm bảo version luôn được bump đúng, không lệch, không thụt lùi, mỗi khi có thay đổi chức năng — mà không cần con người tự nhớ?

## Cách collie giải quyết

**Mechanism (source: `scripts/check-version.sh`, read live at `origin/main`, commit `8c898a0`+):**
- 1 script đọc version từ 4 nguồn: `herdr-plugin.toml` (canonical), `package.json`, `web/package.json`, dòng `## [x.y.z]` mới nhất trong `CHANGELOG.md`. So bằng nhau hết → pass; lệch bất kỳ đâu → fail kèm bảng in ra đúng-sai từng file.
- Script này được gọi từ **3 chỗ độc lập**: `collie-ctl.sh build` (build time), `scripts/git-hooks/pre-commit` (commit time), `.github/workflows/ci.yml` (CI time, dòng 21-23: "The MANDATORY invariant").
- `pre-commit` làm thêm việc thứ 2 mà `check-version.sh` không làm: **bump-on-change**. Nó `git diff --cached --name-only`, nếu có file "functional" (bridge/, web/src/, scripts/, herdr-plugin.toml...) nằm trong staged diff mà version hiện tại (`herdr-plugin.toml`) == version ở `HEAD` → chặn commit, báo "bump version". Doc-only changes không bị chặn.
- **Monotonic guard**: `printf '%s\n%s\n' "$prev" "$cur" | sort -V | tail -1` — nếu kết quả không phải `$cur` nghĩa là version đi lùi → chặn. `sort -V` (version-sort) là cách rẻ để so semver đúng mà không cần parse tay.
- **Escape hatch đặt tên rõ ràng**: `SKIP_VERSION_CHECK=1 git commit …` — luôn gõ inline tại điểm dùng, không phải flag ẩn hay config toggle, nên lịch sử shell/CI log giữ lại bằng chứng ai bypass lúc nào.
- **Release thật sự chỉ trigger bởi 1 hành động**: push git tag `vX.Y.Z` khớp version → `.github/workflows/release.yml` tự tạo GitHub Release, kéo đúng section CHANGELOG đó làm release notes. Tagging là bước thủ công DUY NHẤT trong toàn bộ kỷ luật version — cố ý, vì đó cũng là tín hiệu "cái này đã ship" không thể đảo ngược.

**Why (bối cảnh khiến họ chọn vậy):** 4 file version tồn tại vì herdr đọc `herdr-plugin.toml`, Bun/npm tooling đọc 2 `package.json`, và người dùng đọc CHANGELOG — không có cách gộp 4 nguồn này thành 1 mà không đổi tooling khác (herdr, bun, GitHub Release). Redundant 3 lớp vì "con người là tuyến phòng thủ đầu" (CLAUDE.md), 3 lớp là backstop chứ không tin tưởng 1 điểm chặn duy nhất — hook có thể bị `--no-verify`, CI luôn chạy nhưng chỉ báo SAU khi push.

## herdr-gateway hiện tại (baseline thật, không suy đoán)

- Đúng **1 nguồn version thật**: `Cargo.toml` (`version = "0.1.10"`), `Cargo.lock` tự sync theo khi build.
- `web/package.json` có field `version` riêng nhưng **không liên quan** tới version sản phẩm — hiện đứng yên ở `0.1.0` từ đầu, không ai đồng bộ, không tooling nào đọc nó để publish. Không phải lệch cần fix — chỉ là 1 field chết, không nằm trong "lockstep" nào cả.
- Không có CHANGELOG.md — lịch sử version nằm trong commit message (`chore: bump version to 0.1.X`) + `docs/backlog.md`.
- Không có git hook, không có CI step kiểm version. 2 lần bump gần nhất (`0.1.9`→`c1d562e`, `0.1.10`→`82aae00`) đều là sửa tay `Cargo.toml`, chạy `cargo build --release` để `Cargo.lock` tự cập nhật theo — không ai/không gì xác nhận lại.
- Release thật (`release.yml`) — cần kiểm lại có dùng tag-push để trigger hay không (chưa đọc trong phiên này; nêu ở Open questions).

## So sánh & trade-offs

| | collie | herdr-gateway |
|---|---|---|
| Số file version cần đồng bộ | 4 (manifest+2×package.json+CHANGELOG) | 1 thật (Cargo.toml), Cargo.lock ăn theo tự động qua build |
| Nguy cơ chính | LỆCH giữa các file | QUÊN bump / bump LÙI |
| Enforcement hiện có | 3 lớp (build+hook+CI) | 0 |
| Escape hatch | named env var, inline | N/A (chưa có gate để bypass) |
| Trigger release thật | push tag `vX.Y.Z` | chưa xác nhận (Open question) |

Vì chỉ có 1 file thật, bài toán "lockstep" của collie không áp dụng nguyên xi — cái cần port là 2 tính chất: **(a) đổi code chức năng mà quên bump** và **(b) bump lùi/gõ nhầm**, đúng 2 việc `pre-commit` của họ làm, tách khỏi việc đối chiếu 4 file.

## Giải pháp tổng hợp cho host

1. **1 script nhỏ** `scripts/check-version.sh` (hoặc `.mjs`/tương đương repo convention — repo này Rust nên bash là hợp) đọc version từ `Cargo.toml` duy nhất (`sed`/`grep` dòng `^version = "..."`, đúng kỹ thuật collie dùng, không cần parser TOML riêng).
2. **Git pre-commit hook** (tận dụng `core.hooksPath` pattern — xem thêm `collie.md#hooks-installable-hookspath-switch`, cùng nguồn, cùng đáng port): nếu staged diff đụng `src/`, `web/src/`, hoặc bất kỳ path chức năng nào — và version trong `Cargo.toml` staged == version ở `HEAD:Cargo.toml` → chặn commit, in message yêu cầu bump. Doc-only (`docs/`, `*.md`, `plans/`) không bị chặn.
3. **Monotonic check** bằng đúng `sort -V` trick — không viết parser semver riêng.
4. **Escape hatch**: `SKIP_VERSION_CHECK=1 git commit ...`, đặt tên y hệt để nhất quán thói quen nếu sau này port thêm gate khác.
5. **KHÔNG port**: đối chiếu multi-file (không có 4 file để lệch), không cần CI riêng thêm bước version-check nếu CI đã chạy `cargo build` (bump-lùi vẫn build được — nên `sort -V` vẫn cần chạy trong CI, chỉ đơn giản chưa chắc "functional-change-without-bump" có ý nghĩa để enforce cứng trong CI, vì local hook đã chặn tại nguồn — có thể để CI chỉ chạy `sort -V` không-lùi làm lưới an toàn thứ 2, rẻ, không cần thêm cơ chế mới).
6. Bỏ hẳn `web/package.json`'s `version` field khỏi mọi câu chuyện version — không sửa nó theo Cargo.toml, không xoá nó (không thuộc scope bump discipline), chỉ đừng nhầm nó là "1 trong các file cần lockstep".

## Portable ideas

- `version-bump-on-functional-change-precommit` — R2 E2 F1 — pre-commit chặn commit đổi code chức năng mà version không đổi so với HEAD.
- `version-monotonic-sort-v-guard` — R2 E2 F1 — dùng `sort -V` chống bump lùi/gõ nhầm, không viết parser semver riêng.
- `named-env-var-escape-hatch-convention` — R3 E2 F1 — quy ước đặt tên escape hatch cho MỌI gate tương lai (không chỉ version): named env var, gõ inline tại điểm dùng, không flag ẩn.

## Open questions

- `release.yml` hiện tại của herdr-gateway trigger bằng gì (tag push, hay thủ công `gh release create`)? Chưa đọc lại trong phiên này — cần xác nhận trước khi viết hook, để hook không xung đột với flow release thật.
- Có muốn CHANGELOG.md thật không, hay giữ nguyên "lịch sử nằm trong commit + backlog.md" như hiện tại? Quyết định này ảnh hưởng scope của check script (có cần đối chiếu CHANGELOG hay không).
