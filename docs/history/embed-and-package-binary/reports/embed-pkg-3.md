# embed-pkg-3

Status: [DONE]

Outcome: Reworked `install.sh` to try a prebuilt-release download first (target detection, quoted+https-only curl into an isolated temp dir, `HERDCTL_VERSION` override), relocating the cargo/npm prereq check so it only gates the source-build fallback; removed the now-obsolete static-assets copy and `static_dir` config rewrite; reordered `dev-deploy.sh` to bundle before compile; fixed a stale org URL in the systemd unit. Verified with a real `./install.sh` run on this machine, which genuinely exercised detect → download-fails (no release exists) → fallback-to-source-build → install (exit 0).

Files touched: `install.sh`, `dev-deploy.sh`, `packaging/herdr-gateway.service`

Full trace/evidence: `.bee/cells/embed-pkg-3.json`
