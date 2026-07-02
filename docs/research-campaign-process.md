# Research campaign process

A research campaign is the first-class process for comparing competing
explanations without losing negative evidence. It is intentionally implemented as
repo scripts plus LDGR artifacts before adding native `ldgr-research campaign`
schema or commands.

Use this when a question has multiple plausible lanes and the valuable output is
not only the winner, but also which claims were weakened, falsified, or left
open.

## Files and directories

```text
scripts/campaign_create.sh    # protocol + worktrees + lane LDGR setup
scripts/campaign_launch.sh    # one ldgr-research loop process per lane
scripts/campaign_status.sh    # PID/log/LDGR status watcher
scripts/campaign_collect.sh   # tests + lane ledger snapshots + comparison report
scripts/campaign_cleanup.sh   # archive + remove worktrees

research-campaigns/<slug>/protocol.md      # committed campaign protocol
research-campaigns/<slug>/comparison.md    # committed ranking/decision starter
.ldgr/campaigns/<slug>/campaign.env        # local campaign metadata
.ldgr/campaigns/<slug>/lanes.tsv           # local lane registry
.ldgr/campaigns/<slug>/logs/<lane>.log      # loop logs
.ldgr/campaigns/<slug>/pids/<lane>.pid      # launched process ids
.ldgr/campaigns/<slug>/results/<lane>/      # collected status/test snapshots
```

`.ldgr/` remains local and ignored; commit the protocol and comparison reports.
Lane worktrees have their own `.ldgr/` stores and branch commits.

## 1. Create campaign

Define:

- slug and title;
- question under test;
- baseline ref/commit;
- branch lanes and hypotheses;
- scoring rubric;
- validation requirements.

Example:

```sh
scripts/campaign_create.sh \
  --slug near-family-mitigation \
  --title "Near-family mitigation race" \
  --question "Which mitigation best preserves recall while reducing near-family false positives?" \
  --baseline main \
  --lane prototype-formation:"Use label-free prototype grouping as the mitigation" \
  --lane sequence-prediction:"Use successor ambiguity to disambiguate near-family traces" \
  --lane contrastive-cleanup:"Use contrastive cleanup to separate near-family embeddings" \
  --rubric "Must improve held-out near-family separation without supplied family IDs" \
  --rubric "Must preserve clean-corpus recall within the agreed tolerance" \
  --validation "Run the shared regression and near-family evaluation suite" \
  --max-iterations 8 \
  --commit-protocol
```

What the script does:

1. writes `.ldgr/campaigns/<slug>/campaign.env` and `lanes.tsv`;
2. writes `research-campaigns/<slug>/protocol.md`;
3. creates one git branch/worktree per lane from the baseline commit;
4. initializes LDGR and initializes the research adapter in each worktree;
5. writes `BRANCH_TASK.md` in each lane;
6. creates one lane work item per worktree;
7. commits lane initialization in each lane;
8. optionally commits the protocol doc in the source repo.

## 2. Launch lanes

Run bounded autonomous loops per lane:

```sh
scripts/campaign_launch.sh --slug near-family-mitigation --agent agentctl --max-iterations 8
```

Each lane runs:

```sh
ldgr-research loop run --agent agentctl --max-iterations N
```

Logs and PIDs are stored under `.ldgr/campaigns/<slug>/logs/` and
`.ldgr/campaigns/<slug>/pids/`.

## 3. Watch status

Use the status script as the simple watcher. It reports PID state, LDGR status,
and log tails per lane:

```sh
scripts/campaign_status.sh --slug near-family-mitigation --tail 20
```

A Pi extension can wrap the same files: read lane PIDs/logs and wake the user or
agent when all lanes are stopped.

## 4. Collect results

After all lanes stop, run shared validation and capture lane ledger state:

```sh
scripts/campaign_collect.sh \
  --slug near-family-mitigation \
  --test-command "uv run pytest -q" \
  --commit-lanes
```

The collect script stores, per lane:

- test output and exit code;
- `ldgr status`;
- `ldgr context --json`;
- recent git log;
- git status.

It also creates `research-campaigns/<slug>/comparison.md`, a starter ranking
report based on `templates/campaign-comparison.md`.

## 5. Promote

Promotion is intentionally manual until the artifact convention stabilizes.
After reviewing `comparison.md` and each lane ledger:

1. choose winner/ranking;
2. cherry-pick selected commits or copy selected artifacts into `main`;
3. preserve non-winning lanes as negative evidence, not as trash;
4. record a source-repo LDGR decision containing:
   - result summary;
   - comparative interpretation;
   - limitations;
   - confidence;
   - next falsification target.

Example:

```sh
git cherry-pick campaign/near-family-mitigation/prototype-formation~2..campaign/near-family-mitigation/prototype-formation
ldgr-research decision add <experiment> \
  --decision continue \
  --confidence medium \
  --result "prototype formation ranked first ..." \
  --interpretation "..." \
  --limitations "..." \
  --propose-option validate-prototype-under-interference@validation:"Run the next falsification target identified by the campaign comparison."
```

## 6. Cleanup

Archive local campaign state and remove worktrees:

```sh
scripts/campaign_cleanup.sh --slug near-family-mitigation --prune-python
```

By default branches are kept. Delete lane branches only when their commits are
merged, copied, or intentionally abandoned:

```sh
scripts/campaign_cleanup.sh --slug near-family-mitigation --delete-branches --prune-python
```

The cleanup script archives metadata/logs/results to
`.ldgr/campaigns/<slug>/archive/` before removing worktrees.

## Notes for future native commands

If this pattern stabilizes, these scripts should become `ldgr-research` adapter
commands backed by LDGR records resembling:

```sh
ldgr-research campaign create <slug> --title ... --baseline ... --rubric ...
ldgr-research campaign branch add <campaign> <branch> --worktree ... --hypothesis ...
ldgr-research campaign branch result <campaign> <branch> --summary ... --artifact-id ...
ldgr-research campaign compare <campaign>
ldgr-research campaign render <campaign>
```

Do not add core schema until the artifact-backed process proves which fields are
stable across real campaigns.
