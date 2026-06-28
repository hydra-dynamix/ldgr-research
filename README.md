# ldgr-research

`ldgr-research` is the research adapter for LDGR. It provides one command surface for two things:

- research-specific records stored in `.ldgr/research/research.db`; and
- pass-through access to core `ldgr` commands such as `status`, `work`, `run`, and `loop`.

The research layer uses a proven workflow: programs contain branches, branches contain selectable research options/hypotheses, and selected options become bounded experiments with runs, metrics, artifacts, decisions, facts, and follow-up options.

## Install from GitHub

```sh
cargo install --git https://github.com/hydra-dynamix/ldgr-research ldgr-research
```

From a project directory:

```sh
ldgr-research init
ldgr-research adapter install
ldgr-research profile apply research
ldgr-research status
ldgr-research loop run
```

`ldgr-research loop run` forwards to `ldgr loop run` and automatically supplies `--prompt-slug research-loop` when no prompt source is provided.

## State layout

```text
.ldgr/ldgr.db                     # core LDGR work/run ledger
.ldgr/artifacts/                  # core LDGR managed artifacts
.ldgr/research/research.db        # research primitives
.ldgr/research/policy.yaml        # research policy/current program/branch
.ldgr/research/tools.yaml         # reusable research tool registry
```

## Core workflow

A typical cycle is:

1. create/select a research option or hypothesis;
2. create one experiment from that option;
3. start a run;
4. record metrics and artifacts;
5. finish the run;
6. add an interpreted decision;
7. record facts/evidence and proposed next options;
8. complete or supersede the experiment;
9. let the next fresh loop cycle pick up the next bounded hypothesis.

Example:

```sh
ldgr-research program create demo \
  --title "Demo program" \
  --objective "Validate one research hypothesis at a time"
ldgr-research program set-current demo

ldgr-research branch create main \
  --program demo \
  --title "Main branch" \
  --question "Which explanation survives testing?" \
  --rationale "Initial research direction"
ldgr-research branch set-current main

ldgr-research option add hyp-1 \
  --program demo \
  --branch main \
  --title "First hypothesis" \
  --description "Test the first bounded explanation" \
  --classification validation \
  --hypothesis "The first explanation predicts the measured result"
ldgr-research option select hyp-1 --by agent --rationale "Best next falsification target"

ldgr-research experiment create exp-1 \
  --branch main \
  --option hyp-1 \
  --mode falsification \
  --title "First experiment" \
  --hypothesis "The first explanation predicts the measured result" \
  --setup "Run the narrow validation command" \
  --primary-metric exit_code \
  --pass "exit code is zero" \
  --fail "exit code is nonzero" \
  --allowed-next "queue one concrete follow-up hypothesis" \
  --blocked-next "broad placeholder work"
ldgr-research experiment update exp-1 --status running

run_output=$(ldgr-research run start exp-1 --command "cargo test")
run_id=$(printf '%s\n' "$run_output" | awk '/started run/ {print $3}')

ldgr-research metric add "$run_id" exit_code 0 --unit code --split local
ldgr-research artifact add "$run_id" output/results.json --kind json --description "Experiment results" --checksum
ldgr-research run finish "$run_id" --status success --notes "Validation command passed"

ldgr-research decision add exp-1 \
  --decision continue \
  --confidence medium \
  --result "The expected result was observed" \
  --interpretation "The hypothesis is supported for this narrow setup" \
  --limitations "Only one local validation was run" \
  --propose-option next-check@validation:"Test the next falsification target"

ldgr-research fact add hyp-1-supported \
  --program demo \
  --statement "The first hypothesis was supported in the local validation" \
  --status accepted \
  --evidence-experiment exp-1

ldgr-research experiment complete exp-1
```

## Research primitives

`ldgr-research` includes these research primitives:

- `program`
- `branch`
- `option`
- `experiment`
- `run`
- `metric`
- `artifact`
- `decision`
- `question`
- `fact`
- `axiom`
- `review`
- `override`
- `bug`
- `matrix`
- `tool`
- `graph`
- `dashboard`
- `hypothesis`
- `tree`, `show`, `report`, `export`, `guard`, `lint`, `migrate`, `doctor`

Use `ldgr-research <command> --help` for exact flags.

## LDGR pass-through

Any non-research command is forwarded to `ldgr`:

```sh
ldgr-research status
ldgr-research work create next-hypothesis --title "Next hypothesis" --description "..."
ldgr-research loop run --max-iterations 3
```

For loop runs, `ldgr-research` injects the active research prompt by default:

```sh
ldgr-research loop run
# forwards to: ldgr loop run --prompt-slug research-loop
```

Explicit prompt sources are preserved:

```sh
ldgr-research loop run --prompt custom.md
ldgr-research loop run --bundle cleanroom --prompt-role research-loop
```

## Adapter/profile commands

```sh
ldgr-research adapter install [--adapter-root DIR | --install-root DIR] [--print-path]
ldgr-research profile discover
ldgr-research profile apply [research] [--install-root DIR] [--ldgr-db PATH] [--ldgr-artifact-root DIR] [--materialize-only]
```

By default, adapter bundle files are materialized under `.ldgr/.research/`. `profile apply` installs or updates the `research-loop` prompt in the target LDGR ledger and marks it active. `--materialize-only` copies adapter files without touching a ledger.

## Campaign workflow

For multi-lane branch races, see [`docs/research-campaign-process.md`](docs/research-campaign-process.md). The campaign scripts create worktrees, apply the research profile in each lane, run bounded loops, collect lane status/context, and generate a comparison report.

## Development

```sh
cargo fmt --all -- --check
cargo test -p ldgr-research
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
