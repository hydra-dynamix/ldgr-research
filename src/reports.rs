#![allow(dead_code)]

use std::collections::BTreeMap;

use anyhow::Context;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

const EMPTY: &str = "Not recorded.";

#[derive(Debug, Clone)]
struct ProgramRow {
    id: i64,
    uuid: String,
    slug: String,
    title: String,
    objective: String,
    status: String,
}

#[derive(Debug, Clone)]
struct BranchRow {
    id: i64,
    uuid: String,
    program_id: i64,
    parent_branch_id: Option<i64>,
    slug: String,
    title: String,
    question: String,
    rationale: String,
    status: String,
    decision_summary: Option<String>,
}

#[derive(Debug, Clone)]
struct ExperimentRow {
    id: i64,
    uuid: String,
    branch_id: i64,
    option_id: Option<i64>,
    slug: String,
    title: String,
    phase: Option<String>,
    mode: String,
    hypothesis: Option<String>,
    setup: Option<String>,
    observation_goal: Option<String>,
    rationale: Option<String>,
    primary_metrics_json: String,
    secondary_metrics_json: String,
    pass_criteria: Option<String>,
    fail_criteria: Option<String>,
    allowed_next_steps: Option<String>,
    blocked_next_steps: Option<String>,
    status: String,
}

#[derive(Debug, Clone)]
struct RunRow {
    id: i64,
    uuid: String,
    experiment_id: i64,
    command: Option<String>,
    environment_json: String,
    dataset: Option<String>,
    code_ref: Option<String>,
    started_at: String,
    completed_at: Option<String>,
    status: String,
    notes: Option<String>,
}

#[derive(Debug, Clone)]
struct MetricRow {
    id: i64,
    uuid: String,
    run_id: i64,
    name: String,
    value: f64,
    unit: Option<String>,
    higher_is_better: Option<bool>,
    split: Option<String>,
    metadata_json: String,
}

#[derive(Debug, Clone)]
struct ArtifactRow {
    id: i64,
    uuid: String,
    run_id: i64,
    kind: String,
    path: String,
    description: String,
    checksum: Option<String>,
    metadata_json: String,
}

#[derive(Debug, Clone)]
struct DecisionRow {
    id: i64,
    uuid: String,
    experiment_id: i64,
    result_summary: String,
    interpretation: String,
    limitations: String,
    decision: String,
    confidence: String,
    next_branch_id: Option<i64>,
    next_experiment_id: Option<i64>,
    proposed_options_json: String,
    created_at: String,
}

#[derive(Debug, Clone)]
struct OptionRow {
    id: i64,
    uuid: String,
    program_id: i64,
    branch_id: Option<i64>,
    source_experiment_id: Option<i64>,
    source_decision_id: Option<i64>,
    slug: String,
    title: String,
    hypothesis: Option<String>,
    description: String,
    classification: String,
    status: String,
    review_state: String,
}

#[derive(Debug, Clone)]
struct QuestionRow {
    id: i64,
    uuid: String,
    program_id: i64,
    branch_id: Option<i64>,
    slug: String,
    question: String,
    context: String,
    status: String,
}

#[derive(Debug, Clone)]
struct FactRow {
    id: i64,
    uuid: String,
    program_id: i64,
    branch_id: Option<i64>,
    slug: String,
    statement: String,
    status: String,
    confidence: Option<String>,
    created_from_experiment_id: Option<i64>,
    created_from_decision_id: Option<i64>,
    review_state: String,
}

#[derive(Debug, Clone)]
struct AxiomRow {
    id: i64,
    uuid: String,
    program_id: i64,
    branch_id: Option<i64>,
    slug: String,
    statement: String,
    status: String,
    review_state: String,
}

#[derive(Debug, Clone)]
struct ReviewRow {
    id: i64,
    entity_type: String,
    entity_id: i64,
    reason: String,
    state: String,
}

#[derive(Debug, Clone)]
struct ProgramSnapshot {
    program: ProgramRow,
    branches: Vec<BranchRow>,
    experiments: Vec<ExperimentRow>,
    runs: Vec<RunRow>,
    metrics: Vec<MetricRow>,
    artifacts: Vec<ArtifactRow>,
    decisions: Vec<DecisionRow>,
    options: Vec<OptionRow>,
    questions: Vec<QuestionRow>,
    facts: Vec<FactRow>,
    axioms: Vec<AxiomRow>,
    reviews: Vec<ReviewRow>,
}

pub fn render_status(conn: &Connection) -> anyhow::Result<String> {
    let programs = list_programs(conn)?;
    let options = list_options(conn, None)?;
    let questions = list_questions(conn, None)?;
    let facts = list_facts(conn, None)?;
    let axioms = list_axioms(conn, None)?;
    let reviews = list_reviews(conn)?;

    let open_options = options
        .iter()
        .filter(|option| matches!(option.status.as_str(), "open" | "selected" | "in_progress"))
        .count();
    let open_questions = questions
        .iter()
        .filter(|question| question.status == "open")
        .count();
    let accepted_facts = facts
        .iter()
        .filter(|fact| fact.status == "accepted")
        .count();
    let active_axioms = axioms
        .iter()
        .filter(|axiom| axiom.status == "active")
        .count();
    let attention = reviews
        .iter()
        .filter(|review| review.state == "needs_review")
        .count()
        + options
            .iter()
            .filter(|option| {
                matches!(
                    option.review_state.as_str(),
                    "needs_review" | "approval_required"
                )
            })
            .count();

    let mut out = String::new();
    push_line(&mut out, "Research Status");
    push_line(&mut out, "===============");
    push_line(&mut out, "");
    push_line(&mut out, &format!("Programs: {}", programs.len()));
    push_line(&mut out, &format!("Open research items: {open_options}"));
    push_line(&mut out, &format!("Open questions: {open_questions}"));
    push_line(&mut out, &format!("Accepted facts: {accepted_facts}"));
    push_line(&mut out, &format!("Active axioms: {active_axioms}"));
    push_line(&mut out, &format!("Attention-needed items: {attention}"));

    push_line(&mut out, "");
    push_line(&mut out, "Programs:");
    if programs.is_empty() {
        push_line(&mut out, "- none");
    } else {
        for program in programs {
            push_line(
                &mut out,
                &format!(
                    "- {} ({}) [{}]: {}",
                    program.slug, program.title, program.status, program.objective
                ),
            );
        }
    }

    push_line(&mut out, "");
    push_line(&mut out, "Open Options:");
    push_option_summary(
        &mut out,
        options.iter().filter(|option| option.status == "open"),
    );

    push_line(&mut out, "");
    push_line(&mut out, "Attention Needed:");
    let mut wrote_attention = false;
    for option in options.iter().filter(|option| {
        matches!(
            option.review_state.as_str(),
            "needs_review" | "approval_required"
        )
    }) {
        wrote_attention = true;
        push_line(
            &mut out,
            &format!(
                "- option {} [{}]: {}",
                option.slug, option.review_state, option.title
            ),
        );
    }
    for review in reviews
        .iter()
        .filter(|review| review.state == "needs_review")
    {
        wrote_attention = true;
        push_line(
            &mut out,
            &format!(
                "- review {} {}: {}",
                review.entity_type, review.entity_id, review.reason
            ),
        );
    }
    if !wrote_attention {
        push_line(&mut out, "- none");
    }

    Ok(out)
}

