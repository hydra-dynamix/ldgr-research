# Setup Summary Example

Created:

- Prompt artifact preserving the setup request and target path.
- Observations covering adapter installation, centralized prompt location, research mode, and target inventory.
- Current research program and branch.
- First open question and candidate option/hypothesis.
- First bounded experiment when the initial test was clear.
- Exactly one core LDGR work item queued through `ldgr research work create ...`.
- Validations for `ldgr research doctor`, `status`, `context`, and any target-specific smoke check.

Recommended next command:

```sh
ldgr research loop run --max-iterations 1
```

If a conflicting core command is needed during the next cycle, use `ldgr research core <ldgr-command>` rather than switching control surfaces.
