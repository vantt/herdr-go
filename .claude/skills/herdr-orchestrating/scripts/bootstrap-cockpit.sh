#!/usr/bin/env bash
# bootstrap-cockpit.sh - builds the D13 cockpit/runtime layout for the
# agent-pane-orchestration control loop in a herdr workspace, rooted at the
# MAIN checkout (never a worktree - see docs/history/agent-pane-orchestration/
# CONTEXT.md, D13/D17/D21).
#
# A fresh workspace ends at exactly 3 tabs / 5 panes: the workspace's own
# pre-existing root tab+pane (untouched, never repurposed), the cockpit tab
# (chat / dispatch / merge), and the runtime tab (one pane to start, filled
# up to four by the dispatch loop later). No pane this script creates is
# ever labelled - dispatch and merge name themselves on first run (D17); a
# label set from outside would describe intent, not reality.
#
# --main-root is required and becomes the cwd of every tab and pane this
# script creates: `bee worktree new`/`bee worktree merge` both refuse to run
# from inside a linked worktree, so the control panes must be rooted at the
# MAIN checkout - without this, every dispatch iteration would fail forever
# while the loop dutifully continued (the same silent-stall class of bug a
# stale stop file causes, see below).
#
# Usage:
#   bootstrap-cockpit.sh --workspace ID --main-root PATH [--no-start] [--dry-run]
#
#   --workspace ID     Required. The herdr workspace to build the layout in.
#   --main-root PATH   Required. Absolute path to the MAIN checkout.
#   --no-start         Build the layout only; launch no agent.
#   --dry-run          Print the herdr commands that would run; execute
#                      nothing (no workspace, tab, pane, or agent changes).

set -u

STOP_FILE=".bee/tmp/herdr-orchestrating.stop"

WORKSPACE=""
MAIN_ROOT=""
NO_START=0
DRY_RUN=0

usage() {
  cat <<'EOF'
Usage: bootstrap-cockpit.sh --workspace ID --main-root PATH [--no-start] [--dry-run]

  --workspace ID     Required. The herdr workspace to build the layout in.
  --main-root PATH   Required. Absolute path to the MAIN checkout - becomes
                      the cwd of every tab and pane this script creates.
                      `bee worktree new`/`bee worktree merge` both refuse to
                      run from inside a linked worktree, so every control
                      pane must be rooted here, never in a worktree.
  --no-start         Build the layout only; launch no agent.
  --dry-run          Print the herdr commands that would run; execute
                      nothing (no workspace, tab, pane, or agent changes).
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --workspace)
      WORKSPACE="${2:-}"
      shift 2
      ;;
    --main-root)
      MAIN_ROOT="${2:-}"
      shift 2
      ;;
    --no-start)
      NO_START=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "bootstrap-cockpit.sh: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [ -z "$WORKSPACE" ]; then
  echo "bootstrap-cockpit.sh: --workspace ID is required" >&2
  usage >&2
  exit 1
fi

if [ -z "$MAIN_ROOT" ]; then
  echo "bootstrap-cockpit.sh: --main-root PATH is required - \`bee worktree new\`/\`bee worktree merge\` both refuse to run from inside a linked worktree, so every pane this script creates must be rooted at the MAIN checkout; without it the dispatch loop would fail every iteration while dutifully continuing" >&2
  usage >&2
  exit 1
fi

if [ -f "$STOP_FILE" ]; then
  echo "bootstrap-cockpit.sh: refusing to start - stop file present at $STOP_FILE; starting a loop that a stale stop file would immediately kill is the same silent-stall class of bug as a missing --main-root. Remove the stop file first if that is really what you want." >&2
  exit 1
fi

CONTROL_LOOP="$MAIN_ROOT/.claude/skills/herdr-orchestrating/scripts/control-loop.sh"

fail() {
  echo "bootstrap-cockpit.sh: $1" >&2
  exit 1
}

if [ "$DRY_RUN" -eq 1 ]; then
  echo "herdr tab create --workspace $WORKSPACE --cwd $MAIN_ROOT --label cockpit --no-focus"
  echo "herdr pane split <cockpit_chat_pane> --direction right --cwd $MAIN_ROOT --no-focus"
  echo "herdr pane split <cockpit_dispatch_pane> --direction down --cwd $MAIN_ROOT --no-focus"
  echo "herdr tab create --workspace $WORKSPACE --cwd $MAIN_ROOT --label runtime --no-focus"
  if [ "$NO_START" -eq 0 ]; then
    echo "herdr pane run <cockpit_dispatch_pane> \"bash '$CONTROL_LOOP' --role dispatch\""
  fi
  echo "bootstrap-cockpit.sh: dry-run - no workspace, tab, pane, or agent changes were made"
  exit 0