pub fn render_tree(conn: &Connection, program_slug: Option<&str>) -> anyhow::Result<String> {
    let programs = match program_slug {
        Some(slug) => vec![get_program_by_slug(conn, slug)?
            .with_context(|| format!("program {slug} not found"))?],
        None => list_programs(conn)?,
    };

    let mut out = String::new();
    if programs.is_empty() {
        push_line(&mut out, "Research Tree");
        push_line(&mut out, "=============");
        push_line(&mut out, "");
        push_line(&mut out, "No programs recorded.");
        return Ok(out);
    }

    for (program_index, program) in programs.iter().enumerate() {
        if program_index > 0 {
            push_line(&mut out, "");
        }
        let snapshot = load_program_snapshot_by_id(conn, program.id)?;
        push_line(
            &mut out,
            &format!(
                "Program {} ({}) [{}]",
                snapshot.program.slug, snapshot.program.title, snapshot.program.status
            ),
        );
        let roots = snapshot
            .branches
            .iter()
            .filter(|branch| branch.parent_branch_id.is_none())
            .collect::<Vec<_>>();
        if roots.is_empty() {
            push_line(&mut out, "  (no branches)");
        } else {
            for branch in roots {
                push_branch_tree(&mut out, &snapshot, branch, 1);
            }
        }
    }

    Ok(out)
}

pub fn render_experiment_markdown_by_id(
    conn: &Connection,
    experiment_id: i64,
) -> anyhow::Result<String> {
    let experiment = get_experiment_by_id(conn, experiment_id)?;
    let branch = get_branch_by_id(conn, experiment.branch_id)?;
    let snapshot = load_program_snapshot_by_id(conn, branch.program_id)?;
    render_experiment_from_snapshot(&snapshot, &branch, &experiment)
}

pub fn render_experiment_markdown(
    conn: &Connection,
    program_slug: &str,
    branch_slug: &str,
    experiment_slug: &str,
) -> anyhow::Result<String> {
    let snapshot = load_program_snapshot(conn, program_slug)?;
    let branch = snapshot
        .branches
        .iter()
        .find(|branch| branch.slug == branch_slug)
        .with_context(|| format!("branch {branch_slug} not found in program {program_slug}"))?;
    let experiment = snapshot
        .experiments
        .iter()
        .find(|experiment| experiment.branch_id == branch.id && experiment.slug == experiment_slug)
        .with_context(|| {
            format!("experiment {experiment_slug} not found in branch {branch_slug}")
        })?;
    render_experiment_from_snapshot(&snapshot, branch, experiment)
}

pub fn render_program_markdown(conn: &Connection, program_slug: &str) -> anyhow::Result<String> {
    let snapshot = load_program_snapshot(conn, program_slug)?;
    let mut out = String::new();
    push_line(
        &mut out,
        &format!(
            "# Program: {} ({})",
            snapshot.program.title, snapshot.program.slug
        ),
    );
    push_line(&mut out, "");
    push_line(&mut out, &format!("Status: {}", snapshot.program.status));
    push_line(
        &mut out,
        &format!("Objective: {}", snapshot.program.objective),
    );
    push_line(&mut out, "");
    push_line(
        &mut out,
        "Anchors: [Facts](#facts) | [Next Hypotheses](#next-hypotheses)",
    );
    push_line(&mut out, "");
    push_line(&mut out, "## Branches");
    if snapshot.branches.is_empty() {
        push_line(&mut out, "");
        push_line(&mut out, "No branches recorded.");
    } else {
        for branch in &snapshot.branches {
            push_line(&mut out, "");
            push_line(&mut out, &format!("### {} ({})", branch.title, branch.slug));
            push_line(&mut out, &format!("- Status: {}", branch.status));
            push_line(&mut out, &format!("- Question: {}", branch.question));
            push_line(&mut out, &format!("- Rationale: {}", branch.rationale));
            if let Some(summary) = branch.decision_summary.as_deref() {
                push_line(&mut out, &format!("- Decision summary: {summary}"));
            }

            let experiments = snapshot
                .experiments
                .iter()
                .filter(|experiment| experiment.branch_id == branch.id)
                .collect::<Vec<_>>();
            if experiments.is_empty() {
                push_line(&mut out, "- Experiments: none");
            } else {
                push_line(&mut out, "- Experiments:");
                for experiment in experiments {
                    let latest = latest_decision(&snapshot.decisions, experiment.id);
                    let decision = latest
                        .map(|decision| {
                            format!(
                                "; decision: {} ({})",
                                decision.decision, decision.confidence
                            )
                        })
                        .unwrap_or_default();
                    push_line(
                        &mut out,
                        &format!(
                            "  - {} ({}) [{}]{}",
                            experiment.slug, experiment.title, experiment.status, decision
                        ),
                    );
                }
            }
        }
    }

    push_line(&mut out, "");
    push_anchor(&mut out, "facts", "Facts");
    push_fact_lines(&mut out, &snapshot.facts);

    push_line(&mut out, "");
    push_anchor(&mut out, "next-hypotheses", "Next Hypotheses");
    let next_options = snapshot
        .options
        .iter()
        .filter(|option| matches!(option.status.as_str(), "open" | "selected" | "in_progress"));
    push_option_summary(&mut out, next_options);

    Ok(out)
}

