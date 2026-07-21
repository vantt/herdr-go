---
purpose: draft content for herdr GitHub Discussions "Show and tell" post (PBI-029)
target: https://github.com/ogulcancelik/herdr/discussions/new?category=show-and-tell
date: 2026-07-21
---

## Before posting — 2 things still missing

1. **Screenshots/demo.** README itself has a TODO for this (`README.md`, top): agent list screen + an open terminal with the reply bar. A "Show and tell" post with no visual is easy to scroll past — worth capturing at least one screenshot (ideally a short GIF/screen recording of "see agent blocked on phone → tap in → reply") before posting. Can run `herdr-go --demo` per the README's own note to get a safe instance to screenshot.
2. **Repo readiness for outside traffic.** A post here will send herdr's users straight to `github.com/vantt/herdr-go`. Worth a quick look with fresh eyes right before posting: does `install.sh`/`install.ps1` work from a clean machine, does the README read well top-to-bottom, is there an issue template. Not asking you to redo this now — just flagging it as the thing that determines whether the post's traffic converts or bounces.

## Draft post

**Title:**
> Herdr Go — watch and reply to your herdr agents from your phone

**Body:**

```markdown
Built this because I kept leaving my desk while Claude Code / Codex were mid-task in herdr, and had no way to check in short of SSHing back from my phone.

**Herdr Go** is a mobile-first gateway that sits in front of herdr:

- **See at a glance** — every agent's state (working / blocked / done / idle) in one list, scannable in a couple seconds
- **Reply from anywhere** — tap into a pane, read the real terminal, type back. Full fidelity, not a status summary
- **Never babysit it** — if herdr goes down, Herdr Go brings it back on its own (one systemd unit supervises the gateway, the gateway supervises herdr)
- **Locked down by default** — one token gates everything; nothing is exposed until you say so

[screenshot: agent list]
[screenshot or GIF: open terminal, tap-to-reply]

One-line install:
\`\`\`bash
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash
\`\`\`
(Windows: `irm https://raw.githubusercontent.com/vantt/herdr-go/main/install.ps1 | iex`)

Rust (axum) backend, embeds a small TypeScript/xterm.js web UI into the same binary — no separate frontend deploy. MIT licensed: https://github.com/vantt/herdr-go

Still early (v0.1.x) — feedback, especially on the security/auth model and on how it behaves against your real herdr setup, very welcome.
```

## Notes on tone

- Kept it factual/builder-voice, matching the repo's own README register — herdr's community (repo topics: rust, terminal, tui, workspace-manager, devtools) skews toward people who want specifics, not marketing copy.
- Led with the "why" (a real personal pain point), not the feature list — matches "show and tell" norms better than a landing-page pitch.
- Named the actual weak spot (v0.1.x, wants feedback) instead of overselling — this is what keeps a companion post from reading as promotional/hollow, same concern that ruled out the plugin-for-visibility idea (see `herdr-plugin-feasibility.md`).
- Left screenshot slots as explicit placeholders rather than skipping the post over it — swap in real images/GIF before submitting.
