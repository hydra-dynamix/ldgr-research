#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use rusqlite::{params, Connection, OptionalExtension};

use crate::migrations;
use crate::policy;
use crate::schema::{
    Artifact, Axiom, AxiomFilter, AxiomStatus, AxiomUpdate, Branch, BranchStatus, BugReport,
    BugReportFilter, BugReportStatus, Decision, EvidenceLink, EvidenceRelation, Experiment,
    ExperimentStatus, ExperimentUpdate, Fact, FactFilter, FactStatus, FactUpdate, MatrixAxis,
    MatrixCell, MatrixCellUpdate, MatrixLevel, Metric, NewArtifact, NewAxiom, NewBranch,
    NewBugReport, NewDecision, NewEvidenceLink, NewExperiment, NewFact, NewMatrixAxis,
    NewMatrixCell, NewMatrixLevel, NewMetric, NewOpenQuestion, NewOverrideApprovalRequest,
    NewProgram, NewResearchMatrix, NewResearchOption, NewReviewItem, NewRun, OpenQuestion,
    OpenQuestionFilter, OpenQuestionStatus, OpenQuestionUpdate, OverrideApproval,
    OverrideApprovalFilter, OverrideApprovalStatus, Program, ProgramStatus, ResearchMatrix,
    ResearchMatrixUpdate, ResearchOption, ResearchOptionClassification, ResearchOptionFilter,
    ResearchOptionStatus, ResearchOptionUpdate, ReviewItem, ReviewItemFilter, ReviewItemState,
    ReviewState, Run, RunStatus, RunStatusUpdate,
};

#[derive(Debug, serde::Deserialize)]
struct ProposedOptionJson {
    slug: String,
    description: String,
    classification: Option<String>,
}

pub const DEFAULT_RESEARCH_DIR: &str = ".ldgr/research";
pub const DEFAULT_DB_PATH: &str = ".ldgr/research/research.db";
pub const DEFAULT_POLICY_PATH: &str = ".ldgr/research/policy.yaml";
pub const ARTIFACTS_DIR: &str = "artifacts";
pub const REPORTS_DIR: &str = "reports";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitResult {
    pub research_dir: PathBuf,
    pub db_path: PathBuf,
    pub policy_path: PathBuf,
    pub policy_written: bool,
    pub applied_migrations: Vec<i64>,
}

pub fn default_db_path() -> PathBuf {
    PathBuf::from(DEFAULT_DB_PATH)
}

pub fn default_policy_path() -> PathBuf {
    PathBuf::from(DEFAULT_POLICY_PATH)
}

pub fn open_database(path: impl AsRef<Path>) -> anyhow::Result<Connection> {
    let path = path.as_ref();
    let conn = Connection::open(path)
        .with_context(|| format!("failed to open SQLite database {}", path.display()))?;
    enable_foreign_keys(&conn)?;
    Ok(conn)
}

pub fn open_in_memory_database() -> anyhow::Result<Connection> {
    let conn = Connection::open_in_memory().context("failed to open in-memory SQLite database")?;
    enable_foreign_keys(&conn)?;
    Ok(conn)
}

pub fn init_project(
    db_path: impl AsRef<Path>,
    policy_path: impl AsRef<Path>,
) -> anyhow::Result<InitResult> {
    let db_path = db_path.as_ref();
    let policy_path = policy_path.as_ref();
    let research_dir = research_dir_for(db_path, policy_path);

    fs::create_dir_all(&research_dir).with_context(|| {
        format!(
            "failed to create research directory {}",
            research_dir.display()
        )
    })?;
    fs::create_dir_all(research_dir.join(ARTIFACTS_DIR)).with_context(|| {
        format!(
            "failed to create artifacts directory {}",
            research_dir.join(ARTIFACTS_DIR).display()
        )
    })?;
    fs::create_dir_all(research_dir.join(REPORTS_DIR)).with_context(|| {
        format!(
            "failed to create reports directory {}",
            research_dir.join(REPORTS_DIR).display()
        )
    })?;

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create database directory {}", parent.display()))?;
    }

    let policy_written = policy::write_starter_policy_if_missing(policy_path)?;
    let mut conn = open_database(db_path)?;
    let applied_migrations = migrations::apply_migrations(&mut conn)?
        .into_iter()
        .map(|migration| migration.version)
        .collect();

    Ok(InitResult {
        research_dir,
        db_path: db_path.to_path_buf(),
        policy_path: policy_path.to_path_buf(),
        policy_written,
        applied_migrations,
    })
}

fn enable_foreign_keys(conn: &Connection) -> anyhow::Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")
        .context("failed to enable SQLite foreign-key enforcement")?;

    let enabled: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .context("failed to verify SQLite foreign-key enforcement")?;
    if enabled != 1 {
        bail!("SQLite foreign-key enforcement could not be enabled");
    }
    Ok(())
}

fn research_dir_for(db_path: &Path, policy_path: &Path) -> PathBuf {
    for path in [db_path, policy_path] {
        if let Some(parent) = path.parent() {
            if parent.file_name().and_then(|name| name.to_str()) == Some(DEFAULT_RESEARCH_DIR) {
                return parent.to_path_buf();
            }
        }
    }

    db_path
        .parent()
        .map(Path::to_path_buf)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RESEARCH_DIR))
}

pub fn create_program(conn: &Connection, input: &NewProgram<'_>) -> anyhow::Result<Program> {
    conn.execute(
        "INSERT INTO program (slug, title, objective, status)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            input.slug,
            input.title,
            input.objective,
            input.status.as_str()
        ],
    )
    .with_context(|| format!("failed to create program {}", input.slug))?;

    let id = conn.last_insert_rowid();
    record_event(conn, "program", id, "create", "{}")?;
    get_program_by_id(conn, id)
}