pub fn export_program_json(conn: &Connection, program_slug: &str) -> anyhow::Result<Value> {
    let snapshot = load_program_snapshot(conn, program_slug)?;
    Ok(program_snapshot_json(&snapshot))
}

fn render_experiment_from_snapshot(
    snapshot: &ProgramSnapshot,
    branch: &BranchRow,
    experiment: &ExperimentRow,
) -> anyhow::Result<String> {
    let decisions = snapshot
        .decisions
        .iter()
        .filter(|decision| decision.experiment_id == experiment.id)
        .collect::<Vec<_>>();
    let latest = decisions.first().copied();
    let runs = snapshot
        .runs
        .iter()
        .filter(|run| run.experiment_id == experiment.id)
        .collect::<Vec<_>>();
    let run_ids = runs.iter().map(|run| run.id).collect::<Vec<_>>();
    let metrics = snapshot
        .metrics
        .iter()
        .filter(|metric| run_ids.contains(&metric.run_id))
        .collect::<Vec<_>>();
    let artifacts = snapshot
        .artifacts
        .iter()
        .filter(|artifact| run_ids.contains(&artifact.run_id))
        .collect::<Vec<_>>();

    let mut out = String::new();
    push_line(&mut out, &format!("# Experiment: {}", experiment.title));
    push_line(&mut out, "");
    push_line(&mut out, &format!("Experiment: {}", experiment.slug));
    push_line(&mut out, &format!("Branch: {}", branch.slug));
    push_line(&mut out, &format!("Program: {}", snapshot.program.slug));
    push_line(
        &mut out,
        &format!("Phase: {}", display_opt(&experiment.phase)),
    );
    push_line(&mut out, &format!("Status: {}", experiment.status));
    push_line(&mut out, &format!("Mode: {}", experiment.mode));
    push_line(&mut out, "");
    push_line(
        &mut out,
        "Anchors: [Result](#result) | [Interpretation](#interpretation) | [Limitations](#limitations) | [Facts](#facts) | [Next Hypotheses](#next-hypotheses)",
    );
    push_line(&mut out, "");
    push_line(
        &mut out,
        &format!("Hypothesis: {}", display_opt(&experiment.hypothesis)),
    );
    push_line(
        &mut out,
        &format!("Setup: {}", display_opt(&experiment.setup)),
    );
    push_line(
        &mut out,
        &format!(
            "Observation Goal: {}",
            display_opt(&experiment.observation_goal)
        ),
    );
    push_line(
        &mut out,
        &format!("Rationale: {}", display_opt(&experiment.rationale)),
    );
    push_line(
        &mut out,
        &format!(
            "Primary Metrics: {}",
            json_list_for_markdown(&experiment.primary_metrics_json)
        ),
    );
    push_line(
        &mut out,
        &format!(
            "Secondary Metrics: {}",
            json_list_for_markdown(&experiment.secondary_metrics_json)
        ),
    );
    push_line(
        &mut out,
        &format!("Pass Criteria: {}", display_opt(&experiment.pass_criteria)),
    );
    push_line(
        &mut out,
        &format!("Fail Criteria: {}", display_opt(&experiment.fail_criteria)),
    );

    push_line(&mut out, "");
    push_line(&mut out, "## Runs");
    if runs.is_empty() {
        push_line(&mut out, "No runs recorded.");
    } else {
        for run in runs {
            push_line(
                &mut out,
                &format!(
                    "- run {} [{}] started {}{}",
                    run.id,
                    run.status,
                    run.started_at,
                    run.completed_at
                        .as_deref()
                        .map(|value| format!(", completed {value}"))
                        .unwrap_or_default()
                ),
            );
            push_line(
                &mut out,
                &format!("  - Command: {}", display_opt(&run.command)),
            );
            push_line(
                &mut out,
                &format!("  - Dataset: {}", display_opt(&run.dataset)),
            );
            push_line(
                &mut out,
                &format!("  - Code ref: {}", display_opt(&run.code_ref)),
            );
            push_line(&mut out, &format!("  - Notes: {}", display_opt(&run.notes)));
        }
    }

    push_line(&mut out, "");
    push_line(&mut out, "## Metrics");
    if metrics.is_empty() {
        push_line(&mut out, "No metrics recorded.");
    } else {
        for metric in metrics {
            push_line(&mut out, &format!("- {}", metric_summary(metric)));
        }
    }

    push_line(&mut out, "");
    push_line(&mut out, "## Artifacts");
    if artifacts.is_empty() {
        push_line(&mut out, "No artifacts recorded.");
    } else {
        for artifact in artifacts {
            push_line(
                &mut out,
                &format!(
                    "- {} [{}]: {}{}",
                    artifact.path,
                    artifact.kind,
                    artifact.description,
                    artifact
                        .checksum
                        .as_deref()
                        .map(|checksum| format!(" ({checksum})"))
                        .unwrap_or_default()
                ),
            );
        }
    }

    push_line(&mut out, "");
    push_anchor(&mut out, "result", "Result");
    push_line(
        &mut out,
        latest
            .map(|decision| decision.result_summary.as_str())
            .unwrap_or(EMPTY),
    );
    push_line(&mut out, "");
    push_anchor(&mut out, "interpretation", "Interpretation");
    push_line(
        &mut out,
        latest
            .map(|decision| decision.interpretation.as_str())
            .unwrap_or(EMPTY),
    );
    push_line(&mut out, "");
    push_anchor(&mut out, "limitations", "Limitations");
    push_line(
        &mut out,
        latest
            .map(|decision| decision.limitations.as_str())
            .unwrap_or(EMPTY),
    );
    push_line(&mut out, "");
    push_line(
        &mut out,
        &format!(
            "Decision: {}",
            latest
                .map(|decision| decision.decision.as_str())
                .unwrap_or(EMPTY)
        ),
    );
    push_line(
        &mut out,
        &format!(
            "Confidence: {}",
            latest
                .map(|decision| decision.confidence.as_str())
                .unwrap_or(EMPTY)
        ),
    );
    push_line(
        &mut out,
        &format!(
            "Allowed Next Steps: {}",
            display_opt(&experiment.allowed_next_steps)
        ),
    );
    push_line(
        &mut out,
        &format!(
            "Blocked Next Steps: {}",
            display_opt(&experiment.blocked_next_steps)
        ),
    );

    push_line(&mut out, "");
    push_anchor(&mut out, "facts", "Facts");
    let facts = snapshot
        .facts
        .iter()
        .filter(|fact| fact.created_from_experiment_id == Some(experiment.id))
        .cloned()
        .collect::<Vec<_>>();
    push_fact_lines(&mut out, &facts);

    push_line(&mut out, "");
    push_anchor(&mut out, "next-hypotheses", "Next Hypotheses");
    if let Some(decision) = latest {
        push_json_array_lines(&mut out, &decision.proposed_options_json);
    } else {
        push_line(&mut out, "No next hypotheses recorded.");
    }

    Ok(out)
}

