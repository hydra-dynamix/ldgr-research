# ldgr-research

`ldgr-research` is the alpha research adapter for LDGR. It provides one command surface for two things:

- research-specific records stored in `.ldgr/research/research.db`; and
- research-oriented access to core `ldgr` commands such as `status`, `work`, `run`, and `loop`.

The adapter is publication-ready as an alpha: workflows and schemas may still evolve, but the canonical install/init/loop path is intended to be usable by agents and humans.

The research layer uses a proven workflow: programs contain branches, branches contain selectable research options/hypotheses, and selected options become bounded experiments with runs, metrics, artifacts, decisions, facts, and follow-up options.

## Install from GitHub

```sh
cargo install --git https://github.com/hydra-dynamix/ldgr-research ldgr-research
```

Install the adapter bundle and use the canonical LDGR dispatch surface:

```sh
ldgr-research install
ldgr research --help
ldgr research init
ldgr research status
ldgr research loop run
```

This follows the `ldgr-conduct` adapter pattern: the adapter binary owns install/init/resources/workflows, while LDGR core owns adapter discovery and dispatch through `adapter.toml`. `ldgr-research adapter install` is the installer entrypoint used by LDGR core; humans can run `ldgr-research install`.

`ldgr-research install` materializes adapter resources under `~/.ldgr/research`, copies the research prompt to the centralized prompt directory `~/.ldgr/prompts/research-loop.md`, and installs adapter-owned skills into configured harness skill paths. `ldgr-research init` initializes project research state and imports/activates the `research-loop` prompt in the project core LDGR database. `ldgr-research loop run` and `ldgr research loop run` forward to `ldgr loop run` and automatically supply `--prompt-slug research-loop` when research mode is enabled and no explicit prompt source is provided.

## Research overlay mode

Research mode is enabled by default after `init`. In research mode, `ldgr research status` and `ldgr research context` show research-focused menus with core LDGR status embedded, and `ldgr research loop run` uses the active `research-loop` prompt by default.

```sh
ldgr research mode status
ldgr research mode disable  # stop using research defaults in this project
ldgr research mode enable
```

Most non-conflicting core commands can be run through the same surface:

```sh
ldgr research observation add <run-id> --body "<evidence>"
ldgr research validation record <run-id> --outcome pass --command "<command>" --rationale "<why>"
ldgr research work create <slug> --title "<title>" --description "<bounded next work>"
```

For command names that conflict with research primitives (`run`, `artifact`, `decision`), use the explicit core escape hatch:

```sh
ldgr research core run close <run-id> --status success --outcome continue --rationale "<why>"
ldgr research core artifact add <run-id> --kind report --path <path> --description "<description>"
```

## Agent quickstart

Agents should start from the project root with:

```sh
ldgr research init
ldgr research agent-guide
ldgr research doctor
ldgr research status
ldgr research context
```

`agent-guide` prints copy-pasteable commands for creating the initial program/branch/question/option spine, recording core evidence through the research surface, using `ldgr research core` for conflicting core commands, and running guard/lint checks.

## State layout

```text
.ldgr/ldgr.db                     # core LDGR work/run ledger
.ldgr/artifacts/                  # core LDGR managed artifacts
.ldgr/research/research.db        # research primitives
.ldgr/research/policy.yaml        # research policy/current program/branch
.ldgr/research/tools.yaml         # reusable research tool registry
~/.ldgr/prompts/research-loop.md  # centralized installed research loop prompt
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
ldgr-research observation add 7 --body "Evidence from this run"
ldgr-research validation record 7 --outcome pass --command "cargo test" --rationale "Tests passed"
ldgr-research work create next-hypothesis --title "Next hypothesis" --description "..."
ldgr-research loop run --max-iterations 3
```

For conflicting command names, use the explicit core escape hatch:

```sh
ldgr-research core run close 7 --status success --outcome continue --rationale "..."
ldgr-research core artifact add 7 --kind report --path output.txt --description "Transcript"
```

For loop runs, `ldgr-research` injects the active research prompt by default when research mode is enabled:

```sh
ldgr-research loop run
# forwards to: ldgr loop run --prompt-slug research-loop

ldgr-research mode disable
ldgr-research loop run
# forwards to: ldgr loop run
```

Explicit prompt sources are preserved:

```sh
ldgr-research loop run --prompt custom.md
ldgr-research loop run --bundle cleanroom --prompt-role research-loop
```

## Adapter commands

```sh
ldgr-research install [--adapter-root DIR | --install-root DIR] [--print-path]
ldgr-research adapter install [--adapter-root DIR | --install-root DIR] [--print-path]
ldgr-research init
ldgr research <command> [options]
ldgr research agent-guide
ldgr research mode <status|enable|disable>
ldgr research core <ldgr-command>
```

By default, adapter bundle files are materialized under `LDGR_HOME/research` or `~/.ldgr/research`. Install also copies prompt files into `LDGR_HOME/prompts` or `~/.ldgr/prompts`, and copies adapter-owned skills into configured harness skill paths from `~/.ldgr/config.json`, defaulting to `~/.pi/agent/skills` when no harness config is present. The same bundle layout is used by `ldgr adapter install research`, so core adapter installation also installs prompts and skills.

There is no separate profile step. Install the adapter once, initialize each project with `ldgr research init`, then use the canonical `ldgr research <command>` control surface.

## Campaign workflow

For multi-lane branch races, see [`docs/research-campaign-process.md`](docs/research-campaign-process.md). The campaign scripts create worktrees, initialize the research adapter in each lane, run bounded loops, collect lane status/context, and generate a comparison report.

## Development

```sh
cargo fmt --all -- --check
cargo test -p ldgr-research
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
