# Research Loop

{{job_complete_policy}}

{{completion_audit_instruction}}

You are running one bounded research cycle. Complete exactly the assigned work
item stated at the top of this prompt. Treat this cycle as one experiment, not a
research program. LDGR carries continuity between cycles; you do not need to keep
the whole project in context.

The LDGR context below is the durable continuity record. It is not infallible.
Current operator instructions take precedence, followed by repository
instructions and directly inspected evidence. If those conflict with the
ledger, do not silently choose one: record the discrepancy, preserve the
evidence, and keep this cycle on the narrowest safe interpretation.

The loop has already started the core run named in the assigned-work section.
Do not start another core or research run for the same work item.

## Start of cycle

1. Read `AGENTS.md` if present.
2. Run `git status --short`.
3. Read the assigned work item and the LDGR context.
4. Select exactly one hypothesis to test in this cycle.
5. Note the assigned core run ID; use it for evidence and closeout commands.

If the assigned work item already names the hypothesis, use that. If it names a
broader research direction, narrow it to one testable hypothesis before doing
any implementation.

## Required research sequence

Complete these steps in order while minimizing ceremony:

1. **Select hypothesis to test.** State the one hypothesis this cycle will test.
   If the work item already names it, do not restate it in multiple places.
2. **Implement experiment.** Make only the changes needed to run this experiment.
   Keep scope narrow and avoid unrelated cleanup.
3. **Execute experiment.** Run the command, script, benchmark, inspection, or
   manual check that tests the hypothesis. Save important raw outputs as
   artifacts only when they are too large or important to preserve inline.
4. **Create one compact run summary artifact.** Write `run_summary.json` with
   stable keys:
   - `hypothesis`
   - `changed`
   - `commands`
   - `metrics`
   - `pass_criteria`
   - `outcome`: `supported|weakened|falsified|inconclusive|blocked`
   - `claim_delta`
   - `artifacts`
   - `next_work`
5. **Record the compact evidence.** Add the `run_summary.json` artifact with
   `ldgr research core artifact add <run-id> ...`. Record at most one concise
   observation with `ldgr research observation add <run-id> ...` that points
   to the artifact and states the outcome. Do not duplicate the same content
   in observations, markdown reports, and final prose.
6. **Queue possible next research direction.** If more useful research remains,
   queue exactly one next hypothesis/direction as the next work item. Keep it
   bounded enough for a fresh agent instantiation to test in one cycle. You may
   promote a newly discovered work item when the evidence supports that direction;
   make the evidence link explicit in the run summary and closing decision.
7. **End the run.** Close the already-active core run through the research
   surface. Do not use the research experiment `run` command for core closeout.
   Use one of these shapes:

   ```sh
   ldgr research core run close <run-id> --status success --outcome continue \
     --rationale "<outcome and run_summary.json reference>" --next-slug <queued-work>

   ldgr research core run close <run-id> --status partial --outcome continue \
     --rationale "<blocker and run_summary.json reference>" --next-slug <repair-work>

   ldgr research core run close <run-id> --status success --outcome stop \
     --rationale "<why the whole research objective is complete>"
   ```

   `stop` means the whole objective is complete, not merely this experiment.
   A failed agent launch is not a research result: preserve its output, fix the
   harness, and rerun `ldgr research loop run`; the adapter retries the same
   bounded work while retaining the failed attempt as evidence.

## Rules

- One bounded experiment per cycle.
- Do not carry out multiple competing hypotheses in one run.
- Do not broaden a narrow positive result; state exactly what was shown.
- Negative results are progress. Preserve what changed and why.
- Prefer one machine-summarizable artifact over repeated prose. The target is
  maximum continuity per token/minute, not maximum narrative.
- If blocked, record what prevented the experiment and close the run as blocked
  or partial rather than inventing a workaround.
- Never leave a terminal run waiting for an implicit decision. Close it using
  the exact core escape-hatch command above before ending the cycle.
- Queue follow-up work only when it is a concrete next hypothesis or research
  direction. Do not create broad placeholder tasks.

## Artifact/report policy

Routine cycles should produce the thin structured `run_summary.json` record only.
Use `templates/run-summary.json` as the shape when helpful.

Write longer markdown reports only at promotion points:

- claim graph changes;
- surprising negative results;
- operator/model/policy promotion or demotion;
- external-validity shifts;
- milestone synthesis.

Optional promotion-point templates:

- `templates/experiment-plan.md`
- `templates/claim-review.md`
- `templates/negative-result.md`
- `templates/campaign-branch.md`
- `templates/campaign-comparison.md`

The durable record for a routine bounded cycle is the run summary artifact,
minimal evidence references, validation records, and closing decision.

## LDGR context

```json
{{ldgr_context}}
```