fn load_program_snapshot(conn: &Connection, slug: &str) -> anyhow::Result<ProgramSnapshot> {
    let program =
        get_program_by_slug(conn, slug)?.with_context(|| format!("program {slug} not found"))?;
    load_program_snapshot_by_id(conn, program.id)
}

fn load_program_snapshot_by_id(
    conn: &Connection,
    program_id: i64,
) -> anyhow::Result<ProgramSnapshot> {
    let program = get_program_by_id(conn, program_id)?;
    Ok(ProgramSnapshot {
        branches: list_branches(conn, program_id)?,
        experiments: list_experiments_for_program(conn, program_id)?,
        runs: list_runs_for_program(conn, program_id)?,
        metrics: list_metrics_for_program(conn, program_id)?,
        artifacts: list_artifacts_for_program(conn, program_id)?,
        decisions: list_decisions_for_program(conn, program_id)?,
        options: list_options(conn, Some(program_id))?,
        questions: list_questions(conn, Some(program_id))?,
        facts: list_facts(conn, Some(program_id))?,
        axioms: list_axioms(conn, Some(program_id))?,
        reviews: list_reviews(conn)?,
        program,
    })
}

fn list_programs(conn: &Connection) -> anyhow::Result<Vec<ProgramRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, uuid, slug, title, objective, status FROM program ORDER BY created_at, id",
        )
        .context("failed to prepare program list")?;
    collect_rows(&mut stmt, [], |row| {
        Ok(ProgramRow {
            id: row.get("id")?,
            uuid: row.get("uuid")?,
            slug: row.get("slug")?,
            title: row.get("title")?,
            objective: row.get("objective")?,
            status: row.get("status")?,
        })
    })
}

fn get_program_by_id(conn: &Connection, id: i64) -> anyhow::Result<ProgramRow> {
    conn.query_row(
        "SELECT id, uuid, slug, title, objective, status FROM program WHERE id = ?1",
        params![id],
        |row| {
            Ok(ProgramRow {
                id: row.get("id")?,
                uuid: row.get("uuid")?,
                slug: row.get("slug")?,
                title: row.get("title")?,
                objective: row.get("objective")?,
                status: row.get("status")?,
            })
        },
    )
    .optional()
    .with_context(|| format!("failed to read program id {id}"))?
    .with_context(|| format!("program id {id} not found"))
}

fn get_program_by_slug(conn: &Connection, slug: &str) -> anyhow::Result<Option<ProgramRow>> {
    conn.query_row(
        "SELECT id, uuid, slug, title, objective, status FROM program WHERE slug = ?1",
        params![slug],
        |row| {
            Ok(ProgramRow {
                id: row.get("id")?,
                uuid: row.get("uuid")?,
                slug: row.get("slug")?,
                title: row.get("title")?,
                objective: row.get("objective")?,
                status: row.get("status")?,
            })
        },
    )
    .optional()
    .with_context(|| format!("failed to read program {slug}"))
}

fn get_branch_by_id(conn: &Connection, id: i64) -> anyhow::Result<BranchRow> {
    conn.query_row(
        "SELECT id, uuid, program_id, parent_branch_id, slug, title, question, rationale, status, decision_summary
         FROM branch WHERE id = ?1",
        params![id],
        branch_from_row,
    )
    .optional()
    .with_context(|| format!("failed to read branch id {id}"))?
    .with_context(|| format!("branch id {id} not found"))
}

fn list_branches(conn: &Connection, program_id: i64) -> anyhow::Result<Vec<BranchRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, uuid, program_id, parent_branch_id, slug, title, question, rationale, status, decision_summary
             FROM branch WHERE program_id = ?1 ORDER BY created_at, id",
        )
        .context("failed to prepare branch list")?;
    collect_rows(&mut stmt, params![program_id], branch_from_row)
}

fn get_experiment_by_id(conn: &Connection, id: i64) -> anyhow::Result<ExperimentRow> {
    conn.query_row(
        "SELECT id, uuid, branch_id, option_id, slug, title, phase, mode, hypothesis, setup,
                observation_goal, rationale, primary_metrics_json, secondary_metrics_json,
                pass_criteria, fail_criteria, allowed_next_steps, blocked_next_steps, status
         FROM experiment WHERE id = ?1",
        params![id],
        experiment_from_row,
    )
    .optional()
    .with_context(|| format!("failed to read experiment id {id}"))?
    .with_context(|| format!("experiment id {id} not found"))
}

