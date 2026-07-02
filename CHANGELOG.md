# Changelog

## Unreleased

### Changed

- Align adapter UX with the conduct-style pattern: `ldgr-research install` installs the adapter bundle plus harness resources, `ldgr-research init` activates the research loop prompt, and docs prefer canonical `ldgr research <command>` dispatch.
- Remove the obsolete `profile discover/apply` command surface; agents install with `install`, initialize with `init`, then use `ldgr research <command>`.
- Add `agent-guide` plus smoke coverage for agent-facing install/init/doctor/status/context and first research-spine commands.
- Add research overlay mode controls plus `ldgr research core <command>` so agents can stay on the research surface while still recording core observations, validations, artifacts, decisions, and run closes.
- Make `status` and `context` research-focused menus that include core LDGR status when research mode is enabled, and pass through core loop behavior without research prompt injection when research mode is disabled.
- Add an empty workspace table so local source checkouts nested under the LDGR workspace can be built and tested standalone.

## [0.1.1] - 2026-06-30

### Added

- Add repository-local binary release workflow for tagged and manual releases.

### Changed

- Bump package version for the coordinated LDGR release train.
