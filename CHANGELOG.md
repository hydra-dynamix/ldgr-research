# Changelog

## Unreleased

### Added

- `context` now lists **every branch of the program** (status, experiment counts, and for
  non-current branches the branch question and a switch hint) whenever the program has more
  than one. Previously the cockpit was scoped entirely to the current branch, so a session
  resuming on one branch had no signal that work on another existed at all.
- Evidence provenance: `fact add --source primary|secondary|derived` records whether a
  claim rests on a source read directly, on someone else's report of it (tool summaries,
  subagent findings), or on our own computation. A fact resting on `secondary` evidence
  cannot be `accepted` — it must be re-checked against the primary source and promoted
  with `fact update --source primary`. Added after a false claim about a third party's
  documentation, sourced from an automated page summary, reached publication.
- Pre-registration hashes: `experiment create` records a canonical SHA-256 of the
  falsifiable definition (hypothesis, setup, metrics, pass/fail criteria), and the
  new `experiment verdict <slug> --outcome pass|fail|inconclusive` records outcomes
  against the frozen criteria, refusing on definition drift unless `--allow-drift`.
- Adversarial review as a first-class flow: `experiment create --attacks-fact <slug>`
  links an attack experiment to a claim; a `pass` verdict automatically marks the
  attacked fact `contested` and flags it for review.
- `fact impact <slug>`: the downstream blast radius of a fact - attacking
  experiments and slug mentions across facts and experiment definitions.
- Optional review gate: policy `require_review_for_fact_acceptance: true` makes
  `accepted` facts require `--reviewed-by <identity>`; `fact add` now accepts
  `--reviewed-by` and marks the fact reviewed.
- `run note <id> --body` / `run notes <id>`: timestamped incremental evidence
  attached to research runs.
- `--branch` overrides on experiment show/update/complete/submit/verdict, run
  start/list, metric list, artifact list, and decision add - no more
  `branch set-current` dance for cross-branch operations.
- Artifact kinds `log`, `code`, and `data`.

### Changed

- Graph reasoning is now ON by default (policy `graph_reasoning_enabled: true`;
  disable via policy or override with `--enable-graph-reasoning`), and the agent
  guide documents the verdict/adversarial/impact/graph surface.
- `run finish --status failed` is accepted and routed through the fail path.
- `experiment update --status <terminal>` now answers the linked research option, matching
  `experiment complete` and `experiment submit`. Previously two paths reached the same
  terminal state with different side effects, leaving options stale and the "recommended next
  option" pointing at finished work. Updating to `completed` this way also now notes that it
  skips the readiness validation `experiment complete` performs.
- `metric add` accepts negative values without the `--` escape.

### Changed

- Stop shipping the Pi `ldgr-research.ts` extension and remove stale research
  extension copies during install; research prompts and skills remain available
  to both Pi and Codex without injecting harness code.
- Treat a repeated `ldgr research loop run` after a terminal failed agent attempt
  as an explicit retry of the same bounded work item, retaining the failed run
  and artifacts instead of requiring a manual work-state correction.
- Make the research loop prompt explicit about authority ordering, the
  already-started core run, evidence commands, and canonical core closeout.
- Tighten the project-setup skill so validations must actually run and setup
  cannot hand off with an active or decision-pending run.

## [0.1.6] - 2026-07-30

### Added

- Emit the released `research-workflow/v1` numerical experiment sequence through
  Core-owned opt-in telemetry buffering; failed experiments map to the
  completed-negative terminal (`4`) rather than operational failure.

### Changed

- Document that Research inherits Core telemetry controls (`status`, `preview`,
  `transmit`, `enable`, `disable`) and that already-ingested sequences cannot be
  individually located for deletion because collector records have no user,
  installation, request, timestamp, or join identifier.

## [0.1.4] - 2026-07-06

### Changed

- Install research adapter bundles under `~/.ldgr/adapters/research` by default.
- Route the research loop prompt and `research-project-setup` skill through configured harness paths, preserving Pi defaults and supporting Codex prompt/skill roots when Codex is configured.
- Refresh README and installed skill setup docs for harness-aware resource placement.
- Bump package version for the coordinated LDGR 0.1.4 release train.

### Changed

- Reduce routine research-loop ceremony by requiring a compact `run_summary.json`-style artifact and reserving long narrative reports for promotion points.
- Clarify that research loops may promote newly discovered work items when evidence supports the direction.
- Align adapter UX with the conduct-style pattern: `ldgr-research install` installs the adapter bundle plus harness resources, `ldgr-research init` activates the research loop prompt, and docs prefer canonical `ldgr research <command>` dispatch.
- Remove the obsolete `profile discover/apply` command surface; agents install with `install`, initialize with `init`, then use `ldgr research <command>`.
- Add `agent-guide` plus smoke coverage for agent-facing install/init/doctor/status/context and first research-spine commands.
- Add research overlay mode controls plus `ldgr research core <command>` so agents can stay on the research surface while still recording core observations, validations, artifacts, decisions, and run closes.
- Install the research loop prompt and adapter-owned skills into configured harness paths for both `ldgr-research install` and `ldgr adapter install research`.
- Refresh the `research-project-setup` skill so it creates the current research program/branch/question/option/experiment spine, queues one core LDGR work item, and uses the unified `ldgr research` control surface.
- Make `status` and `context` research-focused menus that include core LDGR status when research mode is enabled, and pass through core loop behavior without research prompt injection when research mode is disabled.
- Add an empty workspace table so local source checkouts nested under the LDGR workspace can be built and tested standalone.

## [0.1.1] - 2026-06-30

### Added

- Add repository-local binary release workflow for tagged and manual releases.

### Changed

- Bump package version for the coordinated LDGR release train.