fn list_experiments_for_program(
    conn: &Connection,
    program_id: i64,
) -> anyhow::Result<Vec<ExperimentRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT experiment.id, experiment.uuid, experiment.branch_id, experiment.option_id, experiment.slug,
                    experiment.title, experiment.phase, experiment.mode, experiment.hypothesis,
                    experiment.setup, experiment.observation_goal, experiment.rationale,
                    experiment.primary_metrics_json, experiment.secondary_metrics_json,
                    experiment.pass_criteria, experiment.fail_criteria,
                    experiment.allowed_next_steps, experiment.blocked_next_steps, experiment.status
             FROM experiment
             JOIN branch ON branch.id = experiment.branch_id
             WHERE branch.program_id = ?1
             ORDER BY branch.created_at, branch.id, experiment.created_at, experiment.id",
        )
        .context("failed to prepare experiment list")?;
    collect_rows(&mut stmt, params![program_id], experiment_from_row)
}

fn list_runs_for_program(conn: &Connection, program_id: i64) -> anyhow::Result<Vec<RunRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT run.id, run.uuid, run.experiment_id, run.command, run.environment_json, run.dataset,
                    run.code_ref, run.started_at, run.completed_at, run.status, run.notes
             FROM run
             JOIN experiment ON experiment.id = run.experiment_id
             JOIN branch ON branch.id = experiment.branch_id
             WHERE branch.program_id = ?1
             ORDER BY branch.created_at, branch.id, experiment.created_at, experiment.id, run.started_at, run.id",
        )
        .context("failed to prepare run list")?;
    collect_rows(&mut stmt, params![program_id], |row| {
        Ok(RunRow {
            id: row.get("id")?,
            uuid: row.get("uuid")?,
            experiment_id: row.get("experiment_id")?,
            command: row.get("command")?,
            environment_json: row.get("environment_json")?,
            dataset: row.get("dataset")?,
            code_ref: row.get("code_ref")?,
            started_at: row.get("started_at")?,
            completed_at: row.get("completed_at")?,
            status: row.get("status")?,
            notes: row.get("notes")?,
        })
    })
}

fn list_metrics_for_program(conn: &Connection, program_id: i64) -> anyhow::Result<Vec<MetricRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT metric.id, metric.uuid, metric.run_id, metric.name, metric.value, metric.unit,
                    metric.higher_is_better, metric.split, metric.metadata_json
             FROM metric
             JOIN run ON run.id = metric.run_id
             JOIN experiment ON experiment.id = run.experiment_id
             JOIN branch ON branch.id = experiment.branch_id
             WHERE branch.program_id = ?1
             ORDER BY branch.created_at, branch.id, experiment.created_at, experiment.id, run.started_at, run.id, metric.id",
        )
        .context("failed to prepare metric list")?;
    collect_rows(&mut stmt, params![program_id], metric_from_row)
}

fn list_artifacts_for_program(
    conn: &Connection,
    program_id: i64,
) -> anyhow::Result<Vec<ArtifactRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT artifact.id, artifact.uuid, artifact.run_id, artifact.kind, artifact.path, artifact.description,
                    artifact.checksum, artifact.metadata_json
             FROM artifact
             JOIN run ON run.id = artifact.run_id
             JOIN experiment ON experiment.id = run.experiment_id
             JOIN branch ON branch.id = experiment.branch_id
             WHERE branch.program_id = ?1
             ORDER BY branch.created_at, branch.id, experiment.created_at, experiment.id, run.started_at, run.id, artifact.id",
        )
        .context("failed to prepare artifact list")?;
    collect_rows(&mut stmt, params![program_id], |row| {
        Ok(ArtifactRow {
            id: row.get("id")?,
            uuid: row.get("uuid")?,
            run_id: row.get("run_id")?,
            kind: row.get("kind")?,
            path: row.get("path")?,
            description: row.get("description")?,
            checksum: row.get("checksum")?,
            metadata_json: row.get("metadata_json")?,
        })
    })
}

fn list_decisions_for_program(
    conn: &Connection,
    program_id: i64,
) -> anyhow::Result<Vec<DecisionRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT decision.id, decision.uuid, decision.experiment_id, decision.result_summary,
                    decision.interpretation, decision.limitations, decision.decision,
                    decision.confidence, decision.next_branch_id, decision.next_experiment_id,
                    decision.proposed_options_json, decision.created_at
             FROM decision
             JOIN experiment ON experiment.id = decision.experiment_id
             JOIN branch ON branch.id = experiment.branch_id
             WHERE branch.program_id = ?1
             ORDER BY decision.created_at DESC, decision.id DESC",
        )
        .context("failed to prepare decision list")?;
    collect_rows(&mut stmt, params![program_id], |row| {
        Ok(DecisionRow {
            id: row.get("id")?,
            uuid: row.get("uuid")?,
            experiment_id: row.get("experiment_id")?,
            result_summary: row.get("result_summary")?,
            interpretation: row.get("interpretation")?,
            limitations: row.get("limitations")?,
            decision: row.get("decision")?,
            confidence: row.get("confidence")?,
            next_branch_id: row.get("next_branch_id")?,
            next_experiment_id: row.get("next_experiment_id")?,
            proposed_options_json: row.get("proposed_options_json")?,
            created_at: row.get("created_at")?,
        })
    })
}

fn list_options(conn: &Connection, program_id: Option<i64>) -> anyhow::Result<Vec<OptionRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, uuid, program_id, branch_id, source_experiment_id, source_decision_id, slug,
                    title, hypothesis, description, classification, status, review_state
             FROM research_option
             WHERE (?1 IS NULL OR program_id = ?1)
             ORDER BY created_at, id",
        )
        .context("failed to prepare option list")?;
    collect_rows(&mut stmt, params![program_id], |row| {
        Ok(OptionRow {
            id: row.get("id")?,
            uuid: row.get("uuid")?,
            program_id: row.get("program_id")?,
            branch_id: row.get("branch_id")?,
            source_experiment_id: row.get("source_experiment_id")?,
            source_decision_id: row.get("source_decision_id")?,
            slug: row.get("slug")?,
            title: row.get("title")?,
            hypothesis: row.get("hypothesis")?,
            description: row.get("description")?,
            classification: row.get("classification")?,
            status: row.get("status")?,
            review_state: row.get("review_state")?,
        })
    })
}

