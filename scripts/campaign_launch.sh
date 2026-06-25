#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=campaign_lib.sh
source "$SCRIPT_DIR/campaign_lib.sh"

usage() {
  cat <<'EOF'
Usage: scripts/campaign_launch.sh --slug SLUG [OPTIONS]

Launch one `ldgr-research loop run` process per campaign lane, writing a PID and
log file for each lane.

Options:
  --agent NAME            Agent passed to ldgr-research loop run [default: agentctl]
  --max-iterations N      Iterations per lane [default: campaign metadata]
  --extra-arg ARG         Extra argument appended to ldgr-research loop run; may repeat
  --help                  Show this help
EOF
}

slug=""
agent="agentctl"
max_iterations=""
extra_args=()

while (($#)); do
  case "$1" in
    --slug) slug=${2:-}; shift 2 ;;
    --agent) agent=${2:-}; shift 2 ;;
    --max-iterations) max_iterations=${2:-}; shift 2 ;;
    --extra-arg) extra_args+=("${2:-}"); shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) campaign_die "unknown option: $1" ;;
  esac
done

campaign_require_slug "$slug"
campaign_load "$slug"
max_iterations=${max_iterations:-${MAX_ITERATIONS:-1}}
[[ "$max_iterations" =~ ^[0-9]+$ ]] || campaign_die "--max-iterations must be an integer"
mkdir -p "$CAMPAIGN_DIR/logs" "$CAMPAIGN_DIR/pids"

while IFS=$'\t' read -r lane _branch worktree _hypothesis; do
  [[ "$lane" == "lane" ]] && continue
  [[ -d "$worktree" ]] || campaign_die "missing worktree for lane $lane: $worktree"
  pid_file="$CAMPAIGN_DIR/pids/$lane.pid"
  log_file="$CAMPAIGN_DIR/logs/$lane.log"
  if campaign_pid_running "$pid_file"; then
    printf 'lane %-24s already running pid=%s\n' "$lane" "$(cat "$pid_file")"
    continue
  fi
  (
    cd "$worktree"
    printf 'Launching lane %s at %s\n' "$lane" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    exec ldgr-research loop run --agent "$agent" --max-iterations "$max_iterations" "${extra_args[@]}"
  ) > "$log_file" 2>&1 &
  pid=$!
  printf '%s\n' "$pid" > "$pid_file"
  printf 'lane %-24s pid=%s log=%s\n' "$lane" "$pid" "$log_file"
done < "$LANES_FILE"