fi

# json_result <dotted.path.under.result> - reads one herdr JSON response on
# stdin, prints the value at that path under .result, or fails loudly
# (surfacing herdr's own .error.message) if the call did not succeed.
json_result() {
  node -e "
    let s = '';
    process.stdin.on('data', (d) => { s += d; });
    process.stdin.on('end', () => {
      let r;
      try { r = JSON.parse(s); } catch (e) { console.error('bootstrap-cockpit.sh: unparseable herdr output: ' + s); process.exit(1); }
      if (r.error) { console.error('bootstrap-cockpit.sh: herdr error: ' + (r.error.message || JSON.stringify(r.error))); process.exit(1); }
      let v = r.result;
      for (const key of '$1'.split('.')) { v = v == null ? v : v[key]; }
      if (v == null) { console.error('bootstrap-cockpit.sh: herdr response missing result.$1: ' + s); process.exit(1); }
      console.log(v);
    });
  "
}

# The cockpit tab: chat is its root pane, created directly by `tab create`
# (never a repurposed pre-existing tab). Splitting right then splitting the
# right pane down yields chat / dispatch / merge (D13); every call carries
# --cwd main-root and no --label, so none of the three panes is named by
# this script.
COCKPIT_JSON=$(herdr tab create --workspace "$WORKSPACE" --cwd "$MAIN_ROOT" --label cockpit --no-focus) || fail "herdr tab create --label cockpit failed"
CHAT_PANE=$(printf '%s' "$COCKPIT_JSON" | json_result root_pane.pane_id) || exit 1

DISPATCH_JSON=$(herdr pane split "$CHAT_PANE" --direction right --cwd "$MAIN_ROOT" --no-focus) || fail "herdr pane split (dispatch) failed"
DISPATCH_PANE=$(printf '%s' "$DISPATCH_JSON" | json_result pane.pane_id) || exit 1

MERGE_JSON=$(herdr pane split "$DISPATCH_PANE" --direction down --cwd "$MAIN_ROOT" --no-focus) || fail "herdr pane split (merge) failed"
MERGE_PANE=$(printf '%s' "$MERGE_JSON" | json_result pane.pane_id) || exit 1

# The runtime tab: one pane to start (its own root pane, rooted at
# main-root), filled up to D5's cap of four by the dispatch loop later.
RUNTIME_JSON=$(herdr tab create --workspace "$WORKSPACE" --cwd "$MAIN_ROOT" --label runtime --no-focus) || fail "herdr tab create --label runtime failed"
RUNTIME_TAB=$(printf '%s' "$RUNTIME_JSON" | json_result tab.tab_id) || exit 1

echo "bootstrap-cockpit.sh: layout built in workspace $WORKSPACE - cockpit ($CHAT_PANE chat, $DISPATCH_PANE dispatch, $MERGE_PANE merge), runtime tab $RUNTIME_TAB"

if [ "$NO_START" -eq 1 ]; then
  echo "bootstrap-cockpit.sh: --no-start - layout built, no agent launched"
  exit 0
fi

if [ ! -f "$CONTROL_LOOP" ]; then
  fail "control-loop.sh not found at $CONTROL_LOOP - layout was built but the dispatch loop was not started"
fi

# Both control loops are started. `pane run` types the command into the
# already-created pane and presses Enter; it does not block on the unbounded
# loop it starts. Dispatch first, so that if merge fails to start the half
# that creates work is at least running and the failure is visible.
herdr pane run "$DISPATCH_PANE" "bash '$CONTROL_LOOP' --role dispatch" >/dev/null || fail "could not start the dispatch loop in pane $DISPATCH_PANE"
herdr pane run "$MERGE_PANE" "bash '$CONTROL_LOOP' --role merge" >/dev/null || fail "could not start the merge loop in pane $MERGE_PANE"
echo "bootstrap-cockpit.sh: dispatch loop started in pane $DISPATCH_PANE, merge loop started in pane $MERGE_PANE"