fn list_questions(conn: &Connection, program_id: Option<i64>) -> anyhow::Result<Vec<QuestionRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, uuid, program_id, branch_id, slug, question, context, status
             FROM open_question
             WHERE (?1 IS NULL OR program_id = ?1)
             ORDER BY created_at, id",
        )
        .context("failed to prepare question list")?;
    collect_rows(&mut stmt, params![program_id], |row| {
        Ok(QuestionRow {
            id: row.get("id")?,
            uuid: row.get("uuid")?,
            program_id: row.get("program_id")?,
            branch_id: row.get("branch_id")?,
            slug: row.get("slug")?,
            question: row.get("question")?,
            context: row.get("context")?,
            status: row.get("status")?,
        })
    })
}

fn list_facts(conn: &Connection, program_id: Option<i64>) -> anyhow::Result<Vec<FactRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, uuid, program_id, branch_id, slug, statement, status, confidence,
                    created_from_experiment_id, created_from_decision_id, review_state
             FROM fact
             WHERE (?1 IS NULL OR program_id = ?1)
             ORDER BY created_at, id",
        )
        .context("failed to prepare fact list")?;
    collect_rows(&mut stmt, params![program_id], |row| {
        Ok(FactRow {
            id: row.get("id")?,
            uuid: row.get("uuid")?,
            program_id: row.get("program_id")?,
            branch_id: row.get("branch_id")?,
            slug: row.get("slug")?,
            statement: row.get("statement")?,
            status: row.get("status")?,
            confidence: row.get("confidence")?,
            created_from_experiment_id: row.get("created_from_experiment_id")?,
            created_from_decision_id: row.get("created_from_decision_id")?,
            review_state: row.get("review_state")?,
        })
    })
}

fn list_axioms(conn: &Connection, program_id: Option<i64>) -> anyhow::Result<Vec<AxiomRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, uuid, program_id, branch_id, slug, statement, status, review_state
             FROM axiom
             WHERE (?1 IS NULL OR program_id = ?1)
             ORDER BY created_at, id",
        )
        .context("failed to prepare axiom list")?;
    collect_rows(&mut stmt, params![program_id], |row| {
        Ok(AxiomRow {
            id: row.get("id")?,
            uuid: row.get("uuid")?,
            program_id: row.get("program_id")?,
            branch_id: row.get("branch_id")?,
            slug: row.get("slug")?,
            statement: row.get("statement")?,
            status: row.get("status")?,
            review_state: row.get("review_state")?,
        })
    })
}

fn list_reviews(conn: &Connection) -> anyhow::Result<Vec<ReviewRow>> {
    let mut stmt = conn
        .prepare("SELECT id, entity_type, entity_id, reason, state FROM review_item ORDER BY created_at, id")
        .context("failed to prepare review list")?;
    collect_rows(&mut stmt, [], |row| {
        Ok(ReviewRow {
            id: row.get("id")?,
            entity_type: row.get("entity_type")?,
            entity_id: row.get("entity_id")?,
            reason: row.get("reason")?,
            state: row.get("state")?,
        })
    })
}

fn branch_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BranchRow> {
    Ok(BranchRow {
        id: row.get("id")?,
        uuid: row.get("uuid")?,
        program_id: row.get("program_id")?,
        parent_branch_id: row.get("parent_branch_id")?,
        slug: row.get("slug")?,
        title: row.get("title")?,
        question: row.get("question")?,
        rationale: row.get("rationale")?,
        status: row.get("status")?,
        decision_summary: row.get("decision_summary")?,
    })
}

fn experiment_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExperimentRow> {
    Ok(ExperimentRow {
        id: row.get("id")?,
        uuid: row.get("uuid")?,
        branch_id: row.get("branch_id")?,
        option_id: row.get("option_id")?,
        slug: row.get("slug")?,
        title: row.get("title")?,
        phase: row.get("phase")?,
        mode: row.get("mode")?,
        hypothesis: row.get("hypothesis")?,
        setup: row.get("setup")?,
        observation_goal: row.get("observation_goal")?,
        rationale: row.get("rationale")?,
        primary_metrics_json: row.get("primary_metrics_json")?,
        secondary_metrics_json: row.get("secondary_metrics_json")?,
        pass_criteria: row.get("pass_criteria")?,
        fail_criteria: row.get("fail_criteria")?,
        allowed_next_steps: row.get("allowed_next_steps")?,
        blocked_next_steps: row.get("blocked_next_steps")?,
        status: row.get("status")?,
    })
}

fn metric_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MetricRow> {
    let higher_is_better: Option<i64> = row.get("higher_is_better")?;
    Ok(MetricRow {
        id: row.get("id")?,
        uuid: row.get("uuid")?,
        run_id: row.get("run_id")?,
        name: row.get("name")?,
        value: row.get("value")?,
        unit: row.get("unit")?,
        higher_is_better: higher_is_better.map(|value| value != 0),
        split: row.get("split")?,
        metadata_json: row.get("metadata_json")?,
    })
}

fn collect_rows<T, P, F>(
    stmt: &mut rusqlite::Statement<'_>,
    params: P,
    mapper: F,
) -> anyhow::Result<Vec<T>>
where
    P: rusqlite::Params,
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    stmt.query_map(params, mapper)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read query rows")
}

