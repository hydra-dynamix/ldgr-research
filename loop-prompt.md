# Research Loop

{{job_complete_policy}}

{{completion_audit_instruction}}

You are running one bounded research cycle. Complete exactly the assigned work
item stated at the top of this prompt. Treat this cycle as one experiment, not a
research program. LDGR carries continuity between cycles; you do not need to keep
the whole project in context.

The LDGR context below is the durable source of truth. When it conflicts with
project docs or memory, the ledger wins.

## Start of cycle

1. Read `AGENTS.md` if present.
2. Run `git status --short`.
3. Read the assigned work item and the LDGR context.
4. Select exactly one hypothesis to test in this cycle.

If the assigned work item already names the hypothesis, use that. If it names a
broader research direction, narrow it to one testable hypothesis before doing
any implementation.

## Required research sequence

Complete these steps in order:

1. **Select hypothesis to test.** State the one hypothesis this cycle will test.
2. **Record hypothesis selection.** Add an LDGR observation before implementation
   that records:
   - the selected hypothesis;
   - why it is the next useful hypothesis;
   - what result would support it;
   - what result would weaken or falsify it.
3. **Implement experiment.** Make only the changes needed to run this experiment.
   Keep scope narrow and avoid unrelated cleanup.
4. **Execute experiment.** Run the command, script, benchmark, inspection, or
   manual check that tests the hypothesis. Save important outputs as artifacts
   when they are too large for an observation.
5. **Interpret results.** Separate raw result from interpretation. Explain what
   happened, what it means, and what uncertainty remains.
6. **Record hypothesis outcome.** Add an LDGR observation that states whether the
   hypothesis was supported, weakened, falsified, or remains inconclusive, with
   evidence references.
7. **Queue possible next research direction.** If more useful research remains,
   queue exactly one next hypothesis/direction as the next work item. Keep it
   bounded enough for a fresh agent instantiation to test in one cycle.
8. **End the run.** Close the run with a decision. The rationale must include:
   result summary, interpretation, limitations, confidence, and next hypothesis
   if one was queued.

## Rules

- One bounded experiment per cycle.
- Do not carry out multiple competing hypotheses in one run.
- Do not broaden a narrow positive result; state exactly what was shown.
- Negative results are progress. Preserve what changed and why.
- Prefer durable observations and artifacts over relying on prose in the final
  response.
- If blocked, record what prevented the experiment and close the run as blocked
  or partial rather than inventing a workaround.
- Queue follow-up work only when it is a concrete next hypothesis or research
  direction. Do not create broad placeholder tasks.

## Useful artifact templates

Use these when they help make the experiment durable:

- `templates/experiment-plan.md`
- `templates/claim-review.md`
- `templates/negative-result.md`
- `templates/campaign-branch.md`
- `templates/campaign-comparison.md`

Templates are optional. The required durable record is the LDGR observations,
artifacts, and closing decision for this bounded cycle.

## LDGR context

```json
{{ldgr_context}}
```
