# Research-LDGR model notes

`ldgr-research` is built around a simple idea: a useful research ledger should
help a project survive being wrong.

Instead of treating research as a pile of notes and files, the profile centers
work around claims, expectations, experiments, evidence, decisions, and status
transitions. The output of a failed expectation is not noise; it is often the
result that changes the next question.

## Product identity

Research-LDGR is a durable system for generating, testing, falsifying, and
comparing competing explanations.

Avoid centering the workflow around only documents:

```text
Paper
 ├─ Notes
 ├─ Results
 └─ Files
```

Prefer a workflow centered around testable propositions:

```text
Claim / Hypothesis
 ├─ Expectations
 ├─ Experiments
 ├─ Artifacts
 ├─ Validations
 ├─ Evidence
 └─ Status transitions
```

## Practical conventions

- Record important propositions as claims or hypothesis-like facts.
- Name what would weaken or falsify a claim.
- Turn predictions into checkable expectations.
- Preserve negative results and surprising failures.
- Use decisions to explain how evidence changed belief or direction.
- Queue the next bounded work item as the next useful falsification target.

## Branch and campaign comparisons

Many research programs need competing attempts against the same question. A
campaign should keep branches comparable by sharing:

- a baseline;
- hypotheses per branch;
- common validation requirements;
- a scoring or comparison rubric;
- artifacts per branch;
- branch facts/failures; and
- one comparative decision explaining what survived best.

The bundled campaign templates and scripts are intentionally lightweight. They
create a repeatable workflow without requiring a separate research database.