pub fn get_program_by_id(conn: &Connection, id: i64) -> anyhow::Result<Program> {
    conn.query_row(
        "SELECT * FROM program WHERE id = ?1",
        params![id],
        Program::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read program id {id}"))?
    .with_context(|| format!("program id {id} not found"))
}

pub fn get_program_by_slug(conn: &Connection, slug: &str) -> anyhow::Result<Option<Program>> {
    conn.query_row(
        "SELECT * FROM program WHERE slug = ?1",
        params![slug],
        Program::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read program {slug}"))
}

pub fn list_programs(conn: &Connection) -> anyhow::Result<Vec<Program>> {
    let mut stmt = conn
        .prepare("SELECT * FROM program ORDER BY created_at, id")
        .context("failed to prepare program list query")?;
    let programs = stmt
        .query_map([], Program::from_row)
        .context("failed to query programs")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read programs")?;
    Ok(programs)
}

pub fn update_program_status(
    conn: &Connection,
    id: i64,
    status: ProgramStatus,
) -> anyhow::Result<Program> {
    conn.execute(
        "UPDATE program SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![status.as_str(), id],
    )
    .with_context(|| format!("failed to update program id {id}"))?;
    record_event(conn, "program", id, "update_status", "{}")?;
    get_program_by_id(conn, id)
}

pub fn create_branch(conn: &Connection, input: &NewBranch<'_>) -> anyhow::Result<Branch> {
    conn.execute(
        "INSERT INTO branch
         (program_id, parent_branch_id, slug, title, question, rationale, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            input.program_id,
            input.parent_branch_id,
            input.slug,
            input.title,
            input.question,
            input.rationale,
            input.status.as_str()
        ],
    )
    .with_context(|| format!("failed to create branch {}", input.slug))?;

    let id = conn.last_insert_rowid();
    record_event(conn, "branch", id, "create", "{}")?;
    get_branch_by_id(conn, id)
}

pub fn get_branch_by_id(conn: &Connection, id: i64) -> anyhow::Result<Branch> {
    conn.query_row(
        "SELECT * FROM branch WHERE id = ?1",
        params![id],
        Branch::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read branch id {id}"))?
    .with_context(|| format!("branch id {id} not found"))
}

pub fn get_branch_by_slug(
    conn: &Connection,
    program_id: i64,
    slug: &str,
) -> anyhow::Result<Option<Branch>> {
    conn.query_row(
        "SELECT * FROM branch WHERE program_id = ?1 AND slug = ?2",
        params![program_id, slug],
        Branch::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read branch {slug}"))
}

pub fn list_branches(conn: &Connection, program_id: i64) -> anyhow::Result<Vec<Branch>> {
    let mut stmt = conn
        .prepare("SELECT * FROM branch WHERE program_id = ?1 ORDER BY created_at, id")
        .context("failed to prepare branch list query")?;
    let branches = stmt
        .query_map(params![program_id], Branch::from_row)
        .context("failed to query branches")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read branches")?;
    Ok(branches)
}

pub fn update_branch_status(
    conn: &Connection,
    id: i64,
    status: BranchStatus,
) -> anyhow::Result<Branch> {
    conn.execute(
        "UPDATE branch SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![status.as_str(), id],
    )
    .with_context(|| format!("failed to update branch id {id}"))?;
    record_event(conn, "branch", id, "update_status", "{}")?;
    get_branch_by_id(conn, id)
}

pub fn create_experiment(
    conn: &Connection,
    input: &NewExperiment<'_>,
) -> anyhow::Result<Experiment> {
    conn.execute(
        "INSERT INTO experiment
         (branch_id, option_id, slug, title, phase, mode, hypothesis, setup, observation_goal,
          rationale, primary_metrics_json, secondary_metrics_json, pass_criteria, fail_criteria,
          allowed_next_steps, blocked_next_steps, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            input.branch_id,
            input.option_id,
            input.slug,
            input.title,
            input.phase,
            input.mode.as_str(),
            input.hypothesis,
            input.setup,
            input.observation_goal,
            input.rationale,
            input.primary_metrics_json,
            input.secondary_metrics_json,
            input.pass_criteria,
            input.fail_criteria,
            input.allowed_next_steps,
            input.blocked_next_steps,
            input.status.as_str()
        ],
    )
    .with_context(|| format!("failed to create experiment {}", input.slug))?;

    let id = conn.last_insert_rowid();
    record_event(conn, "experiment", id, "create", "{}")?;
    get_experiment_by_id(conn, id)
}

pub fn get_experiment_by_id(conn: &Connection, id: i64) -> anyhow::Result<Experiment> {
    conn.query_row(
        "SELECT * FROM experiment WHERE id = ?1",
        params![id],
        Experiment::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read experiment id {id}"))?
    .with_context(|| format!("experiment id {id} not found"))
}

pub fn get_experiment_by_slug(
    conn: &Connection,
    branch_id: i64,
    slug: &str,
) -> anyhow::Result<Option<Experiment>> {
    conn.query_row(
        "SELECT * FROM experiment WHERE branch_id = ?1 AND slug = ?2",
        params![branch_id, slug],
        Experiment::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read experiment {slug}"))
}

pub fn get_experiment_by_slug_in_current_branch(
    conn: &Connection,
    program_slug: &str,
    branch_slug: &str,
    experiment_slug: &str,
) -> anyhow::Result<Option<Experiment>> {
    conn.query_row(
        "SELECT experiment.*
         FROM experiment
         JOIN branch ON branch.id = experiment.branch_id
         JOIN program ON program.id = branch.program_id
         WHERE program.slug = ?1
           AND branch.slug = ?2
           AND experiment.slug = ?3",
        params![program_slug, branch_slug, experiment_slug],
        Experiment::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read experiment {experiment_slug} in branch {branch_slug}"))
}

pub fn list_experiments(conn: &Connection, branch_id: i64) -> anyhow::Result<Vec<Experiment>> {
    let mut stmt = conn
        .prepare("SELECT * FROM experiment WHERE branch_id = ?1 ORDER BY created_at, id")
        .context("failed to prepare experiment list query")?;
    let experiments = stmt
        .query_map(params![branch_id], Experiment::from_row)
        .context("failed to query experiments")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read experiments")?;
    Ok(experiments)
}

pub fn update_experiment(
    conn: &Connection,
    id: i64,
    update: &ExperimentUpdate<'_>,
) -> anyhow::Result<Experiment> {
    let current = get_experiment_by_id(conn, id)?;
    let option_id = update.option_id.unwrap_or(current.option_id);
    let title = update.title.unwrap_or(&current.title);
    let phase = match update.phase {
        Some(value) => value.map(str::to_owned),
        None => current.phase.as_deref().map(str::to_owned),
    };
    let mode = update
        .mode
        .map(|mode| mode.as_str())
        .unwrap_or(&current.mode);
    let hypothesis = match update.hypothesis {
        Some(value) => value.map(str::to_owned),
        None => current.hypothesis.as_deref().map(str::to_owned),
    };
    let setup = match update.setup {
        Some(value) => value.map(str::to_owned),
        None => current.setup.as_deref().map(str::to_owned),
    };
    let observation_goal = match update.observation_goal {
        Some(value) => value.map(str::to_owned),
        None => current.observation_goal.as_deref().map(str::to_owned),
    };
    let rationale = match update.rationale {
        Some(value) => value.map(str::to_owned),
        None => current.rationale.as_deref().map(str::to_owned),
    };
    let primary_metrics_json = update
        .primary_metrics_json
        .unwrap_or(&current.primary_metrics_json);
    let secondary_metrics_json = update
        .secondary_metrics_json
        .unwrap_or(&current.secondary_metrics_json);
    let pass_criteria = match update.pass_criteria {
        Some(value) => value.map(str::to_owned),
        None => current.pass_criteria.as_deref().map(str::to_owned),
    };
    let fail_criteria = match update.fail_criteria {
        Some(value) => value.map(str::to_owned),
        None => current.fail_criteria.as_deref().map(str::to_owned),
    };
    let allowed_next_steps = match update.allowed_next_steps {
        Some(value) => value.map(str::to_owned),
        None => current.allowed_next_steps.as_deref().map(str::to_owned),
    };
    let blocked_next_steps = match update.blocked_next_steps {
        Some(value) => value.map(str::to_owned),
        None => current.blocked_next_steps.as_deref().map(str::to_owned),
    };
    let status = update
        .status
        .map(ExperimentStatus::as_str)
        .unwrap_or(&current.status);

    validate_experiment_status_transition(&current.status, status)?;

    conn.execute(
        "UPDATE experiment
         SET option_id = ?1,
             title = ?2,
             phase = ?3,
             mode = ?4,
             hypothesis = ?5,
             setup = ?6,
             observation_goal = ?7,
             rationale = ?8,
             primary_metrics_json = ?9,
             secondary_metrics_json = ?10,
             pass_criteria = ?11,
             fail_criteria = ?12,
             allowed_next_steps = ?13,
             blocked_next_steps = ?14,
             status = ?15,
             updated_at = datetime('now')
         WHERE id = ?16",
        params![
            option_id,
            title,
            phase,
            mode,
            hypothesis,
            setup,
            observation_goal,
            rationale,
            primary_metrics_json,
            secondary_metrics_json,
            pass_criteria,
            fail_criteria,
            allowed_next_steps,
            blocked_next_steps,
            status,
            id
        ],
    )
    .with_context(|| format!("failed to update experiment id {id}"))?;
    record_event(conn, "experiment", id, "update", "{}")?;
    get_experiment_by_id(conn, id)
}

pub fn update_experiment_status(
    conn: &Connection,
    id: i64,
    status: ExperimentStatus,
) -> anyhow::Result<Experiment> {
    let current = get_experiment_by_id(conn, id)?;
    validate_experiment_status_transition(&current.status, status.as_str())?;
    conn.execute(
        "UPDATE experiment SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![status.as_str(), id],
    )
    .with_context(|| format!("failed to update experiment id {id}"))?;
    record_event(conn, "experiment", id, "update_status", "{}")?;
    get_experiment_by_id(conn, id)
}

pub fn experiment_has_decision(conn: &Connection, experiment_id: i64) -> anyhow::Result<bool> {
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM decision WHERE experiment_id = ?1",
            params![experiment_id],
            |row| row.get(0),
        )
        .with_context(|| format!("failed to count decisions for experiment id {experiment_id}"))?;
    Ok(count > 0)
}

pub fn create_decision(conn: &Connection, input: &NewDecision<'_>) -> anyhow::Result<Decision> {
    let experiment = get_experiment_by_id(conn, input.experiment_id)?;
    let branch = get_branch_by_id(conn, experiment.branch_id)?;
    let proposed_options = parse_proposed_options_json(input.proposed_options_json)?;
    validate_next_branch(conn, branch.program_id, input.next_branch_id)?;
    validate_next_experiment(conn, branch.program_id, input.next_experiment_id)?;

    let tx = conn
        .unchecked_transaction()
        .context("failed to begin decision transaction")?;
    tx.execute(
        "INSERT INTO decision
         (experiment_id, result_summary, interpretation, limitations, decision, confidence,
          next_branch_id, next_experiment_id, proposed_options_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            input.experiment_id,
            input.result_summary,
            input.interpretation,
            input.limitations,
            input.decision.as_str(),
            input.confidence.as_str(),
            input.next_branch_id,
            input.next_experiment_id,
            input.proposed_options_json
        ],
    )
    .with_context(|| {
        format!(
            "failed to create decision for experiment id {}",
            input.experiment_id
        )
    })?;

    let id = tx.last_insert_rowid();
    record_event(&tx, "decision", id, "create", "{}")?;

    for option in &proposed_options {
        let classification = option
            .classification
            .as_deref()
            .map(str::parse::<ResearchOptionClassification>)
            .transpose()
            .with_context(|| format!("invalid classification for proposed option {}", option.slug))?
            .unwrap_or(ResearchOptionClassification::Exploratory);
        create_research_option(
            &tx,
            &NewResearchOption {
                program_id: branch.program_id,
                branch_id: Some(branch.id),
                open_question_id: None,
                source_experiment_id: Some(experiment.id),
                source_decision_id: Some(id),
                slug: &option.slug,
                title: &option.slug,
                hypothesis: Some(&option.description),
                description: &option.description,
                classification,
                status: ResearchOptionStatus::Open,
            },
        )?;
    }

    update_branch_decision_summary(
        &tx,
        branch.id,
        input.decision.as_str(),
        input.result_summary,
    )?;
    let decision = get_decision_by_id(&tx, id)?;
    tx.commit()
        .context("failed to commit decision transaction")?;
    Ok(decision)
}

pub fn get_decision_by_id(conn: &Connection, id: i64) -> anyhow::Result<Decision> {
    conn.query_row(
        "SELECT * FROM decision WHERE id = ?1",
        params![id],
        Decision::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read decision id {id}"))?
    .with_context(|| format!("decision id {id} not found"))
}

pub fn list_decisions_by_experiment(
    conn: &Connection,
    experiment_id: i64,
) -> anyhow::Result<Vec<Decision>> {
    let mut stmt = conn
        .prepare("SELECT * FROM decision WHERE experiment_id = ?1 ORDER BY created_at, id")
        .context("failed to prepare decision list query")?;
    let decisions = stmt
        .query_map(params![experiment_id], Decision::from_row)
        .context("failed to query decisions")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read decisions")?;
    Ok(decisions)
}

pub fn latest_decision_for_experiment(
    conn: &Connection,
    experiment_id: i64,
) -> anyhow::Result<Option<Decision>> {
    conn.query_row(
        "SELECT * FROM decision
         WHERE experiment_id = ?1
         ORDER BY created_at DESC, id DESC
         LIMIT 1",
        params![experiment_id],
        Decision::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read latest decision for experiment id {experiment_id}"))
}

pub fn create_run(conn: &Connection, input: &NewRun<'_>) -> anyhow::Result<Run> {
    conn.execute(
        "INSERT INTO run (experiment_id, command, environment_json, dataset, code_ref, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.experiment_id,
            input.command,
            input.environment_json,
            input.dataset,
            input.code_ref,
            input.notes
        ],
    )
    .with_context(|| {
        format!(
            "failed to create run for experiment id {}",
            input.experiment_id
        )
    })?;

    let id = conn.last_insert_rowid();
    record_event(conn, "run", id, "create", "{}")?;
    get_run_by_id(conn, id)
}

pub fn get_run_by_id(conn: &Connection, id: i64) -> anyhow::Result<Run> {
    conn.query_row(
        "SELECT * FROM run WHERE id = ?1",
        params![id],
        Run::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read run id {id}"))?
    .with_context(|| format!("run id {id} not found"))
}

pub fn list_runs_by_experiment(conn: &Connection, experiment_id: i64) -> anyhow::Result<Vec<Run>> {
    let mut stmt = conn
        .prepare("SELECT * FROM run WHERE experiment_id = ?1 ORDER BY started_at, id")
        .context("failed to prepare run list query")?;
    let runs = stmt
        .query_map(params![experiment_id], Run::from_row)
        .context("failed to query runs")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read runs")?;
    Ok(runs)
}

pub fn update_run_status(
    conn: &Connection,
    id: i64,
    update: &RunStatusUpdate<'_>,
) -> anyhow::Result<Run> {
    let current = get_run_by_id(conn, id)?;
    validate_run_status_transition(&current.status, update.status.as_str())?;

    let completed_at_sql = if is_terminal_run_status(update.status.as_str()) {
        "COALESCE(completed_at, datetime('now'))"
    } else {
        "NULL"
    };

    conn.execute(
        &format!(
            "UPDATE run
             SET status = ?1, completed_at = {completed_at_sql}, notes = COALESCE(?2, notes)
             WHERE id = ?3"
        ),
        params![update.status.as_str(), update.notes, id],
    )
    .with_context(|| format!("failed to update run id {id} status"))?;
    record_event(conn, "run", id, "update_status", "{}")?;
    get_run_by_id(conn, id)
}

pub fn finish_run(
    conn: &Connection,
    id: i64,
    status: RunStatus,
    notes: Option<&str>,
) -> anyhow::Result<Run> {
    if status == RunStatus::Running || status == RunStatus::Failed {
        bail!("finish run requires success or partial status");
    }

    update_run_status(conn, id, &RunStatusUpdate { status, notes })
}

pub fn fail_run(conn: &Connection, id: i64, notes: Option<&str>) -> anyhow::Result<Run> {
    update_run_status(
        conn,
        id,
        &RunStatusUpdate {
            status: RunStatus::Failed,
            notes,
        },
    )
}

pub fn create_metric(conn: &Connection, input: &NewMetric<'_>) -> anyhow::Result<Metric> {
    let higher_is_better = input
        .higher_is_better
        .map(|value| if value { 1_i64 } else { 0_i64 });
    conn.execute(
        "INSERT INTO metric (run_id, name, value, unit, higher_is_better, split, metadata_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            input.run_id,
            input.name,
            input.value,
            input.unit,
            higher_is_better,
            input.split,
            input.metadata_json
        ],
    )
    .with_context(|| {
        format!(
            "failed to create metric {} for run id {}",
            input.name, input.run_id
        )
    })?;

    let id = conn.last_insert_rowid();
    record_event(conn, "metric", id, "create", "{}")?;
    get_metric_by_id(conn, id)
}

pub fn get_metric_by_id(conn: &Connection, id: i64) -> anyhow::Result<Metric> {
    conn.query_row(
        "SELECT * FROM metric WHERE id = ?1",
        params![id],
        Metric::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read metric id {id}"))?
    .with_context(|| format!("metric id {id} not found"))
}

pub fn list_metrics_by_run(conn: &Connection, run_id: i64) -> anyhow::Result<Vec<Metric>> {
    let mut stmt = conn
        .prepare("SELECT * FROM metric WHERE run_id = ?1 ORDER BY id")
        .context("failed to prepare metric list query")?;
    let metrics = stmt
        .query_map(params![run_id], Metric::from_row)
        .context("failed to query metrics")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read metrics")?;
    Ok(metrics)
}

pub fn list_metrics_by_experiment(
    conn: &Connection,
    experiment_id: i64,
) -> anyhow::Result<Vec<Metric>> {
    let mut stmt = conn
        .prepare(
            "SELECT metric.*
             FROM metric
             JOIN run ON run.id = metric.run_id
             WHERE run.experiment_id = ?1
             ORDER BY run.started_at, run.id, metric.id",
        )
        .context("failed to prepare experiment metric list query")?;
    let metrics = stmt
        .query_map(params![experiment_id], Metric::from_row)
        .context("failed to query experiment metrics")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read experiment metrics")?;
    Ok(metrics)
}

pub fn list_metrics_by_name(conn: &Connection, name: &str) -> anyhow::Result<Vec<Metric>> {
    let mut stmt = conn
        .prepare(
            "SELECT *
             FROM metric
             WHERE name = ?1
             ORDER BY id",
        )
        .context("failed to prepare metric name list query")?;
    let metrics = stmt
        .query_map(params![name], Metric::from_row)
        .context("failed to query metrics by name")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read metrics by name")?;
    Ok(metrics)
}

pub fn create_research_matrix(
    conn: &Connection,
    input: &NewResearchMatrix<'_>,
) -> anyhow::Result<ResearchMatrix> {
    conn.execute(
        "INSERT INTO research_matrix (program_id, slug, title, description, status)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            input.program_id,
            input.slug,
            input.title,
            input.description,
            input.status.as_str()
        ],
    )
    .with_context(|| format!("failed to create matrix {}", input.slug))?;
    let id = conn.last_insert_rowid();
    record_event(conn, "research_matrix", id, "create", "{}")?;
    get_research_matrix_by_id(conn, id)
}

pub fn get_research_matrix_by_id(conn: &Connection, id: i64) -> anyhow::Result<ResearchMatrix> {
    conn.query_row(
        "SELECT * FROM research_matrix WHERE id = ?1",
        params![id],
        ResearchMatrix::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read matrix id {id}"))?
    .with_context(|| format!("matrix id {id} not found"))
}

pub fn get_research_matrix_by_slug(
    conn: &Connection,
    program_id: i64,
    slug: &str,
) -> anyhow::Result<Option<ResearchMatrix>> {
    conn.query_row(
        "SELECT * FROM research_matrix WHERE program_id = ?1 AND slug = ?2",
        params![program_id, slug],
        ResearchMatrix::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read matrix {slug}"))
}

pub fn list_research_matrices(
    conn: &Connection,
    program_id: i64,
) -> anyhow::Result<Vec<ResearchMatrix>> {
    let mut stmt = conn
        .prepare("SELECT * FROM research_matrix WHERE program_id = ?1 ORDER BY created_at, id")
        .context("failed to prepare matrix list query")?;
    let matrices = stmt
        .query_map(params![program_id], ResearchMatrix::from_row)
        .context("failed to query matrices")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read matrices")?;
    Ok(matrices)
}

pub fn update_research_matrix(
    conn: &Connection,
    id: i64,
    update: &ResearchMatrixUpdate<'_>,
) -> anyhow::Result<ResearchMatrix> {
    let current = get_research_matrix_by_id(conn, id)?;
    let title = update.title.unwrap_or(&current.title);
    let description = update.description.unwrap_or(&current.description);
    let status = update
        .status
        .map(crate::schema::MatrixStatus::as_str)
        .unwrap_or(&current.status);
    conn.execute(
        "UPDATE research_matrix
         SET title = ?1, description = ?2, status = ?3, updated_at = datetime('now')
         WHERE id = ?4",
        params![title, description, status, id],
    )
    .with_context(|| format!("failed to update matrix id {id}"))?;
    record_event(conn, "research_matrix", id, "update", "{}")?;
    get_research_matrix_by_id(conn, id)
}

pub fn create_matrix_axis(
    conn: &Connection,
    input: &NewMatrixAxis<'_>,
) -> anyhow::Result<MatrixAxis> {
    conn.execute(
        "INSERT INTO matrix_axis (matrix_id, slug, title, position)
         VALUES (?1, ?2, ?3, ?4)",
        params![input.matrix_id, input.slug, input.title, input.position],
    )
    .with_context(|| format!("failed to create matrix axis {}", input.slug))?;
    let id = conn.last_insert_rowid();
    record_event(conn, "matrix_axis", id, "create", "{}")?;
    get_matrix_axis_by_id(conn, id)
}

pub fn get_matrix_axis_by_id(conn: &Connection, id: i64) -> anyhow::Result<MatrixAxis> {
    conn.query_row(
        "SELECT * FROM matrix_axis WHERE id = ?1",
        params![id],
        MatrixAxis::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read matrix axis id {id}"))?
    .with_context(|| format!("matrix axis id {id} not found"))
}

pub fn get_matrix_axis_by_slug(
    conn: &Connection,
    matrix_id: i64,
    slug: &str,
) -> anyhow::Result<Option<MatrixAxis>> {
    conn.query_row(
        "SELECT * FROM matrix_axis WHERE matrix_id = ?1 AND slug = ?2",
        params![matrix_id, slug],
        MatrixAxis::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read matrix axis {slug}"))
}

pub fn list_matrix_axes(conn: &Connection, matrix_id: i64) -> anyhow::Result<Vec<MatrixAxis>> {
    let mut stmt = conn
        .prepare("SELECT * FROM matrix_axis WHERE matrix_id = ?1 ORDER BY position, id")
        .context("failed to prepare matrix axis list query")?;
    let axes = stmt
        .query_map(params![matrix_id], MatrixAxis::from_row)
        .context("failed to query matrix axes")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read matrix axes")?;
    Ok(axes)
}

pub fn next_matrix_axis_position(conn: &Connection, matrix_id: i64) -> anyhow::Result<i64> {
    conn.query_row(
        "SELECT coalesce(max(position), 0) + 1 FROM matrix_axis WHERE matrix_id = ?1",
        params![matrix_id],
        |row| row.get(0),
    )
    .context("failed to compute next matrix axis position")
}

pub fn create_matrix_level(
    conn: &Connection,
    input: &NewMatrixLevel<'_>,
) -> anyhow::Result<MatrixLevel> {
    conn.execute(
        "INSERT INTO matrix_level (axis_id, slug, title, position)
         VALUES (?1, ?2, ?3, ?4)",
        params![input.axis_id, input.slug, input.title, input.position],
    )
    .with_context(|| format!("failed to create matrix level {}", input.slug))?;
    let id = conn.last_insert_rowid();
    record_event(conn, "matrix_level", id, "create", "{}")?;
    get_matrix_level_by_id(conn, id)
}

pub fn get_matrix_level_by_id(conn: &Connection, id: i64) -> anyhow::Result<MatrixLevel> {
    conn.query_row(
        "SELECT * FROM matrix_level WHERE id = ?1",
        params![id],
        MatrixLevel::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read matrix level id {id}"))?
    .with_context(|| format!("matrix level id {id} not found"))
}

pub fn list_matrix_levels(conn: &Connection, axis_id: i64) -> anyhow::Result<Vec<MatrixLevel>> {
    let mut stmt = conn
        .prepare("SELECT * FROM matrix_level WHERE axis_id = ?1 ORDER BY position, id")
        .context("failed to prepare matrix level list query")?;
    let levels = stmt
        .query_map(params![axis_id], MatrixLevel::from_row)
        .context("failed to query matrix levels")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read matrix levels")?;
    Ok(levels)
}

pub fn next_matrix_level_position(conn: &Connection, axis_id: i64) -> anyhow::Result<i64> {
    conn.query_row(
        "SELECT coalesce(max(position), 0) + 1 FROM matrix_level WHERE axis_id = ?1",
        params![axis_id],
        |row| row.get(0),
    )
    .context("failed to compute next matrix level position")
}

pub fn create_matrix_cell(
    conn: &Connection,
    input: &NewMatrixCell<'_>,
) -> anyhow::Result<MatrixCell> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO matrix_cell (matrix_id, slug, title, coordinates_json)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            input.matrix_id,
            input.slug,
            input.title,
            input.coordinates_json
        ],
    )
    .with_context(|| format!("failed to create matrix cell {}", input.slug))?;
    let id = tx.last_insert_rowid();
    for (axis_id, level_id) in input.level_ids_by_axis {
        tx.execute(
            "INSERT INTO matrix_cell_level (cell_id, axis_id, level_id)
             VALUES (?1, ?2, ?3)",
            params![id, axis_id, level_id],
        )
        .with_context(|| format!("failed to link matrix cell {} coordinate", input.slug))?;
    }
    tx.execute(
        "INSERT INTO event_log (entity_type, entity_id, action, payload_json)
         VALUES ('matrix_cell', ?1, 'create', '{}')",
        params![id],
    )
    .context("failed to record matrix cell create event")?;
    tx.commit().context("failed to commit matrix cell create")?;
    get_matrix_cell_by_id(conn, id)
}

pub fn get_matrix_cell_by_id(conn: &Connection, id: i64) -> anyhow::Result<MatrixCell> {
    conn.query_row(
        "SELECT * FROM matrix_cell WHERE id = ?1",
        params![id],
        MatrixCell::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read matrix cell id {id}"))?
    .with_context(|| format!("matrix cell id {id} not found"))
}

pub fn get_matrix_cell_by_slug(
    conn: &Connection,
    matrix_id: i64,
    slug: &str,
) -> anyhow::Result<Option<MatrixCell>> {
    conn.query_row(
        "SELECT * FROM matrix_cell WHERE matrix_id = ?1 AND slug = ?2",
        params![matrix_id, slug],
        MatrixCell::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read matrix cell {slug}"))
}

pub fn list_matrix_cells(conn: &Connection, matrix_id: i64) -> anyhow::Result<Vec<MatrixCell>> {
    let mut stmt = conn
        .prepare("SELECT * FROM matrix_cell WHERE matrix_id = ?1 ORDER BY slug")
        .context("failed to prepare matrix cell list query")?;
    let cells = stmt
        .query_map(params![matrix_id], MatrixCell::from_row)
        .context("failed to query matrix cells")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read matrix cells")?;
    Ok(cells)
}

pub fn update_matrix_cell(
    conn: &Connection,
    id: i64,
    update: &MatrixCellUpdate<'_>,
) -> anyhow::Result<MatrixCell> {
    let current = get_matrix_cell_by_id(conn, id)?;
    let experiment_id = match update.experiment_id {
        Some(value) => value,
        None => current.experiment_id,
    };
    let status = update
        .status
        .map(|status| status.as_str().to_owned())
        .unwrap_or(current.status);
    let notes = match update.notes {
        Some(value) => value.map(str::to_owned),
        None => current.notes,
    };
    conn.execute(
        "UPDATE matrix_cell
         SET experiment_id = ?1, status = ?2, notes = ?3, updated_at = datetime('now')
         WHERE id = ?4",
        params![experiment_id, status, notes, id],
    )
    .with_context(|| format!("failed to update matrix cell id {id}"))?;
    record_event(conn, "matrix_cell", id, "update", "{}")?;
    get_matrix_cell_by_id(conn, id)
}

pub fn find_experiment_by_slug_in_program(
    conn: &Connection,
    program_id: i64,
    slug: &str,
) -> anyhow::Result<Option<Experiment>> {
    let mut stmt = conn
        .prepare(
            "SELECT experiment.*
             FROM experiment
             JOIN branch ON branch.id = experiment.branch_id
             WHERE branch.program_id = ?1 AND experiment.slug = ?2
             ORDER BY experiment.id",
        )
        .context("failed to prepare program experiment lookup")?;
    let experiments = stmt
        .query_map(params![program_id, slug], Experiment::from_row)
        .context("failed to query program experiment")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read program experiment")?;
    if experiments.len() > 1 {
        bail!("experiment slug {slug} is ambiguous in this program; use unique experiment slugs for matrix links");
    }
    Ok(experiments.into_iter().next())
}

pub fn create_artifact(conn: &Connection, input: &NewArtifact<'_>) -> anyhow::Result<Artifact> {
    conn.execute(
        "INSERT INTO artifact (run_id, kind, path, description, checksum, metadata_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.run_id,
            input.kind.as_str(),
            input.path,
            input.description,
            input.checksum,
            input.metadata_json
        ],
    )
    .with_context(|| {
        format!(
            "failed to create artifact {} for run id {}",
            input.path, input.run_id
        )
    })?;

    let id = conn.last_insert_rowid();
    record_event(conn, "artifact", id, "create", "{}")?;
    get_artifact_by_id(conn, id)
}

pub fn get_artifact_by_id(conn: &Connection, id: i64) -> anyhow::Result<Artifact> {
    conn.query_row(
        "SELECT * FROM artifact WHERE id = ?1",
        params![id],
        Artifact::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read artifact id {id}"))?
    .with_context(|| format!("artifact id {id} not found"))
}

pub fn list_artifacts_by_run(conn: &Connection, run_id: i64) -> anyhow::Result<Vec<Artifact>> {
    let mut stmt = conn
        .prepare("SELECT * FROM artifact WHERE run_id = ?1 ORDER BY id")
        .context("failed to prepare artifact list query")?;
    let artifacts = stmt
        .query_map(params![run_id], Artifact::from_row)
        .context("failed to query artifacts")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read artifacts")?;
    Ok(artifacts)
}

pub fn list_artifacts_by_experiment(
    conn: &Connection,
    experiment_id: i64,
) -> anyhow::Result<Vec<Artifact>> {
    let mut stmt = conn
        .prepare(
            "SELECT artifact.*
             FROM artifact
             JOIN run ON run.id = artifact.run_id
             WHERE run.experiment_id = ?1
             ORDER BY run.started_at, run.id, artifact.id",
        )
        .context("failed to prepare experiment artifact list query")?;
    let artifacts = stmt
        .query_map(params![experiment_id], Artifact::from_row)
        .context("failed to query experiment artifacts")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read experiment artifacts")?;
    Ok(artifacts)
}

fn validate_experiment_status_transition(current: &str, next: &str) -> anyhow::Result<()> {
    if current == next {
        return Ok(());
    }

    let allowed = matches!(
        (current, next),
        ("planned", "running")
            | ("planned", "superseded")
            | ("running", "completed")
            | ("running", "failed")
            | ("running", "inconclusive")
            | ("running", "superseded")
    );

    if !allowed {
        bail!("invalid experiment status transition from {current} to {next}");
    }
    Ok(())
}

fn validate_run_status_transition(current: &str, next: &str) -> anyhow::Result<()> {
    if current == next {
        return Ok(());
    }

    let allowed = matches!(
        (current, next),
        ("running", "success") | ("running", "failed") | ("running", "partial")
    );

    if !allowed {
        bail!("invalid run status transition from {current} to {next}");
    }
    Ok(())
}

fn is_terminal_run_status(status: &str) -> bool {
    matches!(status, "success" | "failed" | "partial")
}

pub fn create_open_question(
    conn: &Connection,
    input: &NewOpenQuestion<'_>,
) -> anyhow::Result<OpenQuestion> {
    conn.execute(
        "INSERT INTO open_question (program_id, branch_id, slug, question, context, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.program_id,
            input.branch_id,
            input.slug,
            input.question,
            input.context,
            input.status.as_str()
        ],
    )
    .with_context(|| format!("failed to create open question {}", input.slug))?;

    let id = conn.last_insert_rowid();
    record_event(conn, "open_question", id, "create", "{}")?;
    get_open_question_by_id(conn, id)
}

pub fn get_open_question_by_id(conn: &Connection, id: i64) -> anyhow::Result<OpenQuestion> {
    conn.query_row(
        "SELECT * FROM open_question WHERE id = ?1",
        params![id],
        OpenQuestion::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read open question id {id}"))?
    .with_context(|| format!("open question id {id} not found"))
}

pub fn get_open_question_by_slug(
    conn: &Connection,
    program_id: i64,
    slug: &str,
) -> anyhow::Result<Option<OpenQuestion>> {
    conn.query_row(
        "SELECT * FROM open_question WHERE program_id = ?1 AND slug = ?2",
        params![program_id, slug],
        OpenQuestion::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read open question {slug}"))
}

pub fn list_open_questions(
    conn: &Connection,
    filter: &OpenQuestionFilter,
) -> anyhow::Result<Vec<OpenQuestion>> {
    let status = filter.status.map(OpenQuestionStatus::as_str);
    let mut stmt = conn
        .prepare(
            "SELECT * FROM open_question
             WHERE (?1 IS NULL OR program_id = ?1)
               AND (?2 IS NULL OR branch_id = ?2)
               AND (?3 IS NULL OR status = ?3)
             ORDER BY created_at, id",
        )
        .context("failed to prepare open question list query")?;
    let questions = stmt
        .query_map(
            params![filter.program_id, filter.branch_id, status],
            OpenQuestion::from_row,
        )
        .context("failed to query open questions")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read open questions")?;
    Ok(questions)
}

pub fn update_open_question(
    conn: &Connection,
    id: i64,
    update: &OpenQuestionUpdate<'_>,
) -> anyhow::Result<OpenQuestion> {
    let current = get_open_question_by_id(conn, id)?;
    let branch_id = update.branch_id.unwrap_or(current.branch_id);
    let question = update.question.unwrap_or(&current.question);
    let context = update.context.unwrap_or(&current.context);
    let status = update
        .status
        .map(OpenQuestionStatus::as_str)
        .unwrap_or(&current.status);

    conn.execute(
        "UPDATE open_question
         SET branch_id = ?1, question = ?2, context = ?3, status = ?4, updated_at = datetime('now')
         WHERE id = ?5",
        params![branch_id, question, context, status, id],
    )
    .with_context(|| format!("failed to update open question id {id}"))?;
    record_event(conn, "open_question", id, "update", "{}")?;
    get_open_question_by_id(conn, id)
}

pub fn update_open_question_status(
    conn: &Connection,
    id: i64,
    status: OpenQuestionStatus,
) -> anyhow::Result<OpenQuestion> {
    conn.execute(
        "UPDATE open_question SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![status.as_str(), id],
    )
    .with_context(|| format!("failed to update open question id {id} status"))?;
    record_event(conn, "open_question", id, "update_status", "{}")?;
    get_open_question_by_id(conn, id)
}

pub fn answer_open_question(conn: &Connection, id: i64) -> anyhow::Result<OpenQuestion> {
    update_open_question_status(conn, id, OpenQuestionStatus::Answered)
}

pub fn reject_open_question(conn: &Connection, id: i64) -> anyhow::Result<OpenQuestion> {
    update_open_question_status(conn, id, OpenQuestionStatus::Rejected)
}

pub fn supersede_open_question(conn: &Connection, id: i64) -> anyhow::Result<OpenQuestion> {
    update_open_question_status(conn, id, OpenQuestionStatus::Superseded)
}

pub fn create_research_option(
    conn: &Connection,
    input: &NewResearchOption<'_>,
) -> anyhow::Result<ResearchOption> {
    conn.execute(
        "INSERT INTO research_option
         (program_id, branch_id, open_question_id, source_experiment_id, source_decision_id,
          slug, title, hypothesis, description, classification, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            input.program_id,
            input.branch_id,
            input.open_question_id,
            input.source_experiment_id,
            input.source_decision_id,
            input.slug,
            input.title,
            input.hypothesis,
            input.description,
            input.classification.as_str(),
            input.status.as_str()
        ],
    )
    .with_context(|| format!("failed to create research option {}", input.slug))?;

    let id = conn.last_insert_rowid();
    record_event(conn, "research_option", id, "create", "{}")?;
    get_research_option_by_id(conn, id)
}

pub fn get_research_option_by_id(conn: &Connection, id: i64) -> anyhow::Result<ResearchOption> {
    conn.query_row(
        "SELECT * FROM research_option WHERE id = ?1",
        params![id],
        ResearchOption::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read research option id {id}"))?
    .with_context(|| format!("research option id {id} not found"))
}

pub fn get_research_option_by_slug(
    conn: &Connection,
    program_id: i64,
    slug: &str,
) -> anyhow::Result<Option<ResearchOption>> {
    conn.query_row(
        "SELECT * FROM research_option WHERE program_id = ?1 AND slug = ?2",
        params![program_id, slug],
        ResearchOption::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read research option {slug}"))
}

pub fn list_research_options(
    conn: &Connection,
    filter: &ResearchOptionFilter,
) -> anyhow::Result<Vec<ResearchOption>> {
    let status = filter.status.map(ResearchOptionStatus::as_str);
    let classification = filter
        .classification
        .map(ResearchOptionClassification::as_str);
    let mut stmt = conn
        .prepare(
            "SELECT * FROM research_option
             WHERE (?1 IS NULL OR program_id = ?1)
               AND (?2 IS NULL OR branch_id = ?2)
               AND (?3 IS NULL OR status = ?3)
               AND (?4 IS NULL OR classification = ?4)
             ORDER BY created_at, id",
        )
        .context("failed to prepare research option list query")?;
    let options = stmt
        .query_map(
            params![filter.program_id, filter.branch_id, status, classification],
            ResearchOption::from_row,
        )
        .context("failed to query research options")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read research options")?;
    Ok(options)
}

pub fn list_recommended_research_options(
    conn: &Connection,
    filter: &ResearchOptionFilter,
) -> anyhow::Result<Vec<ResearchOption>> {
    let branch_id = filter.branch_id;
    let classification = filter
        .classification
        .map(ResearchOptionClassification::as_str);
    let mut stmt = conn
        .prepare(
            "SELECT * FROM research_option
             WHERE (?1 IS NULL OR program_id = ?1)
               AND (?2 IS NULL OR branch_id = ?2)
               AND (?3 IS NULL OR classification = ?3)
               AND status = 'open'
               AND classification NOT IN ('blocked', 'long_running', 'maintenance')
	               AND NOT EXISTS (
	                 SELECT 1
	                   FROM experiment
	                  WHERE experiment.option_id = research_option.id
	                    AND experiment.status IN ('completed', 'inconclusive', 'failed', 'superseded')
               )
               AND NOT EXISTS (
                 SELECT 1
                   FROM experiment
                   JOIN branch ON branch.id = experiment.branch_id
                  WHERE branch.program_id = research_option.program_id
                    AND experiment.slug = research_option.slug
                    AND experiment.status IN ('completed', 'inconclusive', 'failed', 'superseded')
               )
             ORDER BY
               CASE classification
                 WHEN 'main_path' THEN 0
                 WHEN 'validation' THEN 1
                 WHEN 'exploratory' THEN 2
                 ELSE 3
               END,
               created_at,
               id",
        )
        .context("failed to prepare recommended research option query")?;
    let options = stmt
        .query_map(
            params![filter.program_id, branch_id, classification],
            ResearchOption::from_row,
        )
        .context("failed to query recommended research options")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read recommended research options")?;
    Ok(options)
}

pub fn update_research_option(
    conn: &Connection,
    id: i64,
    update: &ResearchOptionUpdate<'_>,
) -> anyhow::Result<ResearchOption> {
    let current = get_research_option_by_id(conn, id)?;
    let branch_id = update.branch_id.unwrap_or(current.branch_id);
    let open_question_id = update.open_question_id.unwrap_or(current.open_question_id);
    let source_experiment_id = update
        .source_experiment_id
        .unwrap_or(current.source_experiment_id);
    let source_decision_id = update
        .source_decision_id
        .unwrap_or(current.source_decision_id);
    let title = update.title.unwrap_or(&current.title);
    let hypothesis = match update.hypothesis {
        Some(value) => value.map(str::to_owned),
        None => current.hypothesis.as_deref().map(str::to_owned),
    };
    let description = update.description.unwrap_or(&current.description);
    let classification = update
        .classification
        .map(ResearchOptionClassification::as_str)
        .unwrap_or(&current.classification);
    let status = update
        .status
        .map(ResearchOptionStatus::as_str)
        .unwrap_or(&current.status);

    conn.execute(
        "UPDATE research_option
         SET branch_id = ?1,
             open_question_id = ?2,
             source_experiment_id = ?3,
             source_decision_id = ?4,
             title = ?5,
             hypothesis = ?6,
             description = ?7,
             classification = ?8,
             status = ?9,
             updated_at = datetime('now')
         WHERE id = ?10",
        params![
            branch_id,
            open_question_id,
            source_experiment_id,
            source_decision_id,
            title,
            hypothesis,
            description,
            classification,
            status,
            id
        ],
    )
    .with_context(|| format!("failed to update research option id {id}"))?;
    record_event(conn, "research_option", id, "update", "{}")?;
    get_research_option_by_id(conn, id)
}

pub fn update_research_option_status(
    conn: &Connection,
    id: i64,
    status: ResearchOptionStatus,
) -> anyhow::Result<ResearchOption> {
    conn.execute(
        "UPDATE research_option SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![status.as_str(), id],
    )
    .with_context(|| format!("failed to update research option id {id} status"))?;
    record_event(conn, "research_option", id, "update_status", "{}")?;
    get_research_option_by_id(conn, id)
}

pub fn select_research_option(
    conn: &Connection,
    id: i64,
    rationale: &str,
    selected_by: Option<&str>,
) -> anyhow::Result<ResearchOption> {
    if rationale.trim().is_empty() {
        bail!("selecting a research option requires a rationale");
    }

    let option = get_research_option_by_id(conn, id)?;
    let review_state = if option_selection_needs_review(&option.classification) {
        ReviewState::NeedsReview.as_str()
    } else {
        ReviewState::None.as_str()
    };

    conn.execute(
        "UPDATE research_option
         SET status = ?1,
             selection_rationale = ?2,
             selected_by = ?3,
             selected_at = datetime('now'),
             review_state = ?4,
             updated_at = datetime('now')
         WHERE id = ?5",
        params![
            ResearchOptionStatus::Selected.as_str(),
            rationale,
            selected_by,
            review_state,
            id
        ],
    )
    .with_context(|| format!("failed to select research option id {id}"))?;
    record_event(conn, "research_option", id, "select", "{}")?;

    if option_selection_needs_review(&option.classification) {
        create_review_item(
            conn,
            &NewReviewItem {
                entity_type: "research_option",
                entity_id: id,
                reason: "selected option classification needs review",
                state: ReviewItemState::NeedsReview,
            },
        )?;
    }

    get_research_option_by_id(conn, id)
}

pub fn reject_research_option(conn: &Connection, id: i64) -> anyhow::Result<ResearchOption> {
    update_research_option_status(conn, id, ResearchOptionStatus::Rejected)
}

pub fn supersede_research_option(conn: &Connection, id: i64) -> anyhow::Result<ResearchOption> {
    update_research_option_status(conn, id, ResearchOptionStatus::Superseded)
}

pub fn link_research_option_to_open_question(
    conn: &Connection,
    option_id: i64,
    open_question_id: i64,
) -> anyhow::Result<ResearchOption> {
    conn.execute(
        "UPDATE research_option
         SET open_question_id = ?1, updated_at = datetime('now')
         WHERE id = ?2",
        params![open_question_id, option_id],
    )
    .with_context(|| {
        format!(
            "failed to link research option id {option_id} to open question id {open_question_id}"
        )
    })?;
    record_event(
        conn,
        "research_option",
        option_id,
        "link_open_question",
        "{}",
    )?;
    get_research_option_by_id(conn, option_id)
}

pub fn create_fact(
    conn: &Connection,
    input: &NewFact<'_>,
    evidence: &[NewEvidenceLink<'_>],
) -> anyhow::Result<Fact> {
    if evidence.is_empty() {
        bail!("creating a fact requires at least one evidence link");
    }
    validate_fact_refs(conn, input)?;

    let tx = conn
        .unchecked_transaction()
        .context("failed to begin fact transaction")?;
    let review_state = if input.status == FactStatus::Candidate {
        ReviewState::NeedsReview
    } else {
        ReviewState::None
    };
    tx.execute(
        "INSERT INTO fact
         (program_id, branch_id, slug, statement, status, confidence,
          created_from_experiment_id, created_from_decision_id, review_state)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            input.program_id,
            input.branch_id,
            input.slug,
            input.statement,
            input.status.as_str(),
            input.confidence.map(|confidence| confidence.as_str()),
            input.created_from_experiment_id,
            input.created_from_decision_id,
            review_state.as_str()
        ],
    )
    .with_context(|| format!("failed to create fact {}", input.slug))?;

    let id = tx.last_insert_rowid();
    record_event(&tx, "fact", id, "create", "{}")?;
    let has_contradictory_evidence = evidence
        .iter()
        .any(|link| link.relation == EvidenceRelation::Contradicts);
    for link in evidence {
        create_evidence_link_for_subject(&tx, "fact", id, link)?;
    }
    if has_contradictory_evidence && input.status == FactStatus::Accepted {
        tx.execute(
            "UPDATE fact
             SET status = ?1, review_state = ?2, updated_at = datetime('now')
             WHERE id = ?3",
            params![
                FactStatus::Contested.as_str(),
                ReviewState::NeedsReview.as_str(),
                id
            ],
        )
        .with_context(|| format!("failed to contest fact id {id}"))?;
        record_event(&tx, "fact", id, "contest", "{}")?;
        create_review_item(
            &tx,
            &NewReviewItem {
                entity_type: "fact",
                entity_id: id,
                reason: "accepted fact has contradictory evidence",
                state: ReviewItemState::NeedsReview,
            },
        )?;
    } else if has_contradictory_evidence && input.status != FactStatus::Candidate {
        create_review_item(
            &tx,
            &NewReviewItem {
                entity_type: "fact",
                entity_id: id,
                reason: "fact has contradictory evidence",
                state: ReviewItemState::NeedsReview,
            },
        )?;
    }
    if review_state == ReviewState::NeedsReview {
        create_review_item(
            &tx,
            &NewReviewItem {
                entity_type: "fact",
                entity_id: id,
                reason: "candidate fact needs review",
                state: ReviewItemState::NeedsReview,
            },
        )?;
    }

    let fact = get_fact_by_id(&tx, id)?;
    tx.commit().context("failed to commit fact transaction")?;
    Ok(fact)
}

pub fn get_fact_by_id(conn: &Connection, id: i64) -> anyhow::Result<Fact> {
    conn.query_row(
        "SELECT * FROM fact WHERE id = ?1",
        params![id],
        Fact::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read fact id {id}"))?
    .with_context(|| format!("fact id {id} not found"))
}

pub fn get_fact_by_slug(
    conn: &Connection,
    program_id: i64,
    slug: &str,
) -> anyhow::Result<Option<Fact>> {
    conn.query_row(
        "SELECT * FROM fact WHERE program_id = ?1 AND slug = ?2",
        params![program_id, slug],
        Fact::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read fact {slug}"))
}

pub fn list_facts(conn: &Connection, filter: &FactFilter) -> anyhow::Result<Vec<Fact>> {
    let status = filter.status.map(FactStatus::as_str);
    let review_state = filter.review_state.map(ReviewState::as_str);
    let mut stmt = conn
        .prepare(
            "SELECT * FROM fact
             WHERE (?1 IS NULL OR program_id = ?1)
               AND (?2 IS NULL OR branch_id = ?2)
               AND (?3 IS NULL OR status = ?3)
               AND (?4 IS NULL OR review_state = ?4)
             ORDER BY created_at, id",
        )
        .context("failed to prepare fact list query")?;
    let facts = stmt
        .query_map(
            params![filter.program_id, filter.branch_id, status, review_state],
            Fact::from_row,
        )
        .context("failed to query facts")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read facts")?;
    Ok(facts)
}

pub fn update_fact(conn: &Connection, id: i64, update: &FactUpdate<'_>) -> anyhow::Result<Fact> {
    if let Some(review_state) = update.review_state {
        validate_knowledge_review_state(review_state)?;
    }
    let current = get_fact_by_id(conn, id)?;
    let branch_id = update.branch_id.unwrap_or(current.branch_id);
    let statement = update.statement.unwrap_or(&current.statement);
    let status = update
        .status
        .map(FactStatus::as_str)
        .unwrap_or(&current.status);
    let confidence = match update.confidence {
        Some(value) => value.map(|confidence| confidence.as_str().to_owned()),
        None => current.confidence.as_deref().map(str::to_owned),
    };
    let created_from_experiment_id = update
        .created_from_experiment_id
        .unwrap_or(current.created_from_experiment_id);
    let created_from_decision_id = update
        .created_from_decision_id
        .unwrap_or(current.created_from_decision_id);
    let review_state = update
        .review_state
        .map(ReviewState::as_str)
        .unwrap_or(&current.review_state);

    conn.execute(
        "UPDATE fact
         SET branch_id = ?1,
             statement = ?2,
             status = ?3,
             confidence = ?4,
             created_from_experiment_id = ?5,
             created_from_decision_id = ?6,
             review_state = ?7,
             updated_at = datetime('now')
         WHERE id = ?8",
        params![
            branch_id,
            statement,
            status,
            confidence,
            created_from_experiment_id,
            created_from_decision_id,
            review_state,
            id
        ],
    )
    .with_context(|| format!("failed to update fact id {id}"))?;
    record_event(conn, "fact", id, "update", "{}")?;
    get_fact_by_id(conn, id)
}

pub fn add_fact_evidence(
    conn: &Connection,
    fact_id: i64,
    input: &NewEvidenceLink<'_>,
) -> anyhow::Result<EvidenceLink> {
    let fact = get_fact_by_id(conn, fact_id)?;
    let tx = conn
        .unchecked_transaction()
        .context("failed to begin fact evidence transaction")?;
    let link = create_evidence_link_for_subject(&tx, "fact", fact_id, input)?;
    if input.relation == EvidenceRelation::Contradicts {
        if fact.status == FactStatus::Accepted.as_str() {
            tx.execute(
                "UPDATE fact
                 SET status = ?1, review_state = ?2, updated_at = datetime('now')
                 WHERE id = ?3",
                params![
                    FactStatus::Contested.as_str(),
                    ReviewState::NeedsReview.as_str(),
                    fact_id
                ],
            )
            .with_context(|| format!("failed to contest fact id {fact_id}"))?;
            record_event(&tx, "fact", fact_id, "contest", "{}")?;
            create_review_item(
                &tx,
                &NewReviewItem {
                    entity_type: "fact",
                    entity_id: fact_id,
                    reason: "accepted fact has contradictory evidence",
                    state: ReviewItemState::NeedsReview,
                },
            )?;
        } else {
            create_review_item(
                &tx,
                &NewReviewItem {
                    entity_type: "fact",
                    entity_id: fact_id,
                    reason: "fact has contradictory evidence",
                    state: ReviewItemState::NeedsReview,
                },
            )?;
        }
    }
    tx.commit()
        .context("failed to commit fact evidence transaction")?;
    Ok(link)
}

pub fn create_axiom(conn: &Connection, input: &NewAxiom<'_>) -> anyhow::Result<Axiom> {
    validate_axiom_refs(conn, input)?;
    let tx = conn
        .unchecked_transaction()
        .context("failed to begin axiom transaction")?;
    let review_state = if input.created_by_agent {
        ReviewState::NeedsReview
    } else {
        ReviewState::None
    };
    tx.execute(
        "INSERT INTO axiom
         (program_id, branch_id, slug, statement, status, created_by_actor, created_by_agent,
          review_state)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            input.program_id,
            input.branch_id,
            input.slug,
            input.statement,
            input.status.as_str(),
            input.created_by_actor,
            if input.created_by_agent { 1_i64 } else { 0_i64 },
            review_state.as_str()
        ],
    )
    .with_context(|| format!("failed to create axiom {}", input.slug))?;

    let id = tx.last_insert_rowid();
    record_event(&tx, "axiom", id, "create", "{}")?;
    if input.created_by_agent {
        create_review_item(
            &tx,
            &NewReviewItem {
                entity_type: "axiom",
                entity_id: id,
                reason: "agent-created axiom needs review",
                state: ReviewItemState::NeedsReview,
            },
        )?;
    }
    let axiom = get_axiom_by_id(&tx, id)?;
    tx.commit().context("failed to commit axiom transaction")?;
    Ok(axiom)
}

pub fn get_axiom_by_id(conn: &Connection, id: i64) -> anyhow::Result<Axiom> {
    conn.query_row(
        "SELECT * FROM axiom WHERE id = ?1",
        params![id],
        Axiom::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read axiom id {id}"))?
    .with_context(|| format!("axiom id {id} not found"))
}

pub fn get_axiom_by_slug(
    conn: &Connection,
    program_id: i64,
    slug: &str,
) -> anyhow::Result<Option<Axiom>> {
    conn.query_row(
        "SELECT * FROM axiom WHERE program_id = ?1 AND slug = ?2",
        params![program_id, slug],
        Axiom::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read axiom {slug}"))
}

pub fn list_axioms(conn: &Connection, filter: &AxiomFilter) -> anyhow::Result<Vec<Axiom>> {
    let status = filter.status.map(AxiomStatus::as_str);
    let review_state = filter.review_state.map(ReviewState::as_str);
    let mut stmt = conn
        .prepare(
            "SELECT * FROM axiom
             WHERE (?1 IS NULL OR program_id = ?1)
               AND (?2 IS NULL OR branch_id = ?2)
               AND (?3 IS NULL OR status = ?3)
               AND (?4 IS NULL OR review_state = ?4)
             ORDER BY created_at, id",
        )
        .context("failed to prepare axiom list query")?;
    let axioms = stmt
        .query_map(
            params![filter.program_id, filter.branch_id, status, review_state],
            Axiom::from_row,
        )
        .context("failed to query axioms")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read axioms")?;
    Ok(axioms)
}

pub fn update_axiom(conn: &Connection, id: i64, update: &AxiomUpdate<'_>) -> anyhow::Result<Axiom> {
    if let Some(review_state) = update.review_state {
        validate_knowledge_review_state(review_state)?;
    }
    let current = get_axiom_by_id(conn, id)?;
    if let Some(status) = update.status {
        if status.as_str() != current.status && update.approved_by.unwrap_or("").trim().is_empty() {
            bail!("axiom state changes require approved_by actor metadata");
        }
    }

    let branch_id = update.branch_id.unwrap_or(current.branch_id);
    let statement = update.statement.unwrap_or(&current.statement);
    let status = update
        .status
        .map(AxiomStatus::as_str)
        .unwrap_or(&current.status);
    let created_by_actor = match update.created_by_actor {
        Some(value) => value.map(str::to_owned),
        None => current.created_by_actor.as_deref().map(str::to_owned),
    };
    let review_state = update
        .review_state
        .map(ReviewState::as_str)
        .unwrap_or(&current.review_state);

    conn.execute(
        "UPDATE axiom
         SET branch_id = ?1,
             statement = ?2,
             status = ?3,
             created_by_actor = ?4,
             review_state = ?5,
             updated_at = datetime('now')
         WHERE id = ?6",
        params![
            branch_id,
            statement,
            status,
            created_by_actor,
            review_state,
            id
        ],
    )
    .with_context(|| format!("failed to update axiom id {id}"))?;
    if let Some(approved_by) = update.approved_by {
        let payload = serde_json::json!({ "approved_by": approved_by.trim() }).to_string();
        record_event(conn, "axiom", id, "update", &payload)?;
    } else {
        record_event(conn, "axiom", id, "update", "{}")?;
    }
    get_axiom_by_id(conn, id)
}

pub fn add_axiom_evidence(
    conn: &Connection,
    axiom_id: i64,
    input: &NewEvidenceLink<'_>,
) -> anyhow::Result<EvidenceLink> {
    get_axiom_by_id(conn, axiom_id)?;
    let tx = conn
        .unchecked_transaction()
        .context("failed to begin axiom evidence transaction")?;
    let link = create_evidence_link_for_subject(&tx, "axiom", axiom_id, input)?;
    if input.relation == EvidenceRelation::Contradicts {
        tx.execute(
            "UPDATE axiom
             SET status = ?1, review_state = ?2, updated_at = datetime('now')
             WHERE id = ?3",
            params![
                AxiomStatus::Contested.as_str(),
                ReviewState::NeedsReview.as_str(),
                axiom_id
            ],
        )
        .with_context(|| format!("failed to contest axiom id {axiom_id}"))?;
        record_event(&tx, "axiom", axiom_id, "contest", "{}")?;
        create_review_item(
            &tx,
            &NewReviewItem {
                entity_type: "axiom",
                entity_id: axiom_id,
                reason: "axiom has contradictory evidence",
                state: ReviewItemState::NeedsReview,
            },
        )?;
    }
    tx.commit()
        .context("failed to commit axiom evidence transaction")?;
    Ok(link)
}

pub fn list_evidence_links(
    conn: &Connection,
    subject_type: &str,
    subject_id: i64,
) -> anyhow::Result<Vec<EvidenceLink>> {
    let mut stmt = conn
        .prepare(
            "SELECT * FROM evidence_link
             WHERE subject_type = ?1 AND subject_id = ?2
             ORDER BY created_at, id",
        )
        .context("failed to prepare evidence link list query")?;
    let links = stmt
        .query_map(params![subject_type, subject_id], EvidenceLink::from_row)
        .context("failed to query evidence links")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read evidence links")?;
    Ok(links)
}

fn create_evidence_link_for_subject(
    conn: &Connection,
    subject_type: &str,
    subject_id: i64,
    input: &NewEvidenceLink<'_>,
) -> anyhow::Result<EvidenceLink> {
    validate_evidence_subject(subject_type)?;
    validate_evidence_targets(conn, input)?;
    conn.execute(
        "INSERT INTO evidence_link
         (subject_type, subject_id, relation, experiment_id, run_id, metric_id, artifact_id,
          decision_id, report_path, report_anchor, summary)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            subject_type,
            subject_id,
            input.relation.as_str(),
            input.experiment_id,
            input.run_id,
            input.metric_id,
            input.artifact_id,
            input.decision_id,
            input.report_path,
            input.report_anchor,
            input.summary
        ],
    )
    .with_context(|| format!("failed to create evidence link for {subject_type} {subject_id}"))?;

    let id = conn.last_insert_rowid();
    record_event(conn, "evidence_link", id, "create", "{}")?;
    get_evidence_link_by_id(conn, id)
}

pub fn get_evidence_link_by_id(conn: &Connection, id: i64) -> anyhow::Result<EvidenceLink> {
    conn.query_row(
        "SELECT * FROM evidence_link WHERE id = ?1",
        params![id],
        EvidenceLink::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read evidence link id {id}"))?
    .with_context(|| format!("evidence link id {id} not found"))
}

pub fn create_review_item(
    conn: &Connection,
    input: &NewReviewItem<'_>,
) -> anyhow::Result<ReviewItem> {
    conn.execute(
        "INSERT INTO review_item (entity_type, entity_id, reason, state)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            input.entity_type,
            input.entity_id,
            input.reason,
            input.state.as_str()
        ],
    )
    .with_context(|| {
        format!(
            "failed to create review item for {} {}",
            input.entity_type, input.entity_id
        )
    })?;

    let id = conn.last_insert_rowid();
    record_event(conn, "review_item", id, "create", "{}")?;
    get_review_item_by_id(conn, id)
}

pub fn get_review_item_by_id(conn: &Connection, id: i64) -> anyhow::Result<ReviewItem> {
    conn.query_row(
        "SELECT * FROM review_item WHERE id = ?1",
        params![id],
        ReviewItem::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read review item id {id}"))?
    .with_context(|| format!("review item id {id} not found"))
}

pub fn list_review_items(
    conn: &Connection,
    filter: &ReviewItemFilter,
) -> anyhow::Result<Vec<ReviewItem>> {
    let state = filter.state.map(ReviewItemState::as_str);
    let mut stmt = conn
        .prepare(
            "SELECT * FROM review_item
             WHERE (?1 IS NULL OR entity_type = ?1)
               AND (?2 IS NULL OR entity_id = ?2)
               AND (?3 IS NULL OR state = ?3)
             ORDER BY created_at, id",
        )
        .context("failed to prepare review item list query")?;
    let items = stmt
        .query_map(
            params![filter.entity_type.as_deref(), filter.entity_id, state],
            ReviewItem::from_row,
        )
        .context("failed to query review items")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read review items")?;
    Ok(items)
}

pub fn update_review_item_state(
    conn: &Connection,
    id: i64,
    state: ReviewItemState,
    reviewed_by: Option<&str>,
    notes: Option<&str>,
) -> anyhow::Result<ReviewItem> {
    let reviewed_at = if state == ReviewItemState::NeedsReview {
        None
    } else {
        Some("datetime('now')")
    };
    let reviewed_at_sql = reviewed_at.unwrap_or("NULL");
    let changed = conn
        .execute(
            &format!(
                "UPDATE review_item
             SET state = ?1, reviewed_at = {reviewed_at_sql}, reviewed_by = ?2, notes = ?3
             WHERE id = ?4"
            ),
            params![state.as_str(), reviewed_by, notes, id],
        )
        .with_context(|| format!("failed to update review item id {id} state"))?;
    if changed == 0 {
        bail!("review item id {id} not found");
    }
    record_event(conn, "review_item", id, "update_state", "{}")?;
    get_review_item_by_id(conn, id)
}

pub fn mark_review_item_reviewed(
    conn: &Connection,
    id: i64,
    reviewed_by: Option<&str>,
    notes: Option<&str>,
) -> anyhow::Result<ReviewItem> {
    update_review_item_state(conn, id, ReviewItemState::Reviewed, reviewed_by, notes)
}

pub fn dismiss_review_item(
    conn: &Connection,
    id: i64,
    dismissed_by: Option<&str>,
    notes: Option<&str>,
) -> anyhow::Result<ReviewItem> {
    update_review_item_state(conn, id, ReviewItemState::Dismissed, dismissed_by, notes)
}

pub fn request_override_approval(
    conn: &Connection,
    input: &NewOverrideApprovalRequest<'_>,
) -> anyhow::Result<OverrideApproval> {
    if input.justification.trim().is_empty() {
        bail!("override request requires a justification");
    }
    if input.blocked_work.trim().is_empty() {
        bail!("override request requires blocked-work text");
    }
    if input.requested_action.trim().is_empty() {
        bail!("override request requires a requested action");
    }

    conn.execute(
        "INSERT INTO override_approval
         (entity_type, entity_id, blocked_work, requested_action, justification, approved_by, approved_at)
         VALUES (?1, ?2, ?3, ?4, ?5, '', '')",
        params![
            input.entity_type,
            input.entity_id,
            input.blocked_work,
            input.requested_action,
            input.justification
        ],
    )
    .with_context(|| {
        format!(
            "failed to request override approval for {} {}",
            input.entity_type, input.entity_id
        )
    })?;

    let id = conn.last_insert_rowid();
    record_event(conn, "override_approval", id, "request", "{}")?;
    get_override_approval_by_id(conn, id)
}

pub fn get_override_approval_by_id(conn: &Connection, id: i64) -> anyhow::Result<OverrideApproval> {
    conn.query_row(
        "SELECT * FROM override_approval WHERE id = ?1",
        params![id],
        OverrideApproval::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read override approval id {id}"))?
    .with_context(|| format!("override approval id {id} not found"))
}

pub fn list_override_approvals(
    conn: &Connection,
    filter: &OverrideApprovalFilter,
) -> anyhow::Result<Vec<OverrideApproval>> {
    let mut stmt = conn
        .prepare(
            "SELECT * FROM override_approval
             WHERE (?1 IS NULL OR entity_type = ?1)
               AND (?2 IS NULL OR entity_id = ?2)
             ORDER BY created_at, id",
        )
        .context("failed to prepare override approval list query")?;
    let mut approvals = stmt
        .query_map(
            params![filter.entity_type.as_deref(), filter.entity_id],
            OverrideApproval::from_row,
        )
        .context("failed to query override approvals")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read override approvals")?;
    if let Some(status) = filter.status {
        approvals.retain(|approval| approval.status == status);
    }
    Ok(approvals)
}

pub fn approve_override_approval(
    conn: &Connection,
    id: i64,
    approved_by: &str,
) -> anyhow::Result<OverrideApproval> {
    if approved_by.trim().is_empty() {
        bail!("approving an override requires an approver");
    }
    let approval = get_override_approval_by_id(conn, id)?;
    if approval.status != OverrideApprovalStatus::Pending {
        bail!("override approval id {id} is not pending");
    }

    let changed = conn
        .execute(
            "UPDATE override_approval
             SET approved_by = ?1, approved_at = datetime('now')
             WHERE id = ?2",
            params![approved_by.trim(), id],
        )
        .with_context(|| format!("failed to approve override approval id {id}"))?;
    if changed == 0 {
        bail!("override approval id {id} not found");
    }
    record_event(conn, "override_approval", id, "approve", "{}")?;
    get_override_approval_by_id(conn, id)
}

pub fn reject_override_approval(
    conn: &Connection,
    id: i64,
    rejected_by: &str,
) -> anyhow::Result<OverrideApproval> {
    if rejected_by.trim().is_empty() {
        bail!("rejecting an override requires a reviewer");
    }
    let approval = get_override_approval_by_id(conn, id)?;
    if approval.status != OverrideApprovalStatus::Pending {
        bail!("override approval id {id} is not pending");
    }

    let rejected_marker = format!("rejected:{}", rejected_by.trim());
    let changed = conn
        .execute(
            "UPDATE override_approval
             SET approved_by = ?1, approved_at = datetime('now')
             WHERE id = ?2",
            params![rejected_marker, id],
        )
        .with_context(|| format!("failed to reject override approval id {id}"))?;
    if changed == 0 {
        bail!("override approval id {id} not found");
    }
    record_event(conn, "override_approval", id, "reject", "{}")?;
    get_override_approval_by_id(conn, id)
}

pub fn create_bug_report(conn: &Connection, input: &NewBugReport<'_>) -> anyhow::Result<BugReport> {
    if input.title.trim().is_empty() {
        bail!("bug report requires a title");
    }
    if input.description.trim().is_empty() {
        bail!("bug report requires a description");
    }

    conn.execute(
        "INSERT INTO bug_report
         (program_id, branch_id, experiment_id, title, description, severity, command, error,
          reproduction, log_path, log_excerpt, reported_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            input.program_id,
            input.branch_id,
            input.experiment_id,
            input.title.trim(),
            input.description.trim(),
            input.severity.as_str(),
            input.command.map(str::trim),
            input.error.map(str::trim),
            input.reproduction.map(str::trim),
            input.log_path.map(str::trim),
            input.log_excerpt.map(str::trim),
            input.reported_by.map(str::trim),
        ],
    )
    .context("failed to create bug report")?;

    let id = conn.last_insert_rowid();
    record_event(conn, "bug_report", id, "create", "{}")?;
    get_bug_report_by_id(conn, id)
}

pub fn get_bug_report_by_id(conn: &Connection, id: i64) -> anyhow::Result<BugReport> {
    conn.query_row(
        "SELECT * FROM bug_report WHERE id = ?1",
        params![id],
        BugReport::from_row,
    )
    .optional()
    .with_context(|| format!("failed to read bug report id {id}"))?
    .with_context(|| format!("bug report id {id} not found"))
}

pub fn list_bug_reports(
    conn: &Connection,
    filter: &BugReportFilter,
) -> anyhow::Result<Vec<BugReport>> {
    let status = filter.status.map(BugReportStatus::as_str);
    let mut stmt = conn
        .prepare(
            "SELECT * FROM bug_report
             WHERE (?1 IS NULL OR program_id = ?1)
               AND (?2 IS NULL OR branch_id = ?2)
               AND (?3 IS NULL OR experiment_id = ?3)
               AND (?4 IS NULL OR status = ?4)
             ORDER BY created_at, id",
        )
        .context("failed to prepare bug report list query")?;
    let reports = stmt
        .query_map(
            params![
                filter.program_id,
                filter.branch_id,
                filter.experiment_id,
                status
            ],
            BugReport::from_row,
        )
        .context("failed to query bug reports")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read bug reports")?;
    Ok(reports)
}

pub fn update_bug_report_status(
    conn: &Connection,
    id: i64,
    status: BugReportStatus,
    by: Option<&str>,
    notes: Option<&str>,
) -> anyhow::Result<BugReport> {
    let resolved_at_sql = if matches!(
        status,
        BugReportStatus::Resolved | BugReportStatus::Dismissed
    ) {
        "datetime('now')"
    } else {
        "NULL"
    };
    let changed = conn
        .execute(
            &format!(
                "UPDATE bug_report
                 SET status = ?1,
                     resolution_notes = ?2,
                     resolved_by = ?3,
                     resolved_at = {resolved_at_sql},
                     updated_at = datetime('now')
                 WHERE id = ?4"
            ),
            params![status.as_str(), notes, by, id],
        )
        .with_context(|| format!("failed to update bug report id {id}"))?;
    if changed == 0 {
        bail!("bug report id {id} not found");
    }
    record_event(conn, "bug_report", id, "update_status", "{}")?;
    get_bug_report_by_id(conn, id)
}

fn option_selection_needs_review(classification: &str) -> bool {
    matches!(
        classification,
        "long_running" | "exploratory" | "blocked" | "maintenance"
    )
}

fn validate_fact_refs(conn: &Connection, input: &NewFact<'_>) -> anyhow::Result<()> {
    get_program_by_id(conn, input.program_id)?;
    if let Some(branch_id) = input.branch_id {
        let branch = get_branch_by_id(conn, branch_id)?;
        if branch.program_id != input.program_id {
            bail!("fact branch id {branch_id} is not in the same program");
        }
    }
    if let Some(experiment_id) = input.created_from_experiment_id {
        validate_experiment_in_program(conn, input.program_id, experiment_id)?;
    }
    if let Some(decision_id) = input.created_from_decision_id {
        validate_decision_in_program(conn, input.program_id, decision_id)?;
    }
    Ok(())
}

fn validate_axiom_refs(conn: &Connection, input: &NewAxiom<'_>) -> anyhow::Result<()> {
    get_program_by_id(conn, input.program_id)?;
    if let Some(branch_id) = input.branch_id {
        let branch = get_branch_by_id(conn, branch_id)?;
        if branch.program_id != input.program_id {
            bail!("axiom branch id {branch_id} is not in the same program");
        }
    }
    Ok(())
}

fn validate_knowledge_review_state(review_state: ReviewState) -> anyhow::Result<()> {
    if matches!(
        review_state,
        ReviewState::None | ReviewState::NeedsReview | ReviewState::Reviewed
    ) {
        Ok(())
    } else {
        bail!(
            "knowledge review_state must be one of none, needs_review, reviewed, got {}",
            review_state.as_str()
        );
    }
}

fn validate_evidence_subject(subject_type: &str) -> anyhow::Result<()> {
    if matches!(subject_type, "fact" | "axiom") {
        Ok(())
    } else {
        bail!("evidence subject_type must be fact or axiom");
    }
}

fn validate_evidence_targets(conn: &Connection, input: &NewEvidenceLink<'_>) -> anyhow::Result<()> {
    if input.summary.trim().is_empty() {
        bail!("evidence link requires a summary");
    }
    if input.report_anchor.is_some() && input.report_path.is_none() {
        bail!("report anchor evidence requires a report path");
    }
    if input.experiment_id.is_none()
        && input.run_id.is_none()
        && input.metric_id.is_none()
        && input.artifact_id.is_none()
        && input.decision_id.is_none()
        && input.report_path.is_none()
    {
        bail!("evidence link requires at least one target");
    }

    if let Some(experiment_id) = input.experiment_id {
        get_experiment_by_id(conn, experiment_id)?;
    }
    if let Some(run_id) = input.run_id {
        get_run_by_id(conn, run_id)?;
    }
    if let Some(metric_id) = input.metric_id {
        get_metric_by_id(conn, metric_id)?;
    }
    if let Some(artifact_id) = input.artifact_id {
        get_artifact_by_id(conn, artifact_id)?;
    }
    if let Some(decision_id) = input.decision_id {
        get_decision_by_id(conn, decision_id)?;
    }
    Ok(())
}

fn validate_experiment_in_program(
    conn: &Connection,
    program_id: i64,
    experiment_id: i64,
) -> anyhow::Result<()> {
    let experiment = get_experiment_by_id(conn, experiment_id)?;
    let branch = get_branch_by_id(conn, experiment.branch_id)?;
    if branch.program_id != program_id {
        bail!("experiment id {experiment_id} is not in the same program");
    }
    Ok(())
}

fn validate_decision_in_program(
    conn: &Connection,
    program_id: i64,
    decision_id: i64,
) -> anyhow::Result<()> {
    let decision = get_decision_by_id(conn, decision_id)?;
    validate_experiment_in_program(conn, program_id, decision.experiment_id)
}

fn parse_proposed_options_json(value: &str) -> anyhow::Result<Vec<ProposedOptionJson>> {
    let options: Vec<ProposedOptionJson> =
        serde_json::from_str(value).context("failed to parse proposed decision options JSON")?;
    for option in &options {
        if option.slug.trim().is_empty() || option.description.trim().is_empty() {
            bail!("proposed options must include non-empty slug and description");
        }
        if let Some(classification) = option.classification.as_deref() {
            classification
                .parse::<ResearchOptionClassification>()
                .with_context(|| {
                    format!("invalid classification for proposed option {}", option.slug)
                })?;
        }
    }
    Ok(options)
}

fn validate_next_branch(
    conn: &Connection,
    program_id: i64,
    next_branch_id: Option<i64>,
) -> anyhow::Result<()> {
    if let Some(next_branch_id) = next_branch_id {
        let next_branch = get_branch_by_id(conn, next_branch_id)
            .with_context(|| format!("next branch id {next_branch_id} could not be resolved"))?;
        if next_branch.program_id != program_id {
            bail!("next branch id {next_branch_id} is not in the same program");
        }
    }
    Ok(())
}

fn validate_next_experiment(
    conn: &Connection,
    program_id: i64,
    next_experiment_id: Option<i64>,
) -> anyhow::Result<()> {
    if let Some(next_experiment_id) = next_experiment_id {
        let next_experiment =
            get_experiment_by_id(conn, next_experiment_id).with_context(|| {
                format!("next experiment id {next_experiment_id} could not be resolved")
            })?;
        let next_branch = get_branch_by_id(conn, next_experiment.branch_id)?;
        if next_branch.program_id != program_id {
            bail!("next experiment id {next_experiment_id} is not in the same program");
        }
    }
    Ok(())
}

fn update_branch_decision_summary(
    conn: &Connection,
    branch_id: i64,
    decision: &str,
    result_summary: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE branch
         SET decision_summary = ?1, updated_at = datetime('now')
         WHERE id = ?2",
        params![format!("{decision}: {result_summary}"), branch_id],
    )
    .with_context(|| format!("failed to update branch id {branch_id} decision summary"))?;
    record_event(conn, "branch", branch_id, "update_decision_summary", "{}")?;
    Ok(())
}

fn record_event(
    conn: &Connection,
    entity_type: &str,
    entity_id: i64,
    action: &str,
    payload_json: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO event_log (entity_type, entity_id, action, payload_json)
         VALUES (?1, ?2, ?3, ?4)",
        params![entity_type, entity_id, action, payload_json],
    )
    .with_context(|| format!("failed to record {action} event for {entity_type} {entity_id}"))?;
    Ok(())
}
