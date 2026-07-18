# Advisor consult digest — embed-and-package-binary

No advisor is configured in `.bee/config.json` (no `advisor` key). Per AO2b, an advisor consult is not a hard dependency when unconfigured — this file records that fact for the CLI-enforced `advisor_ref` precondition on Gate 3 for high-risk work.

## Evidence bundle (what would have been sent to a configured advisor)

**Plan summary:** Embed herdctl's built web UI into the binary (rust-embed + axum-embed) with a disk-override fallback; decouple the sqlite state-file path from `static_dir` into a new `data_dir()`; rework `install.sh` to try a prebuilt-release download before falling back to source build; simplify `release.yml`'s tarball; update README/installation docs. 5 cells, high-risk lane (5 risk flags: external systems, public contracts, cross-platform, existing covered behavior, multi-domain).

**Risk map (from approach.md):** rust-embed/axum-embed API shape (MEDIUM, since de-risked by confirming `ServeEmbed::with_parameters(fallback_file, fallback_behavior, index_file)` + `FallbackBehavior::Ok` exists on docs.rs), build.rs guard (LOW), data_dir()/sqlite decoupling (MEDIUM — confirmed by inspection to be byte-identical to today's real path for default installs), install.sh download+fallback (MEDIUM — no GitHub release exists yet, download-success branch is untestable this session, fallback branch is fully testable and is today's real condition), CI rust job (LOW with build.rs guard), release.yml simplification (LOW), docs accuracy (LOW).

**Validation findings so far:** Reality gate PASS (see validation report). Feasibility matrix in progress. Persona panel (coherence/feasibility/security/product/scope-guardian) dispatched to background review agents; cell cold-pickup review dispatched. This digest is recorded before those return, per decision 0017 (plan-checker runs in background, blocks nothing until the Gate 3 presentation itself).

**Open questions:** axum-embed's exact `with_parameters` signature (param types/order) needs confirming against the real crate at implementation time (cell embed-pkg-2 already instructs this, not guessing from docs.rs prose alone).

## Final outcome (post-panel)

Persona panel (3 background reviewers) returned: 2 BLOCKERs (install.sh toolchain-gate ordering, embed-pkg-2 verify-command ordering), 5 WARNINGs (3 fixed: debug-embed documentation, doctor-doc-surface scope, download hygiene; 2 accepted-as-documented: build.rs rerun-if-changed edge case, download-success branch untestable pre-release). Cell review: 0 CRITICAL, 2 MINOR fixed. Full detail: docs/history/embed-and-package-binary/reports/validation-current-slice.md. Structure PASS after 1 repair iteration.
