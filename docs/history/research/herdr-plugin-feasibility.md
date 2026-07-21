---
artifact_contract: bee-research/v1
topic: herdr-plugin-feasibility
depth: standard
date: 2026-07-21
---

## Bottom Line

- Recommendation (ladder rung): **none — not feasible as scoped**
- Why this is the lightest credible path: no rung applies. herdr's plugin model requires herdr to be the launcher and the plugin to be a subordinate, on-demand/event-driven process with no autostart or daemon capability. herdr-gateway's core job (autostart on boot, always-on supervisor that starts/restarts herdr) is the structural inverse of that model — this isn't a missing feature that reuse/built-in/adapt/build could close, it's a lifecycle-direction conflict.
- Why the next-best rung lost: "adapt-upstream" (making the *whole gateway* a herdr plugin) was rejected because the plugin runtime literally cannot daemonize or launch independently of herdr (Docs, confirmed live below) — adapting around that would mean removing the autostart/supervision behavior that is the gateway's reason to exist.
- Confidence: 90%
- Suggested next step: **none** for PBI-029 as currently worded ("triển khai + publish herdr-gateway như plugin herdr"). This finding reinforces an already-locked PRD decision (see Local, below) — flag to user rather than proceeding into planning for a design that repeats a documented rejected direction.

## Repo Snapshot

- Repo type / primary languages / runtimes: Rust (edition 2021) backend (`herdr-go`, binary `herdr-go`), Vite + TypeScript web UI in `web/` (xterm.js terminal), embedded into the Rust binary via `rust-embed`/`axum-embed`.
- Frameworks and detectable versions: axum 0.7 (ws), tokio 1 (full), tower/tower-http 0.5, rusqlite 0.31 (bundled), reqwest 0.12, interprocess 2.4.2 — all from `Cargo.toml`.
- Relevant packages, services, tools: this project (`herdr-gateway`) is a web-first remote gateway + supervisor *for* herdr (per `Cargo.toml` description), not a herdr plugin today.
- Constraints or workflows that shape the answer: `docs/PRD.md` §9 already specifies the intended lifecycle model — one systemd user unit supervises the gateway, and the gateway supervises herdr (health-check + restart).

## Question & Assumptions

- What was asked (PBI-028): does herdr have an official plugin mechanism; what is its shape (manifest, entry point, registration); is there a publish registry/marketplace; what constraints apply (protocol version, sandboxing, permissions) — output a feasibility decision plus the technical shape PBI-029 would need.
- What success appears to mean: a clear go/no-go on packaging/publishing herdr-gateway as a herdr plugin, with enough shape detail to plan PBI-029 if the answer is go.
- Assumptions still needing confirmation: none blocking — the no-go is structural, not version-dependent (see Inference).

## Findings

