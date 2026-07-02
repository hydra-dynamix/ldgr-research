#!/usr/bin/env bash
set -euo pipefail

campaign_die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

campaign_repo_root() {
  git rev-parse --show-toplevel 2>/dev/null || campaign_die "run from inside a git repository"
}

campaign_repo_name() {
  basename "$(campaign_repo_root)"
}

campaign_default_worktree_root() {
  local root
  root=$(campaign_repo_root)
  printf '%s\n' "$(dirname "$root")/$(campaign_repo_name)-campaign-worktrees"
}

campaign_dir() {
  local slug=$1
  printf '%s\n' "$(campaign_repo_root)/.ldgr/campaigns/$slug"
}

campaign_protocol_dir() {
  local slug=$1
  printf '%s\n' "$(campaign_repo_root)/research-campaigns/$slug"
}

campaign_require_slug() {
  local slug=${1:-}
  [[ -n "$slug" ]] || campaign_die "--slug is required"
  [[ "$slug" =~ ^[a-zA-Z0-9._-]+$ ]] || campaign_die "slug may only contain letters, numbers, dots, underscores, and dashes"
}

campaign_load() {
  local slug=$1
  local dir
  dir=$(campaign_dir "$slug")
  [[ -f "$dir/campaign.env" ]] || campaign_die "campaign '$slug' not found at $dir/campaign.env"
  # shellcheck source=/dev/null
  source "$dir/campaign.env"
  CAMPAIGN_DIR="$dir"
  LANES_FILE="$dir/lanes.tsv"
  [[ -f "$LANES_FILE" ]] || campaign_die "missing lanes file: $LANES_FILE"
}

campaign_pid_running() {
  local pid_file=$1
  [[ -s "$pid_file" ]] || return 1
  local pid
  pid=$(cat "$pid_file")
  [[ "$pid" =~ ^[0-9]+$ ]] || return 1
  kill -0 "$pid" 2>/dev/null
}

campaign_safe_source_value() {
  printf '%q' "$1"
}

campaign_write_env() {
  local file=$1
  shift
  : > "$file"
  while (($#)); do
    local key=$1 value=$2
    shift 2
    printf '%s=%q\n' "$key" "$value" >> "$file"
  done
}

campaign_ldgr_init() {
  local worktree=$1
  command -v ldgr-research >/dev/null 2>&1 || campaign_die "ldgr-research is required to initialize campaign lanes"
  (cd "$worktree" && ldgr-research init)
}

campaign_ensure_local_excludes() {
  local repo=$1
  local exclude_file
  exclude_file=$(git -C "$repo" rev-parse --git-path info/exclude)
  mkdir -p "$(dirname "$exclude_file")"
  touch "$exclude_file"
  for pattern in '.ldgr/' '.venv/' '__pycache__/' '.pytest_cache/' '.mypy_cache/' '.ruff_cache/'; do
    grep -Fxq "$pattern" "$exclude_file" || printf '%s\n' "$pattern" >> "$exclude_file"
  done
}

campaign_commit_if_changed() {
  local repo=$1 message=$2
  campaign_ensure_local_excludes "$repo"
  if [[ -n "$(git -C "$repo" status --short)" ]]; then
    git -C "$repo" add -A
    git -C "$repo" commit -m "$message"
  fi
}

campaign_commit_paths() {
  local repo=$1 message=$2
  shift 2
  campaign_ensure_local_excludes "$repo"
  git -C "$repo" add -- "$@"
  if ! git -C "$repo" diff --cached --quiet -- "$@"; then
    git -C "$repo" commit -m "$message"
  fi
}
