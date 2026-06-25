#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=campaign_lib.sh
source "$SCRIPT_DIR/campaign_lib.sh"

usage() {
  cat <<'EOF'
Usage: scripts/campaign_collect.sh --slug SLUG [OPTIONS]

Collect lane LDGR context, git state, optional test output, and a starter
ranking report for a completed campaign.

Options:
  --test-command CMD      Command to run in each lane before collection
  --commit-lanes          Commit changed files in each lane after collection
  --no-report-commit      Do not commit the comparison report in the source repo
  --help                  Show this help
EOF
}

slug=""
test_command=""
commit_lanes=0
commit_report=1

while (($#)); do
  case "$1" in
    --slug) slug=${2:-}; shift 2 ;;
    --test-command) test_command=${2:-}; shift 2 ;;
    --commit-lanes) commit_lanes=1; shift ;;
    --no-report-commit) commit_report=0; shift ;;
    --help|-h) usage; exit 0 ;;
    *) campaign_die "unknown option: $1" ;;
  esac
done

campaign_require_slug "$slug"
campaign_load "$slug"
mkdir -p "$CAMPAIGN_DIR/results"
report_dir=$(campaign_protocol_dir "$slug")
mkdir -p "$report_dir"
comparison="$report_dir/comparison.md"

{
  printf '# Campaign comparison: %s\n\n' "$CAMPAIGN_TITLE"
  printf '## Question\n\n%s\n\n' "$CAMPAIGN_QUESTION"
  printf '## Shared baseline\n\n- Ref: `%s`\n- Commit: `%s`\n\n' "$BASELINE_REF" "$BASELINE_COMMIT"
  printf '## Branches compared\n\n| Branch | Hypothesis | Key artifacts | Status |\n|--------|------------|---------------|--------|\n'
} > "$comparison"

while IFS=$'\t' read -r lane branch worktree hypothesis; do
  [[ "$lane" == "lane" ]] && continue
  [[ -d "$worktree" ]] || campaign_die "missing worktree for lane $lane: $worktree"
  result_dir="$CAMPAIGN_DIR/results/$lane"
  mkdir -p "$result_dir"

  if [[ -n "$test_command" ]]; then
    printf 'running test command for %s: %s\n' "$lane" "$test_command"
    set +e
    (cd "$worktree" && bash -lc "$test_command") > "$result_dir/test.log" 2>&1
    test_status=$?
    set -e
    printf '%s\n' "$test_status" > "$result_dir/test.exit"
  else
    printf 'no test command supplied for %s\n' "$lane" > "$result_dir/test.log"
    printf '0\n' > "$result_dir/test.exit"
  fi

  (cd "$worktree" && ldgr status) > "$result_dir/ldgr-status.txt" 2>&1 || true
  (cd "$worktree" && ldgr context --json) > "$result_dir/ldgr-context.json" 2>&1 || true
  git -C "$worktree" status --short > "$result_dir/git-status.txt"
  git -C "$worktree" log --oneline -n 5 > "$result_dir/git-log.txt"

  if [[ "$commit_lanes" -eq 1 ]]; then
    campaign_commit_if_changed "$worktree" "Collect campaign lane outputs $slug/$lane"
  fi

  test_exit=$(cat "$result_dir/test.exit")
  printf '| `%s` | %s | `.ldgr/campaigns/%s/results/%s/` | test_exit=%s |\n' \
    "$branch" "$hypothesis" "$slug" "$lane" "$test_exit" >> "$comparison"
done < "$LANES_FILE"

cat >> "$comparison" <<'EOF'

## Rubric

Fill from the campaign protocol. Keep criteria comparable across branches.

| Criterion | Weight / priority | Notes |
|-----------|-------------------|-------|
|           |                   |       |

## Results

| Branch | Evidence for | Evidence against | Negative results | Score/rank |
|--------|--------------|------------------|------------------|------------|
|        |              |                  |                  |            |

## Comparative interpretation

Which explanation survived best, which claims were supported/weakened/falsified,
and what uncertainty remains?

## Promotion plan

What commits, files, or artifacts should be cherry-picked into main? What should
remain only as negative evidence?

## Decision and next falsification target

Record the LDGR decision that selects a winner or stops the campaign, then name
the next validation most likely to change the ranking.
EOF

if [[ "$commit_report" -eq 1 ]]; then
  campaign_commit_paths "$(campaign_repo_root)" "Collect research campaign results $slug" "research-campaigns/$slug/comparison.md"
fi

printf 'results collected under: %s/results\n' "$CAMPAIGN_DIR"
printf 'comparison report: %s\n' "$comparison"
