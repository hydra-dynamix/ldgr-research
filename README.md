# ldgr-research

`ldgr-research` is the alpha research adapter for LDGR. It provides one command surface for two things:

- research-specific records stored in `.ldgr/research/research.db`; and
- research-oriented access to core `ldgr` commands such as `status`, `work`, `run`, and `loop`.

The adapter is publication-ready as an alpha: workflows and schemas may still evolve, but the canonical install/init/loop path is intended to be usable by agents and humans.

The research layer uses a proven workflow: programs contain branches, branches contain selectable research options/hypotheses, and selected options become bounded experiments with runs, metrics, artifacts, decisions, facts, and follow-up options.

## Canonical installation

Start with LDGR Core and install Research through Core's authenticated adapter
catalog and discovery surface:

```sh
ldgr adapter install research
ldgr research install
ldgr research workflow
ldgr research init
ldgr research doctor
```

Do not install or invoke a standalone `ldgr-research` binary to bootstrap a
missing/stale bundle. `ldgr adapter install research` authenticates the signed
catalog, selects a compatibility-v2 artifact for the active Core profile,
verifies the archive and staged sidecar, writes the receipt, and registers an
absolute adapter command. `ldgr research install` then idempotently materializes
the Research-owned prompt and skills through that discovered command. It must
not rewrite Core's signed-release receipt.

Run `workflow` before project mutation to read the installed adapter workflow.
`init` creates or migrates `.ldgr/research/research.db` and imports/activates the
`research-loop` prompt in `.ldgr/ldgr.db`; `doctor` verifies both stores and the
configuration. Repeat `init` for each project. There is no separate profile
step.

Research follows the `ldgr-conduct` ownership pattern: the adapter owns its
install/init/resources/workflow behavior and local Research database, while
Core owns catalog verification, bundle discovery, compatibility evaluation, and
dispatch through `adapter.toml`. The adapter copies resources into configured
harness paths in canonical `~/.ldgr/config.toml` (with `config.json` retained as
a legacy mirror). With no harness config, Pi-compatible defaults are
`~/.ldgr/prompts` and `~/.pi/agent/skills`. The adapter does not install harness
extensions and removes stale `ldgr-research.ts` copies from older releases.

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

After the canonical installation, agents should start from the project root
with:

```sh
ldgr research workflow
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
~/.ldgr/prompts/research-loop.md  # default installed research loop prompt
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

## Numerical workflow sequences

When Core numerical telemetry is explicitly enabled, terminal experiment status
changes are submitted through LDGR Core's sequence buffer at
`/sequences/research-workflow/v1`. The adapter does not prompt for consent,
inspect the Core consent file, or open a telemetry network connection. Core owns
local buffering, preview, HTTPS transmission, disablement, and the
`LDGR_TELEMETRY=off` kill switch.

The released Research workflow alphabet records only the committed experiment
state path: planned (`0`), running (`1`), completed (`3`), failed research
finding (`4`), inconclusive (`5`), operational failure (`6`), and
superseded/cancelled (`7`). A failed experiment is treated as a
completed-negative counterexample, not operational failure. Code `2` (`held`) is
not declared for this endpoint.

Use `ldgr telemetry status` and `ldgr telemetry preview` to inspect the effective
Core decision and exact pending Research arrays, and use
`ldgr telemetry transmit --collector https://...` to submit them best-effort to
an HTTPS collector. `ldgr telemetry disable` deletes unsent local Research
payloads and sends no network event. Already-ingested sequences cannot be
individually located for deletion because accepted collector records have no
user, installation, request, timestamp, or join identifier.

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

If an agent attempt exits unsuccessfully and leaves its work item in the core
decision-pending state, running `ldgr research loop run` again explicitly retries
that same bounded work item. The failed run and its artifacts remain in the
ledger; the adapter only restores the work item to pending before forwarding the
new loop invocation. A genuinely active run is never replaced this way.

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

By default, Core materializes the adapter bundle under
`LDGR_HOME/adapters/research` or `~/.ldgr/adapters/research`. The adapter
installer copies prompt files and adapter-owned skills into configured harness
paths. Codex harness entries use `~/.codex/prompts` and `~/.codex/skills`; Pi
defaults use `~/.ldgr/prompts` and `~/.pi/agent/skills`.

The direct `ldgr-research install` and `ldgr-research adapter install` entrypoints
remain for adapter development and for Core to invoke internally. They are not
the supported first-install or repair path. Operators install the adapter once
with `ldgr adapter install research`, initialize each project with
`ldgr research init`, then use the canonical `ldgr research <command>` control
surface.

## Compatibility and repair

Research's compatibility-v2 sidecar declares protocol epoch 1, minimum Core
schema 5, Core capabilities `prompt.v1`, `telemetry.v1`, and `work.v1`, no
central component, and one adapter-local SQLite store. Its local migration
digest and schema version describe `.ldgr/research/research.db`; they are not
part of the central database contract or the release-index compatibility
fingerprint. A Research-local schema change therefore does not require exact
global release-set identity and cannot invalidate an unrelated adapter.

Diagnose an unavailable namespace through Core:

```sh
ldgr adapter list
ldgr adapter show research --json
ldgr update --adapter research
ldgr adapter show research --json
```

A valid v2 install is `ready`. An exact historical v1 install may be `degraded`
with a warning. Stale v1 metadata remains visible as `blocked` with
`compatibility.legacy_global_contract_mismatch` or
`compatibility.legacy_core_schema_mismatch`; malformed metadata is `invalid`.
Use the exact `repair.command` emitted by Core. Do not copy a global hash into
`adapter-database-contract.json`, delete the v2 sidecar to force legacy
fallback, or run a standalone binary to rewrite the installed bundle.

After repair, run the complete canonical sequence again. `ldgr research install`
is idempotent, and `ldgr research doctor` verifies the adapter-owned local
migration and Core-owned prompt activation.

## Campaign workflow

For multi-lane branch races, see [`docs/research-campaign-process.md`](docs/research-campaign-process.md). The campaign scripts create worktrees, initialize the research adapter in each lane, run bounded loops, collect lane status/context, and generate a comparison report.

## Development

```sh
cargo fmt --all -- --check
cargo test -p ldgr-research
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
