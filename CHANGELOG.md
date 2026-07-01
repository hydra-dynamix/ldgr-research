# Changelog

## Unreleased

### Changed

- Align adapter UX with the conduct-style pattern: `ldgr-research install` installs the adapter bundle plus harness resources, `ldgr-research init` activates the research loop prompt, and docs prefer canonical `ldgr research <command>` dispatch. Legacy `profile discover/apply` remains for compatibility.
- Add an empty workspace table so local source checkouts nested under the LDGR workspace can be built and tested standalone.

## [0.1.1] - 2026-06-30

### Added

- Add repository-local binary release workflow for tagged and manual releases.

### Changed

- Bump package version for the coordinated LDGR release train.
