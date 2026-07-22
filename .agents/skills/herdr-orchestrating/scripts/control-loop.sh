#!/usr/bin/env bash
# control-loop.sh - unbounded, stoppable poll-act loop for the herdr-orchestrating
# control agents (dispatch / merge).
#
# Each iteration invokes a FRESH headless claude session (no continuation
# across iterations): the cold start per iteration is what keeps the
# context profile flat no matter how many iterations run.
#
# The loop never terminates itself on a failing iteration - a non-zero exit
# from the iteration is reported and the loop continues. The only way to
# stop it is the human's stop gesture: create the stop file at
# .bee/tmp/herdr-orchestrating.stop. The loop checks for that file at the
# top of every iteration and exits cleanly when it is found.
#
# Usage:
#   control-loop.sh --role dispatch|merge [--interval N] [--max-iterations N]
#                    [--once] [--command CMD]
#
#   --role dispatch|merge   Which control agent this loop drives; selects
#                           the prompt file sent to claude. NOTE: --role
#                           merge is not implemented yet (slice 2 of the
#                           agent-pane-orchestration feature); it fails at
#                           runtime until the merge pane ships.
#   --interval N            Seconds between iterations. Default: 60.
#   --max-iterations N      Stop after N iterations. Test-only; omit for an
#                           unbounded loop.
#   --once                  Run exactly one iteration then exit. Test-only.
#   --command CMD           Test-only. Evaluated as a shell string via
#                           `bash -c`, run instead of invoking claude. Lets
#                           tests exercise the loop mechanics without
#                           spawning a real agent.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STOP_FILE=".bee/tmp/herdr-orchestrating.stop"

ROLE=""
INTERVAL=60
MAX_ITERATIONS=""
ONCE=0
TEST_COMMAND=""

usage() {
  cat <<'EOF'
Usage: control-loop.sh --role dispatch|merge [--interval N] [--max-iterations N] [--once] [--command CMD]

  --role dispatch|merge   Which control agent this loop drives (selects the
                          prompt file sent to claude). --role merge is not
                          implemented yet (slice 2 of agent-pane-orchestration);
                          it fails at runtime until the merge pane ships.
  --interval N            Seconds between iterations. Default: 60.
  --max-iterations N      Stop after N iterations. Test-only.
  --once                  Run exactly one iteration then exit. Test-only.
  --command CMD           Test-only: evaluated as a shell string via
                          `bash -c`, run instead of invoking claude.
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --role)
      ROLE="${2:-}"
      shift 2
      ;;
    --interval)
      INTERVAL="${2:-}"
      shift 2
      ;;
    --max-iterations)
      MAX_ITERATIONS="${2:-}"
      shift 2
      ;;
    --once)
      ONCE=1
      shift
      ;;
    --command)
      TEST_COMMAND="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "control-loop.sh: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [ -z "$ROLE" ]; then
  echo "control-loop.sh: --role dispatch|merge is required" >&2
  usage >&2
  exit 1
fi

case "$ROLE" in
  dispatch)
    ;;
  merge)
    echo "control-loop.sh: --role merge is not implemented yet (slice 2 of agent-pane-orchestration) - the merge prompt does not exist" >&2
    exit 1
    ;;
  *)
    echo "control-loop.sh: unknown role '$ROLE' (expected dispatch or merge)" >&2
    exit 1
    ;;
esac

PROMPT_FILE="$SCRIPT_DIR/../references/${ROLE}-prompt.md"

echo "interval=${INTERVAL}s"

run_iteration() {
  if [ -n "$TEST_COMMAND" ]; then
    bash -c "$TEST_COMMAND"
    return $?
  fi

  if [ ! -f "$PROMPT_FILE" ]; then
    echo "control-loop.sh: prompt file not found: $PROMPT_FILE" >&2
    return 1
  fi

  PROMPT="$(cat "$PROMPT_FILE")"
  claude -p "$PROMPT" --model sonnet --permission-mode bypassPermissions
  return $?
}

count=0
while true; do
  if [ -f "$STOP_FILE" ]; then
    echo "control-loop.sh: stop file found at $STOP_FILE; exiting"
    exit 0
  fi

  if [ -n "$MAX_ITERATIONS" ] && [ "$count" -ge "$MAX_ITERATIONS" ]; then
    exit 0
  fi

  run_iteration
  rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "control-loop.sh: iteration failed with exit code $rc; continuing" >&2
  fi

  count=$((count + 1))

  if [ "$ONCE" -eq 1 ]; then
    exit 0
  fi

  if [ -n "$MAX_ITERATIONS" ] && [ "$count" -ge "$MAX_ITERATIONS" ]; then
    exit 0
  fi

  sleep "$INTERVAL"
done
