#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=campaign_lib.sh
source "$SCRIPT_DIR/campaign_lib.sh"

usage() {
  cat <<'EOF'
Usage: scripts/campaign_create.sh --slug SLUG --title TITLE --question QUESTION --lane SLUG:HYPOTHESIS [--lane ...] [OPTIONS]

Create a research campaign protocol, one git worktree per lane, initialize LDGR
with the research adapter initialized in each lane, add BRANCH_TASK.md, and create a lane
work item.

Options:
  --baseline REF          Baseline branch/commit for all lanes [default: HEAD]
  --worktree-root DIR     Root for lane worktrees [default: ../<repo>-campaign-worktrees]
  --rubric TEXT           Scoring rubric text; may be repeated
  --validation TEXT       Shared validation requirement; may be repeated
  --max-iterations N      Default launch iteration count recorded in metadata [default: 1]
  --commit-protocol       Commit research-campaigns/<slug>/protocol.md in the source repo
  --help                  Show this help

Lane format:
  --lane prototype-formation:"Test label-free prototype grouping"
EOF
}

slug=""
title=""
question=""
baseline="HEAD"
worktree_root=""
max_iterations="1"
commit_protocol=0
lanes=()
rubrics=()
validations=()

while (($#)); do
  case "$1" in
    --slug) slug=${2:-}; shift 2 ;;
    --title) title=${2:-}; shift 2 ;;
    --question) question=${2:-}; shift 2 ;;
    --baseline) baseline=${2:-}; shift 2 ;;
    --worktree-root) worktree_root=${2:-}; shift 2 ;;
    --lane) lanes+=("${2:-}"); shift 2 ;;
    --rubric) rubrics+=("${2:-}"); shift 2 ;;
    --validation) validations+=("${2:-}"); shift 2 ;;
    --max-iterations) max_iterations=${2:-}; shift 2 ;;
    --commit-protocol) commit_protocol=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) campaign_die "unknown option: $1" ;;
  esac
done

