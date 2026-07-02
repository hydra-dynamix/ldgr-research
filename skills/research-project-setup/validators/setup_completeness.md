# Setup Completeness Validator

Pass when setup produced:

- prompt artifact with the original request and target path
- adapter install evidence, including centralized prompt location or an explicit reason it was skipped
- `ldgr research init` completed or existing initialized state was verified
- `ldgr research mode status` recorded and enabled for research work
- `ldgr research doctor`, `status`, and `context` recorded as validations or observations
- current research program and branch are set
- at least one open question and one option/hypothesis exist
- first experiment exists when the initial test is already known, or the next work item explicitly asks to define it
- exactly one actionable core LDGR next work item exists with validation hints and research slug references
- a concise handoff summary states the recommended next command

Fail or mark partial when setup leaves multiple broad pending tasks, uses the removed profile/discover/apply workflow, or requires agents to switch between unrelated control surfaces without documenting why.
