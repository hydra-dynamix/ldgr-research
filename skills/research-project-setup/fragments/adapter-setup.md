# Adapter Setup Fragment

Use the single research control surface unless a command name conflicts with a research primitive.

1. Confirm adapter installation. If missing, run `ldgr adapter install research` or `ldgr-research install`.
2. Confirm prompt installation in the configured harness prompt path; Codex uses `~/.codex/prompts/research-loop.md`, while the Pi-compatible default is `~/.ldgr/prompts/research-loop.md`.
3. Confirm adapter-owned skills were copied to the configured harness skill path.
4. From the project root, run `ldgr research init` without erasing existing state.
5. Verify `ldgr research mode status`, `ldgr research doctor`, `ldgr research status`, and `ldgr research context`.
6. Record setup evidence through `ldgr research observation` / `ldgr research validation`.

Use `ldgr research core <ldgr-command>` only for core commands whose names conflict with research primitives, for example core `run`, `artifact`, or `decision` commands.