campaign_require_slug "$slug"
[[ -n "$title" ]] || campaign_die "--title is required"
[[ -n "$question" ]] || campaign_die "--question is required"
((${#lanes[@]} > 0)) || campaign_die "at least one --lane is required"
[[ "$max_iterations" =~ ^[0-9]+$ ]] || campaign_die "--max-iterations must be an integer"

repo_root=$(campaign_repo_root)
campaign_ensure_local_excludes "$repo_root"
baseline_commit=$(git rev-parse "$baseline")
worktree_root=${worktree_root:-$(campaign_default_worktree_root)/$slug}
cdir=$(campaign_dir "$slug")
pdir=$(campaign_protocol_dir "$slug")
mkdir -p "$cdir/logs" "$cdir/results" "$pdir" "$worktree_root"

if [[ -e "$cdir/campaign.env" ]]; then
  campaign_die "campaign already exists: $cdir"
fi

campaign_write_env "$cdir/campaign.env" \
  CAMPAIGN_SLUG "$slug" \
  CAMPAIGN_TITLE "$title" \
  CAMPAIGN_QUESTION "$question" \
  BASELINE_REF "$baseline" \
  BASELINE_COMMIT "$baseline_commit" \
  WORKTREE_ROOT "$worktree_root" \
  MAX_ITERATIONS "$max_iterations" \
  CREATED_AT "$(date -u +%Y-%m-%dT%H:%M:%SZ)"

{
  printf 'lane\tbranch\tworktree\thypothesis\n'
  for lane_spec in "${lanes[@]}"; do
    lane=${lane_spec%%:*}
    hypothesis=${lane_spec#*:}
    [[ "$lane" != "$lane_spec" && -n "$lane" && -n "$hypothesis" ]] || campaign_die "lane must be SLUG:HYPOTHESIS: $lane_spec"
    campaign_require_slug "$lane"
    branch="campaign/$slug/$lane"
    worktree="$worktree_root/$lane"
    printf '%s\t%s\t%s\t%s\n' "$lane" "$branch" "$worktree" "$hypothesis"
  done
} > "$cdir/lanes.tsv"

protocol="$pdir/protocol.md"
{
  printf '# Research campaign: %s\n\n' "$title"
  printf '## Campaign metadata\n\n'
  printf -- '- Slug: `%s`\n' "$slug"
  printf -- '- Baseline ref: `%s`\n' "$baseline"
  printf -- '- Baseline commit: `%s`\n' "$baseline_commit"
  printf -- '- Worktree root: `%s`\n\n' "$worktree_root"
  printf '## Question\n\n%s\n\n' "$question"
  printf '## Branch lanes\n\n| Lane | Branch | Hypothesis |\n|------|--------|------------|\n'
  tail -n +2 "$cdir/lanes.tsv" | while IFS=$'\t' read -r lane branch _worktree hypothesis; do
    printf '| `%s` | `%s` | %s |\n' "$lane" "$branch" "$hypothesis"
  done
  printf '\n## Scoring rubric\n\n'
  if ((${#rubrics[@]})); then
    for rubric in "${rubrics[@]}"; do printf -- '- %s\n' "$rubric"; done
  else
    printf -- '- Define objective criteria before launch.\n'
  fi
  printf '\n## Validation requirements\n\n'
  if ((${#validations[@]})); then
    for validation in "${validations[@]}"; do printf -- '- %s\n' "$validation"; done
  else
    printf -- '- Define shared checks before collecting results.\n'
  fi
  printf '\n## Operating process\n\n'
  printf '1. Launch lanes with `scripts/campaign_launch.sh --slug %s`.\n' "$slug"
  printf '2. Watch with `scripts/campaign_status.sh --slug %s`.\n' "$slug"
  printf '3. Collect with `scripts/campaign_collect.sh --slug %s --test-command "<cmd>"`.\n' "$slug"
  printf '4. Record comparative decision and next falsification target.\n'
} > "$protocol"

while IFS=$'\t' read -r lane branch worktree hypothesis; do
  [[ "$lane" == "lane" ]] && continue
  if [[ -d "$worktree/.git" || -f "$worktree/.git" ]]; then
    printf 'worktree exists, skipping create: %s\n' "$worktree"
  else
    git worktree add -b "$branch" "$worktree" "$baseline_commit"
  fi
  campaign_ensure_local_excludes "$worktree"
  campaign_ldgr_init "$worktree"
  cat > "$worktree/BRANCH_TASK.md" <<EOF
# Campaign lane: $lane

Campaign: $slug
Question: $question
Baseline: $baseline_commit
Branch: $branch

## Hypothesis

$hypothesis

## Shared process

- Use ldgr-research as the durable source of truth.
- Record the first run observation as hypothesis/setup/goal.
- Preserve negative results as progress.
- Attach artifacts for experiments, claims, and branch results.
- Finish with a decision containing result summary, interpretation, limitations, and confidence.
EOF
  (cd "$worktree" && ldgr-research work show "$slug-$lane" >/dev/null 2>&1) || \
    (cd "$worktree" && ldgr-research work create "$slug-$lane" \
      --title "[main-path] Campaign $slug lane $lane" \
      --description "Test branch hypothesis for campaign '$slug': $hypothesis. Baseline: $baseline_commit. Record evidence, negative results, and branch decision for collection.")
  campaign_commit_paths "$worktree" "Initialize campaign lane $slug/$lane" BRANCH_TASK.md
done < "$cdir/lanes.tsv"

if [[ "$commit_protocol" -eq 1 ]]; then
  campaign_commit_paths "$repo_root" "Create research campaign protocol $slug" "research-campaigns/$slug/protocol.md"
else
  printf 'protocol written but not committed: %s\n' "$protocol"
  printf 'commit with: git add %q && git commit -m %q\n' "research-campaigns/$slug/protocol.md" "Create research campaign protocol $slug"
fi

printf 'campaign created: %s\n' "$slug"
printf 'metadata: %s\n' "$cdir"
printf 'worktrees: %s\n' "$worktree_root"