fn push_branch_tree(
    out: &mut String,
    snapshot: &ProgramSnapshot,
    branch: &BranchRow,
    depth: usize,
) {
    let indent = "  ".repeat(depth);
    push_line(
        out,
        &format!(
            "{indent}- Branch {} ({}) [{}]",
            branch.slug, branch.title, branch.status
        ),
    );
    for experiment in snapshot
        .experiments
        .iter()
        .filter(|experiment| experiment.branch_id == branch.id)
    {
        let latest = latest_decision(&snapshot.decisions, experiment.id);
        let metrics = notable_metrics(snapshot, experiment.id);
        let decision = latest
            .map(|decision| {
                format!(
                    "; latest decision: {} ({})",
                    decision.decision, decision.confidence
                )
            })
            .unwrap_or_else(|| "; latest decision: none".to_string());
        let metric_text = if metrics.is_empty() {
            String::new()
        } else {
            format!("; metrics: {}", metrics.join(", "))
        };
        push_line(
            out,
            &format!(
                "{indent}  - Experiment {} ({}) [{}]{}{metric_text}",
                experiment.slug, experiment.title, experiment.status, decision
            ),
        );
    }

    for child in snapshot
        .branches
        .iter()
        .filter(|child| child.parent_branch_id == Some(branch.id))
    {
        push_branch_tree(out, snapshot, child, depth + 1);
    }
}

fn latest_decision(decisions: &[DecisionRow], experiment_id: i64) -> Option<&DecisionRow> {
    decisions
        .iter()
        .find(|decision| decision.experiment_id == experiment_id)
}

fn notable_metrics(snapshot: &ProgramSnapshot, experiment_id: i64) -> Vec<String> {
    let run_ids = snapshot
        .runs
        .iter()
        .filter(|run| run.experiment_id == experiment_id)
        .map(|run| run.id)
        .collect::<Vec<_>>();
    let mut by_name = BTreeMap::new();
    for metric in snapshot
        .metrics
        .iter()
        .filter(|metric| run_ids.contains(&metric.run_id))
    {
        by_name.insert(metric.name.clone(), metric_summary(metric));
    }
    by_name.into_values().collect()
}

fn metric_summary(metric: &MetricRow) -> String {
    let unit = metric
        .unit
        .as_deref()
        .map(|unit| format!(" {unit}"))
        .unwrap_or_default();
    let split = metric
        .split
        .as_deref()
        .map(|split| format!(" ({split})"))
        .unwrap_or_default();
    format!(
        "{}={}{}{}",
        metric.name,
        stable_float(metric.value),
        unit,
        split
    )
}

fn program_snapshot_json(snapshot: &ProgramSnapshot) -> Value {
    json!({
        "format": "ldgr-research.program_export.v2",
        "exported_at": Utc::now().to_rfc3339(),
        "ldgr-research_version": env!("CARGO_PKG_VERSION"),
        "schema_version": crate::migrations::CURRENT_SCHEMA_VERSION,
        "source": Value::Null,
        "program": {
            "id": snapshot.program.id,
            "uuid": snapshot.program.uuid,
            "slug": snapshot.program.slug,
            "title": snapshot.program.title,
            "objective": snapshot.program.objective,
            "status": snapshot.program.status,
        },
        "branches": snapshot.branches.iter().map(branch_json).collect::<Vec<_>>(),
        "experiments": snapshot.experiments.iter().map(|experiment| experiment_json(snapshot, experiment)).collect::<Vec<_>>(),
        "open_questions": snapshot.questions.iter().map(question_json).collect::<Vec<_>>(),
        "research_options": snapshot.options.iter().map(option_json).collect::<Vec<_>>(),
        "facts": snapshot.facts.iter().map(fact_json).collect::<Vec<_>>(),
        "axioms": snapshot.axioms.iter().map(axiom_json).collect::<Vec<_>>(),
    })
}

fn branch_json(branch: &BranchRow) -> Value {
    json!({
        "id": branch.id,
        "uuid": branch.uuid,
        "program_id": branch.program_id,
        "parent_branch_id": branch.parent_branch_id,
        "slug": branch.slug,
        "title": branch.title,
        "question": branch.question,
        "rationale": branch.rationale,
        "status": branch.status,
        "decision_summary": branch.decision_summary,
    })
}

fn experiment_json(snapshot: &ProgramSnapshot, experiment: &ExperimentRow) -> Value {
    let runs = snapshot
        .runs
        .iter()
        .filter(|run| run.experiment_id == experiment.id)
        .collect::<Vec<_>>();
    let run_ids = runs.iter().map(|run| run.id).collect::<Vec<_>>();
    json!({
        "id": experiment.id,
        "uuid": experiment.uuid,
        "branch_id": experiment.branch_id,
        "option_id": experiment.option_id,
        "slug": experiment.slug,
        "title": experiment.title,
        "phase": experiment.phase,
        "mode": experiment.mode,
        "hypothesis": experiment.hypothesis,
        "setup": experiment.setup,
        "observation_goal": experiment.observation_goal,
        "rationale": experiment.rationale,
        "primary_metrics": parse_json_or_string(&experiment.primary_metrics_json),
        "secondary_metrics": parse_json_or_string(&experiment.secondary_metrics_json),
        "pass_criteria": experiment.pass_criteria,
        "fail_criteria": experiment.fail_criteria,
        "allowed_next_steps": experiment.allowed_next_steps,
        "blocked_next_steps": experiment.blocked_next_steps,
        "status": experiment.status,
        "runs": runs.iter().map(|run| run_json(run)).collect::<Vec<_>>(),
        "metrics": snapshot.metrics.iter().filter(|metric| run_ids.contains(&metric.run_id)).map(metric_json).collect::<Vec<_>>(),
        "artifacts": snapshot.artifacts.iter().filter(|artifact| run_ids.contains(&artifact.run_id)).map(artifact_json).collect::<Vec<_>>(),
        "decisions": snapshot.decisions.iter().filter(|decision| decision.experiment_id == experiment.id).map(decision_json).collect::<Vec<_>>(),
    })
}

fn run_json(run: &RunRow) -> Value {
    json!({
        "id": run.id,
        "uuid": run.uuid,
        "experiment_id": run.experiment_id,
        "command": run.command,
        "environment": parse_json_or_string(&run.environment_json),
        "dataset": run.dataset,
        "code_ref": run.code_ref,
        "started_at": run.started_at,
        "completed_at": run.completed_at,
        "status": run.status,
        "notes": run.notes,
    })
}

