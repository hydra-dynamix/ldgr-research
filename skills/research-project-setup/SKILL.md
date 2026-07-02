---
name: research-project-setup
description: Onboard a project for the Research adapter by inspecting the target, recording setup evidence, initializing durable state, and creating actionable first work.
license: MIT OR Apache-2.0
compatibility: Portable Agent Skills-style package; LDGR-specific contracts are optional in skill.toml.
---

# Research Project Setup

Goal: turn a new or unfamiliar target into a ready-to-run Research adapter project with durable context and clear next work.

Use this skill before running adapter automation on a project that has not been onboarded, or when a fresh agent needs to understand the adapter-specific setup contract.

## Procedure

1. **Preserve the setup prompt.** Record the operator's project/setup request and any target paths as a prompt artifact on the active run.
2. **Inspect the target.** Review the relevant repository, benchmark, research program, or artifact workspace. Record observations with path and command evidence.
3. **Initialize durable state.** Run safe initialization for LDGR and this adapter when needed. Do not overwrite existing state unless the operator explicitly asks.
4. **Initialize the research adapter.** Run `ldgr research init` from the project root, then verify `doctor`, `status`, and `context` work.
5. **Capture setup artifacts.** Attach inventories, config summaries, command outputs, or reports that help future agents continue without rediscovery.
6. **Create first work.** Decompose the setup findings into actionable LDGR work items with validation hints and evidence pointers.
7. **Summarize handoff.** Report created artifacts, observations, work items, active adapter/tooling state, and the recommended next command.

## Adapter focus

Research projects should preserve hypotheses, claims, evidence, open questions, and experiment plans. Favor evidence-linked facts and explicit rejected/unknown claims over broad prose summaries.

## Setup hints

Useful commands: ./install-adapter.sh, ldgr-research init, ldgr-research context, ldgr-research option list, ldgr-research fact list, and ldgr-research work create for experiments or literature/code investigations.

## Validation hints

Check that the research adapter is initialized, the research ledger is initialized, facts have evidence links where appropriate, open questions are represented as questions/options/work, and one concrete next hypothesis is queued.

## Guardrails

- Keep setup idempotent and evidence-backed.
- Do not invent domain requirements beyond the target evidence and adapter contract.
- Do not run destructive commands or broad automation until setup observations and work items exist.
- Prefer small work items that a fresh adapter run can execute independently.
