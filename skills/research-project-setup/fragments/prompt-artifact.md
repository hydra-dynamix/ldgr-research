# Setup Prompt Artifact

Save the original setup request before modifying project state.

Include:

- target path and adapter name (`research`)
- original prompt, quoted verbatim
- current git/status summary when applicable
- existing core LDGR state (`ldgr research core status` or `ldgr status`)
- existing research state (`ldgr research status` / `ldgr research context` when initialized)
- research mode state (`ldgr research mode status` when initialized)
- centralized prompt/skill install state when relevant
- explicit operator constraints

Record the artifact through the research surface. If the core `artifact` command is needed, use `ldgr research core artifact add ...`.
