#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=campaign_lib.sh
source "$SCRIPT_DIR/campaign_lib.sh"

usage() {
  cat <<'EOF'
Usage: scripts/campaign_status.sh --slug SLUG [OPTIONS]

Show PID/log state and compact LDGR status for each campaign lane.

Options:
  --tail N       Show last N log lines per lane [default: 5]
  --no-ldgr      Do not run ldgr status in worktrees
  --help         Show this help
EOF
}

slug=""
tail_lines=5
show_ldgr=1

while (($#)); do
  case "$1" in
    --slug) slug=${2:-}; shift 2 ;;
    --tail) tail_lines=${2:-}; shift 2 ;;
    --no-ldgr) show_ldgr=0; shift ;;
    --help|-h) usage; exit 0 ;;
    *) campaign_die "unknown option: $1" ;;
  esac
done

campaign_require_slug "$slug"
campaign_load "$slug"
[[ "$tail_lines" =~ ^[0-9]+$ ]] || campaign_die "--tail must be an integer"

all_done=1
while IFS=$'\t' read -r lane branch worktree _hypothesis; do
  [[ "$lane" == "lane" ]] && continue
  pid_file="$CAMPAIGN_DIR/pids/$lane.pid"
  log_file="$CAMPAIGN_DIR/logs/$lane.log"
  status="stopped"
  pid="-"
  if campaign_pid_running "$pid_file"; then
    status="running"
    pid=$(cat "$pid_file")
    all_done=0
  elif [[ -s "$pid_file" ]]; then
    pid=$(cat "$pid_file")
  fi
  printf '\n== %s ==\n' "$lane"
  printf 'branch: %s\nworktree: %s\npid: %s\nstatus: %s\nlog: %s\n' "$branch" "$worktree" "$pid" "$status" "$log_file"
  if [[ "$show_ldgr" -eq 1 && -d "$worktree" ]]; then
    printf -- '-- ldgr status --\n'
    (cd "$worktree" && ldgr status) || true
  fi
  if [[ -f "$log_file" && "$tail_lines" -gt 0 ]]; then
    printf -- '-- log tail --\n'
    tail -n "$tail_lines" "$log_file" || true
  fi
done < "$LANES_FILE"

if [[ "$all_done" -eq 1 ]]; then
  printf '\nall lanes are stopped/completed\n'
else
  printf '\none or more lanes are still running\n'
fi
