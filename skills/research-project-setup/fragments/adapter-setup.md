# Adapter Setup Fragment

Use the single research control surface unless a command name conflicts with a research primitive.

1. Confirm adapter installation. If missing, run `ldgr adapter install research` or `ldgr-research install`.
2. Confirm centralized prompt installation: `~/.ldgr/prompts/research-loop.md` or `$LDGR_HOME/prompts/research-loop.md` should exist after install.
3. Confirm adapter-owned skills were copied to configured harness skill paths, or record the harness limitation.
4. From the project root, run `ldgr research init` without erasing existing state.
5. Verify `ldgr research mode status`, `ldgr research doctor`, `ldgr research status`, and `ldgr research context`.
6. Record setup evidence through `ldgr research observation` / `ldgr research validation`.

Use `ldgr research core <ldgr-command>` only for core commands whose names conflict with research primitives, for example core `run`, `artifact`, or `decision` commands.