fn metric_json(metric: &MetricRow) -> Value {
    json!({
        "id": metric.id,
        "uuid": metric.uuid,
        "run_id": metric.run_id,
        "name": metric.name,
        "value": metric.value,
        "unit": metric.unit,
        "higher_is_better": metric.higher_is_better,
        "split": metric.split,
        "metadata": parse_json_or_string(&metric.metadata_json),
    })
}

fn artifact_json(artifact: &ArtifactRow) -> Value {
    json!({
        "id": artifact.id,
        "uuid": artifact.uuid,
        "run_id": artifact.run_id,
        "kind": artifact.kind,
        "path": artifact.path,
        "description": artifact.description,
        "checksum": artifact.checksum,
        "metadata": parse_json_or_string(&artifact.metadata_json),
    })
}

fn decision_json(decision: &DecisionRow) -> Value {
    json!({
        "id": decision.id,
        "uuid": decision.uuid,
        "experiment_id": decision.experiment_id,
        "result_summary": decision.result_summary,
        "interpretation": decision.interpretation,
        "limitations": decision.limitations,
        "decision": decision.decision,
        "confidence": decision.confidence,
        "next_branch_id": decision.next_branch_id,
        "next_experiment_id": decision.next_experiment_id,
        "proposed_options": parse_json_or_string(&decision.proposed_options_json),
        "created_at": decision.created_at,
    })
}

fn question_json(question: &QuestionRow) -> Value {
    json!({
        "id": question.id,
        "uuid": question.uuid,
        "program_id": question.program_id,
        "branch_id": question.branch_id,
        "slug": question.slug,
        "question": question.question,
        "context": question.context,
        "status": question.status,
    })
}

fn option_json(option: &OptionRow) -> Value {
    json!({
        "id": option.id,
        "uuid": option.uuid,
        "program_id": option.program_id,
        "branch_id": option.branch_id,
        "source_experiment_id": option.source_experiment_id,
        "source_decision_id": option.source_decision_id,
        "slug": option.slug,
        "title": option.title,
        "hypothesis": option.hypothesis,
        "description": option.description,
        "classification": option.classification,
        "status": option.status,
        "review_state": option.review_state,
    })
}

fn fact_json(fact: &FactRow) -> Value {
    json!({
        "id": fact.id,
        "uuid": fact.uuid,
        "program_id": fact.program_id,
        "branch_id": fact.branch_id,
        "slug": fact.slug,
        "statement": fact.statement,
        "status": fact.status,
        "confidence": fact.confidence,
        "created_from_experiment_id": fact.created_from_experiment_id,
        "created_from_decision_id": fact.created_from_decision_id,
        "review_state": fact.review_state,
    })
}

fn axiom_json(axiom: &AxiomRow) -> Value {
    json!({
        "id": axiom.id,
        "uuid": axiom.uuid,
        "program_id": axiom.program_id,
        "branch_id": axiom.branch_id,
        "slug": axiom.slug,
        "statement": axiom.statement,
        "status": axiom.status,
        "review_state": axiom.review_state,
    })
}

fn push_anchor(out: &mut String, id: &str, title: &str) {
    push_line(out, &format!("<a id=\"{id}\"></a>"));
    push_line(out, &format!("## {title}"));
}

fn push_fact_lines(out: &mut String, facts: &[FactRow]) {
    if facts.is_empty() {
        push_line(out, "No facts recorded.");
    } else {
        for fact in facts {
            let confidence = fact
                .confidence
                .as_deref()
                .map(|confidence| format!(", confidence {confidence}"))
                .unwrap_or_default();
            push_line(
                out,
                &format!(
                    "- {} [{}{}]: {}",
                    fact.slug, fact.status, confidence, fact.statement
                ),
            );
        }
    }
}

fn push_option_summary<'a, I>(out: &mut String, options: I)
where
    I: IntoIterator<Item = &'a OptionRow>,
{
    let mut wrote = false;
    for option in options {
        wrote = true;
        let hypothesis = option
            .hypothesis
            .as_deref()
            .map(|hypothesis| format!("; hypothesis: {hypothesis}"))
            .unwrap_or_default();
        push_line(
            out,
            &format!(
                "- {} [{}:{}]: {}{}",
                option.slug, option.classification, option.status, option.title, hypothesis
            ),
        );
    }
    if !wrote {
        push_line(out, "- none");
    }
}

fn push_json_array_lines(out: &mut String, source: &str) {
    match serde_json::from_str::<Value>(source) {
        Ok(Value::Array(values)) if values.is_empty() => {
            push_line(out, "No next hypotheses recorded.");
        }
        Ok(Value::Array(values)) => {
            for value in values {
                match value {
                    Value::String(text) => push_line(out, &format!("- {text}")),
                    Value::Object(map) => {
                        let title = map
                            .get("title")
                            .and_then(Value::as_str)
                            .or_else(|| map.get("hypothesis").and_then(Value::as_str))
                            .unwrap_or("proposed option");
                        push_line(out, &format!("- {title}"));
                    }
                    other => push_line(out, &format!("- {other}")),
                }
            }
        }
        Ok(value) => push_line(out, &format!("- {value}")),
        Err(_) if source.trim().is_empty() => push_line(out, "No next hypotheses recorded."),
        Err(_) => push_line(out, &format!("- {source}")),
    }
}

fn json_list_for_markdown(source: &str) -> String {
    match serde_json::from_str::<Value>(source) {
        Ok(Value::Array(values)) if values.is_empty() => "none".to_string(),
        Ok(Value::Array(values)) => values
            .iter()
            .map(|value| match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", "),
        Ok(value) => value.to_string(),
        Err(_) => source.to_string(),
    }
}

fn parse_json_or_string(source: &str) -> Value {
    serde_json::from_str(source).unwrap_or_else(|_| json!(source))
}

fn display_opt(value: &Option<String>) -> &str {
    value
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(EMPTY)
}

fn stable_float(value: f64) -> String {
    let mut text = value.to_string();
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.push('0');
        }
    }
    text
}

fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}
