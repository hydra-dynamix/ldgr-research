# Changelog

## Unreleased

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