### Local
- `docs/PRD.md:158-162` already locks the lifecycle model: *"Mô hình: systemd canh gateway; gateway canh herdr."* and explicitly: *"Gateway KHÔNG phải herdr plugin — herdr không có autostart-plugin-khi-boot, và plugin sẽ khoá vòng đời gateway vào herdr (ngược hoàn toàn: ở đây gateway phải trên herdr để giám sát nó)."* This is a standing architectural decision, not a new conclusion of this brief — this research independently arrives at the same place with fresh evidence (below).
- `docs/distillery/sources/herdr.md` (dated 2026-07-15, 6 days old, distilled from herdr's own docs/source) already inventories the plugin system in detail: manifest `herdr-plugin.toml`, `[[actions]]`/`[[panes]]`/`[[events]]` entry points, `herdr plugin link/install/unlink/uninstall`, `HERDR_BIN_PATH`-based CLI-as-API model, trust/preview gate + `--yes`, `min_herdr_version` hard gate, GitHub-topic (`herdr-plugin`) marketplace with no curation. No mention anywhere in the distilled inventory of autostart, daemonization, or independent process lifecycle for a plugin.
- No `.bee/decisions.jsonl` entry mentions "plugin" — this is a PRD-level product decision, not a formally logged bee D-ID, but it is an explicit prior user/architecture decision per the repo's own docs.

### Upstream
- Not separately inspected as a distinct GitHub repo in this pass — the official docs (below) already cite exact CLI surface and manifest fields, and the local distillation (which was itself sourced from herdr's docs/code) corroborates the same shape. No upstream example plugin repo was pulled; not needed to answer feasibility.

### Docs
- Source checked: `https://herdr.dev/docs/plugins/` (fetched live 2026-07-21, current).
- Manifest: `herdr-plugin.toml`, required fields `id`, `name`, `version`, `min_herdr_version`; entry points via `[[actions]]`, `[[panes]]`, `[[events]]`, each needing a `command` (argv).
- Registration: `herdr plugin link` (local dev, no build step) / `herdr plugin install owner/repo[/subdir]` (clone + build after trust preview) / `unlink` / `uninstall`.
- **Autostart/daemon: "No autostart-on-boot or background daemon functionality is documented. Plugins are event-driven or manually invoked."**
- **Independence: plugins "cannot run independently. They are a directory with a herdr-plugin.toml manifest and commands Herdr can launch." Runtime commands get injected env vars and call back into Herdr via `HERDR_BIN_PATH` or the socket API.**
- Registry: official marketplace at `/plugins/`, auto-discovered via the GitHub topic `herdr-plugin`, index refreshes every 30 minutes — no curated/reviewed registry, distribution is plain GitHub.
- Sandboxing: none beyond a one-time trust preview before install; "does not review or sandbox what a plugin does."
- Version gate: `min_herdr_version` enforced at link/install time — "Herdr refuses to link or install a plugin when its minimum version is newer than the current binary."
- This matches the local distillation from 6 days ago almost verbatim — no drift, no version-sensitive surprise.

### Inference
- herdr-gateway must be the thing that starts, health-checks, and restarts herdr (PRD §9) and must itself autostart on boot (systemd unit, `Restart=always`). A herdr plugin is, by the live docs, the opposite shape: it is launched *by* herdr, on demand or on an herdr-emitted event, with no daemon/autostart capability and no ability to run independently. Packaging the gateway as a herdr plugin would either (a) strip the autostart/supervisor role entirely — defeating the product's purpose — or (b) require the gateway to keep running outside the plugin mechanism anyway and use the plugin only as a thin registration shim, which is not what PBI-028/029 describe ("publish herdr-gateway như 1 plugin").
- A narrower, different feature — a small companion herdr plugin exposing gateway status/actions as an `[[action]]` or `[[pane]]` inside herdr's own UI, with the real gateway process running independently as today (systemd-supervised) — would be technically buildable (manifest + one CLI command calling back into the gateway's existing HTTP/socket API). That is a materially smaller and different feature than "publish herdr-gateway as a plugin," and is not what either backlog item currently asks for.

## Risks, Unknowns, Follow-Ups

- Technical risk: none identified that changes the verdict — the blocking constraint (no daemon/autostart, herdr-as-launcher) is stated plainly and consistently in both the live docs and the local distillation.
- Evidence gap: the upstream herdr GitHub repo's plugin-runtime source itself was not read directly (docs + existing local distillation were sufficient to answer feasibility; not needed for a no-go verdict of this clarity).
- Open question for the user: PBI-028/029 as worded assume the gateway itself becomes a plugin. Given this is already precluded by `docs/PRD.md:162` and now independently reconfirmed, the real decision is whether to (a) close PBI-028/029 as answered-by-existing-PRD-decision, (b) re-scope PBI-029 to the much smaller "companion status/action plugin" idea above, or (c) something else. This is a backlog/product call, not a research call.

## Addendum (2026-07-21, same day): community recognition without a plugin

The user's actual goal behind PBI-028/029 is community recognition/visibility for herdr-gateway, not the plugin mechanism per se — a companion-plugin-just-to-be-listed felt contrived. Widened the research to "what are all the ways a third-party project gets recognized in herdr's ecosystem", ranked lightest-first:

### Docs
- `https://herdr.dev` (live, 2026-07-21) surfaces four channels: the plugin marketplace (`/plugins/`, "150+ community plugins, auto-discovered from GitHub", tag `herdr-plugin`); an **Integrations** page (`/docs/integrations/`, 15 listed agent CLIs — Pi, Claude Code, Codex, Droid, Amp, OpenCode, Grok CLI, Hermes, Cursor, Antigravity, Kimi, Kiro, Copilot, Qoder CLI, MastraCode — "any terminal agent works out of the box, integrations add richer state and session resume"); a **"from the community"** section of independent YouTube showcase videos (Jilles, DevOps Toolbox, Better Stack) with no visible self-service submission path; and `/docs/plugins/` itself.
- `https://herdr.dev/docs/plugins/` (live, 2026-07-21) documents no submission process beyond the GitHub-topic tag — no Discussions category, form, or Discord is mentioned there.
- `gh api repos/ogulcancelik/herdr`: `has_discussions: true`, 18,915 stars — a large, active repo.
- `gh api graphql` discussion categories on `ogulcancelik/herdr`: `Announcements`, `General`, `Ideas`, `Q&A`, and **`Show and tell` — "Show off something you've made"**. This is an official, maintainer-run community channel built exactly for this purpose.

### Local
- `docs/distillery/sources/herdr.md:237-241` (`per-agent-integration-hooks`) confirms Integrations are lifecycle hooks for agent CLIs running *inside* herdr panes (each writes into that agent's own config dir and reports over herdr's socket) — herdr-gateway is a supervisor/relay that sits outside/above herdr, not an agent running in a pane, so this channel does not fit regardless of effort spent.

### Inference
- The Integrations page is structurally inapplicable (wrong domain — agent lifecycle hooks, not gateway/relay tools); ruled out.
- Community showcase videos are not self-service; not actionable directly.
- The plugin marketplace remains available later, but only stops being contrived once there is a real action/pane worth exposing inside herdr's UI — not evaluated further now.
- **The "Show and tell" GitHub Discussion is the lightest credible path to the user's actual goal**: zero code, uses an existing official community channel at the intended purpose, on a repo with real reach (18.9k stars). This rung ("reuse existing capability") beats both "adapt-upstream" (build a plugin) and "build" outright.

### Decision (user, 2026-07-21)
User chose: post to herdr's "Show and tell" Discussion; park the companion-plugin idea until a genuinely useful action/pane exists (not pursued for its own sake).

## Source Pack

- Local files read: `Cargo.toml`, `web/package.json`, `docs/PRD.md` (§9, lines ~145-174), `docs/distillery/sources/herdr.md`, `docs/distillery/reports/distill-herdr-inventory-2026-07-15.md`, `docs/distillery/comparison-matrix.md`, `.bee/decisions.jsonl` (grepped for "plugin", no hits).
- Upstream repos/pages checked: `github.com/ogulcancelik/herdr` (via `gh api`, repo metadata + discussion categories).
- Docs pages checked: `https://herdr.dev/docs/plugins/` (live, 2026-07-21), `https://herdr.dev` (live, 2026-07-21).
