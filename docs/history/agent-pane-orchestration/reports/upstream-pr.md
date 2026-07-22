# Upstream PR — slice 3 (D11)

**PR: https://github.com/thanhsmind/beegog/pull/50**
Opened 2026-07-22 from `vantt:feat/herdr-orchestrating` against `thanhsmind/beegog:main`.

D11's acceptance is "PR opened upstream from the fork". Whether it is merged depends on a maintainer and is residual risk, not an acceptance criterion.

## How it was raised, and what was deliberately avoided

The fork checkout at `/home/vantt/projects/research/beegog` had **another session's uncommitted work in it** — branch `intervention-log-v2`, dirty tree including untracked `docs/history/intervention-log-v2/`. Staging, committing or switching branches there would have swept that work into this PR or destroyed it.

So the port was built in a **separate git worktree of that repository**, based on `upstream/main`:

```
git -C research/beegog worktree add <scratch>/beegog-port -b feat/herdr-orchestrating upstream/main
```

The occupied checkout was verified still on `intervention-log-v2` afterwards, untouched. This is the same paved road bee prescribes for an occupied checkout, applied to someone else's repository rather than our own.

Base was `upstream/main`, not the fork's HEAD: the fork sits 436 commits behind upstream, so branching from it would have produced a PR full of unrelated divergence.

## What was ported

The whole skill directory, **byte-identical** to this repo's copy (`diff -r` clean at commit time):

```
skills/herdr-orchestrating/
  SKILL.md
  scripts/{control-loop.sh, classify-lane.mjs, bootstrap-cockpit.sh}
  references/{dispatch-prompt.md, merge-prompt.md, dispatch-dry-run.md, spawn-proof.md}
```

The name stays `herdr-orchestrating` rather than being changed to upstream's `bee-*` convention. That is deliberate and flagged in the PR body: keeping both copies identical is the point of porting rather than forking, and a rename should move both at once. The maintainer can rename freely.

The two recorded proofs went with it. They are the part a reader will not otherwise believe: `spawn-proof.md` records an observed pane id, label and argv from one real spawn and its teardown, and `dispatch-dry-run.md` records the cold decision including the honest empty result — with its own superseded-lookup note kept rather than deleted, since what it proves is still true.

## Not carried upstream

- `docs/specs/agent-orchestration.md`, `docs/history/agent-pane-orchestration/**`, and the backlog rows: host-repo knowledge, not skill content.
- The `flock`-wrapped `commands.verify` in `.bee/config.json`: it carries a machine-specific absolute lock path. Recorded in the review as a residual — anyone adopting this skill needs their own lock path, and the skill does not currently say so.

## Open after this

The PR is the last of D11. What it does not do is make the loop *run* here: main still tracks the bee session logs, so `bee worktree merge` refuses on a dirty main and the merge delivering that very fix is itself blocked until a human untracks them in the main checkout.
