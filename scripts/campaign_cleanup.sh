#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=campaign_lib.sh
source "$SCRIPT_DIR/campaign_lib.sh"

usage() {
  cat <<'EOF'
Usage: scripts/campaign_cleanup.sh --slug SLUG [OPTIONS]

Archive campaign metadata/logs/results, remove stopped worktrees, and optionally
prune branch and Python cache debris.

Options:
  --delete-branches       Delete local lane branches after removing worktrees
  --force-running         Remove worktrees even when a lane PID still appears active
  --prune-python          Remove .venv and __pycache__ directories inside lane worktrees before removal
  --archive-dir DIR       Archive directory [default: .ldgr/campaigns/<slug>/archive]
  --help                  Show this help
EOF
}

slug=""
delete_branches=0
force_running=0
prune_python=0
archive_dir=""

while (($#)); do
  case "$1" in
    --slug) slug=${2:-}; shift 2 ;;
    --delete-branches) delete_branches=1; shift ;;
    --force-running) force_running=1; shift ;;
    --prune-python) prune_python=1; shift ;;
    --archive-dir) archive_dir=${2:-}; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) campaign_die "unknown option: $1" ;;
  esac
done

campaign_require_slug "$slug"
campaign_load "$slug"
archive_dir=${archive_dir:-$CAMPAIGN_DIR/archive}
mkdir -p "$archive_dir"
archive="$archive_dir/${slug}-$(date -u +%Y%m%dT%H%M%SZ).tar.gz"

tar -czf "$archive" -C "$CAMPAIGN_DIR" \
  --exclude ./archive \
  campaign.env lanes.tsv logs results pids 2>/dev/null || true
printf 'archive written: %s\n' "$archive"

while IFS=$'\t' read -r lane branch worktree _hypothesis; do
  [[ "$lane" == "lane" ]] && continue
  pid_file="$CAMPAIGN_DIR/pids/$lane.pid"
  if campaign_pid_running "$pid_file" && [[ "$force_running" -ne 1 ]]; then
    printf 'lane %s still running pid=%s; skip cleanup (use --force-running to override)\n' "$lane" "$(cat "$pid_file")"
    continue
  fi
  if [[ "$prune_python" -eq 1 && -d "$worktree" ]]; then
    find "$worktree" -type d \( -name __pycache__ -o -name .pytest_cache -o -name .mypy_cache -o -name .ruff_cache \) -prune -exec rm -rf {} + 2>/dev/null || true
    rm -rf "$worktree/.venv"
  fi
  if git worktree list --porcelain | grep -Fqx "worktree $worktree"; then
    git worktree remove "$worktree"
    printf 'removed worktree: %s\n' "$worktree"
  elif [[ -d "$worktree" ]]; then
    printf 'worktree path exists but is not registered; leaving untouched: %s\n' "$worktree"
  fi
  if [[ "$delete_branches" -eq 1 ]]; then
    git branch -D "$branch" || true
  fi
  rm -f "$pid_file"
done < "$LANES_FILE"

git worktree prune
printf 'cleanup complete for campaign: %s\n' "$slug"
