---
name: research-project-setup
description: Onboard a project for the Research adapter by installing/initializing the adapter, creating the initial research program spine, recording setup evidence through the `ldgr research` surface, and queuing one bounded next work item.
license: MIT OR Apache-2.0
compatibility: Portable Agent Skills-style package; LDGR-specific contracts are optional in skill.toml.
---

# Research Project Setup

Goal: turn a new or unfamiliar target into a ready-to-run `ldgr-research` project with one coherent control surface, durable context, a research program/branch/question/option spine, and one bounded next work item for the loop.

Use this skill before running research automation on a project that has not been onboarded, or when a fresh agent needs to understand the current `ldgr-research` setup contract.

## Current control surface

Prefer the canonical adapter dispatch form:

```sh
ldgr research <command>
```

The research surface owns research records and forwards non-conflicting core commands. Use it for both research setup and core evidence whenever possible:

```sh
ldgr research observation add <run-id> --body "<evidence>"
ldgr research validation record <run-id> --outcome pass --command "<command>" --rationale "<why>"
ldgr research work create <slug> --title "<title>" --description "<bounded next work>"
```

For core command names that conflict with research primitives (`run`, `artifact`, `decision`), use the explicit escape hatch:

```sh
ldgr research core run close <run-id> --status success --outcome continue --rationale "<why>"
ldgr research core artifact add <run-id> --kind report --path <path> --description "<description>"
```

`ldgr research mode status` should be enabled for research projects. If the operator is not doing research, disable the overlay with `ldgr research mode disable`.

## Procedure

1. **Preserve the setup request.** Record the operator's request, target paths, and constraints as a core artifact/observation through the research surface. Include `git status --short` when the target is a git repo.
2. **Inspect the target.** Read project instructions (`AGENTS.md`, `CLAUDE.md`, README, benchmark docs, or equivalent). Record observations with command/path evidence.
3. **Install adapter resources if needed.** If the adapter is not installed globally, run `ldgr adapter install research` or `ldgr-research install`. This installs the adapter bundle, the `research-loop` prompt, and adapter-owned skills into the configured harness paths from `~/.ldgr/config.json`.
4. **Initialize project state.** From the project root, run `ldgr research init`, then verify `ldgr research mode status`, `ldgr research doctor`, `ldgr research status`, and `ldgr research context`.
5. **Create the research program spine.** Create/select exactly one initial program, branch, open question, and option/hypothesis that match the setup request. Keep names stable and slug-like.
6. **Create a bounded first experiment when the first hypothesis is known.** Use `ldgr research experiment create ...` for the narrow first test. Prefer exploration mode when the desired observation is not a strict falsification criterion yet.
7. **Queue exactly one loop work item.** The loop consumes core LDGR work, so create one matching core work item via `ldgr research work create ...`. Reference the research program/branch/question/option/experiment slugs in the description.
8. **Record setup validations.** Record `doctor`, `status`, `context`, and any target inventory/check commands with `ldgr research validation record`. Keep rationales compact; do not duplicate command output already captured in artifacts.
9. **Summarize handoff.** Report created research slugs, core work slug, artifacts/observations/validations, current research mode, and the recommended next command (usually `ldgr research loop run --max-iterations 1`). Keep the handoff concise and avoid a full narrative report unless setup changed claims or uncovered a surprising blocker.

## Initial research spine template

Use concrete slugs/titles from the project. Do not leave placeholders in durable records.

```sh
ldgr research program create <program-slug> \
  --title "<program title>" \
  --objective "<one-sentence research objective>"
ldgr research program set-current <program-slug>

ldgr research branch create main \
  --program <program-slug> \
  --title "Main" \
  --question "<central research question>" \
  --rationale "<why this branch is the starting path>"
ldgr research branch set-current main

ldgr research question add <question-slug> \
  --program <program-slug> \
  --branch main \
  --question "<first open question>" \
  --context "<target evidence or setup context>"

ldgr research option add <option-slug> \
  --program <program-slug> \
  --branch main \
  --question <question-slug> \
  --classification validation \
  --description "<bounded candidate hypothesis or setup plan>"

ldgr research experiment create <experiment-slug> \
  --branch main \
  --option <option-slug> \
  --mode exploration \
  --title "<bounded first experiment>" \
  --hypothesis "<what should be true>" \
  --setup "<command or inspection to run>" \
  --observation-goal "<what to observe>"

ldgr research work create <work-slug> \
  --title "<bounded first loop task>" \
  --description "Run experiment <experiment-slug> for program <program-slug>/branch main. Record hypothesis, command output, artifacts, validations, interpretation, and one next work item if needed."
```

If the first experiment is not yet clear, stop after the question/option and create a core work item to define the experiment in one loop cycle.

## Adapter focus

Research projects should preserve hypotheses, claims, evidence, open questions, and experiment plans. Favor evidence-linked facts and explicit rejected/unknown claims over broad prose summaries. Keep core LDGR records authoritative for execution history; use research records as the semantic overlay.

Routine research cycles should be thin and machine-summarizable: one compact `run_summary.json`-style artifact, validation records, a concise decision, and one next work item when needed. Reserve full prose reports for promotion points such as claim graph changes, surprising negative results, operator/model/policy promotion or demotion, external-validity shifts, or milestone synthesis. The goal is to maximize continuity per token/minute.

## Useful commands

```sh
ldgr research init
ldgr research mode status
ldgr research doctor
ldgr research status
ldgr research context
ldgr research agent-guide
ldgr research program list
ldgr research branch list
ldgr research question list
ldgr research option list
ldgr research experiment list
ldgr research observation add <run-id> --body "<evidence>"
ldgr research validation record <run-id> --outcome pass --command "<command>" --rationale "<why>"
ldgr research core artifact add <run-id> --kind report --path <path> --description "<description>"
ldgr research loop run --max-iterations 1
```

## Validation hints

Pass setup when:

- `ldgr research doctor` does not report blocking setup errors;
- `ldgr research status` and `ldgr research context` show the research overlay plus core status;
- `ldgr research mode status` is enabled for research work;
- the configured harness prompt path contains `research-loop.md` after install;
- adapter-owned skills were installed into the configured harness skill path;
- one current program and branch are set;
- at least one open question/option exists;
- exactly one bounded core next work item is queued for the first loop cycle.

## Guardrails

- Keep setup idempotent and evidence-backed.
- Do not create multiple broad pending work items; queue exactly one next bounded task.
- Do not run destructive commands or broad automation until setup observations and the initial spine exist.
- Do not use the removed profile/discover/apply workflow.
- If setup is blocked, record the blocker and queue a small repair task rather than improvising a new workflow.
