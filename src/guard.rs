#![allow(dead_code)]

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::Context;
use rusqlite::{params, Connection, OptionalExtension};

use crate::migrations;
use crate::policy::Policy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    Error,
    Warning,
}

impl fmt::Display for FindingSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => f.write_str("error"),
            Self::Warning => f.write_str("warning"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingType {
    MissingCurrentProgram,
    MissingCurrentBranch,
    CompletedExperimentMissingDecision,
    RunMetricsWithoutArtifacts,
    ArtifactOutsideAllowedRoots,
    ActiveBranchWithoutExperiment,
    LongRunningSelectionNeedsReview,
    PendingOverrideApproval,
}

impl fmt::Display for FindingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCurrentProgram => f.write_str("missing_current_program"),
            Self::MissingCurrentBranch => f.write_str("missing_current_branch"),
            Self::CompletedExperimentMissingDecision => {
                f.write_str("completed_experiment_missing_decision")
            }
            Self::RunMetricsWithoutArtifacts => f.write_str("run_metrics_without_artifacts"),
            Self::ArtifactOutsideAllowedRoots => f.write_str("artifact_outside_allowed_roots"),
            Self::ActiveBranchWithoutExperiment => f.write_str("active_branch_without_experiment"),
            Self::LongRunningSelectionNeedsReview => {
                f.write_str("long_running_selection_needs_review")
            }
            Self::PendingOverrideApproval => f.write_str("pending_override_approval"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: FindingSeverity,
    pub finding_type: FindingType,
    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub field: String,
    pub message: String,
}

impl Finding {
    fn new(
        severity: FindingSeverity,
        finding_type: FindingType,
        entity_type: impl Into<String>,
        entity_id: Option<i64>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            finding_type,
            entity_type: entity_type.into(),
            entity_id,
            field: field.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationPaths {
    pub project_root: PathBuf,
}

impl Default for ValidationPaths {
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
        }
    }
}

impl ValidationPaths {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }
}

pub fn validate(
    conn: &Connection,
    policy: &Policy,
    paths: &ValidationPaths,
) -> anyhow::Result<Vec<Finding>> {
    let mut findings = Vec::new();

    validate_current_refs(conn, policy, &mut findings)?;
    validate_completed_experiments_have_decisions(conn, policy, &mut findings)?;
    validate_runs_with_metrics_have_artifacts(conn, &mut findings)?;
    validate_artifact_roots(conn, policy, paths, &mut findings)?;
    validate_active_branches_have_work(conn, &mut findings)?;
    validate_long_running_options_reviewed(conn, &mut findings)?;
    validate_pending_overrides(conn, policy, &mut findings)?;

    Ok(findings)
}

pub fn has_errors(findings: &[Finding]) -> bool {
    findings
        .iter()
        .any(|finding| finding.severity == FindingSeverity::Error)
}

pub fn format_findings(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "ok: no validation findings\n".to_owned();
    }

    findings
        .iter()
        .map(|finding| {
            let id = finding
                .entity_id
                .map(|id| format!(" {id}"))
                .unwrap_or_default();
            format!(
                "{} [{}] {}{} {}: {}\n",
                finding.severity,
                finding.finding_type,
                finding.entity_type,
                id,
                finding.field,
                finding.message
            )
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub database_path: PathBuf,
    pub database_readable: bool,
    pub schema_version: Option<i64>,
    pub pending_migrations: bool,
    pub policy_path: PathBuf,
    pub policy_loaded: bool,
    pub current_program_resolved: bool,
    pub current_branch_resolved: bool,
    pub artifact_roots: Vec<ArtifactRootStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRootStatus {
    pub root: PathBuf,
    pub exists: bool,
    pub readable: bool,
}

pub fn doctor_report(
    conn: Option<&Connection>,
    policy: Option<&Policy>,
    db_path: impl Into<PathBuf>,
    policy_path: impl Into<PathBuf>,
    paths: &ValidationPaths,
) -> anyhow::Result<DoctorReport> {
    let schema_version = match conn {
        Some(conn) => migrations::current_schema_version(conn)?,
        None => None,
    };
    let pending_migrations = schema_version.unwrap_or(0) < migrations::CURRENT_SCHEMA_VERSION;

    let mut current_program_resolved = false;
    let mut current_branch_resolved = false;
    if let (Some(conn), Some(policy)) = (conn, policy) {
        current_program_resolved = resolve_current_program(conn, policy)?.is_some();
        current_branch_resolved = resolve_current_branch(conn, policy)?.is_some();
    }

    let artifact_roots = policy
        .map(|policy| {
            policy
                .allowed_artifact_roots
                .iter()
                .map(|root| {
                    let path = paths.project_root.join(root);
                    ArtifactRootStatus {
                        root: path.clone(),
                        exists: path.exists(),
                        readable: path.read_dir().is_ok(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(DoctorReport {
        database_path: db_path.into(),
        database_readable: conn.is_some(),
        schema_version,
        pending_migrations,
        policy_path: policy_path.into(),
        policy_loaded: policy.is_some(),
        current_program_resolved,
        current_branch_resolved,
        artifact_roots,
    })
}

pub fn format_doctor_report(report: &DoctorReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "database: {} ({})\n",
        report.database_path.display(),
        if report.database_readable {
            "readable"
        } else {
            "unreadable"
        }
    ));
    out.push_str(&format!(
        "schema_version: {}\n",
        report
            .schema_version
            .map(|version| version.to_string())
            .unwrap_or_else(|| "none".to_owned())
    ));
    out.push_str(&format!(
        "pending_migrations: {}\n",
        report.pending_migrations
    ));
    out.push_str(&format!(
        "policy: {} ({})\n",
        report.policy_path.display(),
        if report.policy_loaded {
            "loaded"
        } else {
            "missing_or_invalid"
        }
    ));
    out.push_str(&format!(
        "current_program_resolved: {}\n",
        report.current_program_resolved
    ));
    out.push_str(&format!(
        "current_branch_resolved: {}\n",
        report.current_branch_resolved
    ));
    for root in &report.artifact_roots {
        out.push_str(&format!(
            "artifact_root: {} exists={} readable={}\n",
            root.root.display(),
            root.exists,
            root.readable
        ));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrateReport {
    pub applied_versions: Vec<i64>,
    pub current_version: Option<i64>,
}

pub fn migrate_report(conn: &mut Connection) -> anyhow::Result<MigrateReport> {
    let applied_versions = migrations::apply_migrations(conn)?
        .into_iter()
        .map(|migration| migration.version)
        .collect();
    let current_version = migrations::current_schema_version(conn)?;
    Ok(MigrateReport {
        applied_versions,
        current_version,
    })
}

pub fn format_migrate_report(report: &MigrateReport) -> String {
    if report.applied_versions.is_empty() {
        format!(
            "migrations: already current at {}\n",
            report
                .current_version
                .map(|version| version.to_string())
                .unwrap_or_else(|| "none".to_owned())
        )
    } else {
        format!(
            "migrations: applied {:?}; current {}\n",
            report.applied_versions,
            report
                .current_version
                .map(|version| version.to_string())
                .unwrap_or_else(|| "none".to_owned())
        )
    }
}

fn validate_current_refs(
    conn: &Connection,
    policy: &Policy,
    findings: &mut Vec<Finding>,
) -> anyhow::Result<()> {
    match policy
        .current_program
        .as_deref()
        .filter(|slug| !slug.is_empty())
    {
        Some(slug) => {
            if resolve_current_program(conn, policy)?.is_none() {
                findings.push(Finding::new(
                    FindingSeverity::Error,
                    FindingType::MissingCurrentProgram,
                    "policy",
                    None,
                    "current_program",
                    format!("current program {slug:?} does not resolve to a program"),
                ));
            }
        }
        None => findings.push(Finding::new(
            FindingSeverity::Error,
            FindingType::MissingCurrentProgram,
            "policy",
            None,
            "current_program",
            "policy does not set current_program",
        )),
    }

    match policy
        .current_branch
        .as_deref()
        .filter(|slug| !slug.is_empty())
    {
        Some(slug) => {
            if resolve_current_branch(conn, policy)?.is_none() {
                findings.push(Finding::new(
                    FindingSeverity::Error,
                    FindingType::MissingCurrentBranch,
                    "policy",
                    None,
                    "current_branch",
                    format!("current branch {slug:?} does not resolve within current_program"),
                ));
            }
        }
        None => findings.push(Finding::new(
            FindingSeverity::Error,
            FindingType::MissingCurrentBranch,
            "policy",
            None,
            "current_branch",
            "policy does not set current_branch",
        )),
    }

    Ok(())
}

fn validate_completed_experiments_have_decisions(
    conn: &Connection,
    policy: &Policy,
    findings: &mut Vec<Finding>,
) -> anyhow::Result<()> {
    let severity = if policy.required_decision_after_experiment {
        FindingSeverity::Error
    } else {
        FindingSeverity::Warning
    };
    let mut stmt = conn
        .prepare(
            "SELECT experiment.id, experiment.slug
             FROM experiment
             LEFT JOIN decision ON decision.experiment_id = experiment.id
             WHERE experiment.status = 'completed'
             GROUP BY experiment.id
             HAVING count(decision.id) = 0
             ORDER BY experiment.id",
        )
        .context("failed to prepare completed experiment decision validation")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read completed experiment decision validation")?;

    findings.extend(rows.into_iter().map(|(id, slug)| {
        Finding::new(
            severity,
            FindingType::CompletedExperimentMissingDecision,
            "experiment",
            Some(id),
            "decision",
            format!("completed experiment {slug:?} has no decision"),
        )
    }));
    Ok(())
}

fn validate_runs_with_metrics_have_artifacts(
    conn: &Connection,
    findings: &mut Vec<Finding>,
) -> anyhow::Result<()> {
    let mut stmt = conn
        .prepare(
            "SELECT run.id
             FROM run
             WHERE EXISTS (SELECT 1 FROM metric WHERE metric.run_id = run.id)
               AND NOT EXISTS (SELECT 1 FROM artifact WHERE artifact.run_id = run.id)
             ORDER BY run.id",
        )
        .context("failed to prepare run artifact validation")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read run artifact validation")?;

    findings.extend(rows.into_iter().map(|id| {
        Finding::new(
            FindingSeverity::Warning,
            FindingType::RunMetricsWithoutArtifacts,
            "run",
            Some(id),
            "artifact",
            "run records metrics but no artifacts",
        )
    }));
    Ok(())
}

fn validate_artifact_roots(
    conn: &Connection,
    policy: &Policy,
    paths: &ValidationPaths,
    findings: &mut Vec<Finding>,
) -> anyhow::Result<()> {
    if policy.allowed_artifact_roots.is_empty() {
        return Ok(());
    }

    let mut stmt = conn
        .prepare("SELECT id, path FROM artifact ORDER BY id")
        .context("failed to prepare artifact root validation")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read artifact root validation")?;

    for (id, artifact_path) in rows {
        if !artifact_path_allowed(&artifact_path, &policy.allowed_artifact_roots, paths) {
            findings.push(Finding::new(
                FindingSeverity::Error,
                FindingType::ArtifactOutsideAllowedRoots,
                "artifact",
                Some(id),
                "path",
                format!("artifact path {artifact_path:?} is outside allowed_artifact_roots"),
            ));
        }
    }

    Ok(())
}

fn validate_active_branches_have_work(
    conn: &Connection,
    findings: &mut Vec<Finding>,
) -> anyhow::Result<()> {
    let mut stmt = conn
        .prepare(
            "SELECT branch.id, branch.slug
             FROM branch
             WHERE branch.status = 'active'
               AND NOT EXISTS (
                   SELECT 1
                   FROM experiment
                   WHERE experiment.branch_id = branch.id
                     AND experiment.status IN ('planned', 'running')
               )
               AND EXISTS (
                   SELECT 1
                   FROM research_option
                   WHERE research_option.branch_id = branch.id
                     AND research_option.status IN ('open', 'selected', 'in_progress')
               )
             ORDER BY branch.id",
        )
        .context("failed to prepare active branch validation")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read active branch validation")?;

    findings.extend(rows.into_iter().map(|(id, slug)| {
        Finding::new(
            FindingSeverity::Warning,
            FindingType::ActiveBranchWithoutExperiment,
            "branch",
            Some(id),
            "experiment",
            format!("active branch {slug:?} has open work but no planned or running experiment"),
        )
    }));
    Ok(())
}

fn validate_long_running_options_reviewed(
    conn: &Connection,
    findings: &mut Vec<Finding>,
) -> anyhow::Result<()> {
    let mut stmt = conn
        .prepare(
            "SELECT id, slug
             FROM research_option
             WHERE classification = 'long_running'
               AND status IN ('selected', 'in_progress')
               AND review_state != 'reviewed'
             ORDER BY id",
        )
        .context("failed to prepare long-running option validation")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read long-running option validation")?;

    findings.extend(rows.into_iter().map(|(id, slug)| {
        Finding::new(
            FindingSeverity::Warning,
            FindingType::LongRunningSelectionNeedsReview,
            "research_option",
            Some(id),
            "review_state",
            format!("long-running selected option {slug:?} has not been reviewed"),
        )
    }));
    Ok(())
}

fn validate_pending_overrides(
    conn: &Connection,
    policy: &Policy,
    findings: &mut Vec<Finding>,
) -> anyhow::Result<()> {
    if !policy.require_human_approval_for_blocked_overrides {
        return Ok(());
    }

    if table_exists(conn, "override_request")? {
        validate_pending_override_request_table(conn, findings)?;
    }

    if table_exists(conn, "override_approval")? {
        validate_pending_override_approval_table(conn, findings)?;
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, slug, review_state
             FROM research_option
             WHERE classification = 'blocked'
               AND status IN ('selected', 'in_progress')
               AND review_state NOT IN ('approved', 'reviewed')
             ORDER BY id",
        )
        .context("failed to prepare blocked option override validation")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read blocked option override validation")?;

    findings.extend(rows.into_iter().map(|(id, slug, review_state)| {
        Finding::new(
            FindingSeverity::Error,
            FindingType::PendingOverrideApproval,
            "research_option",
            Some(id),
            "review_state",
            format!(
                "blocked selected option {slug:?} requires approved override; current review_state is {review_state:?}"
            ),
        )
    }));
    Ok(())
}

fn validate_pending_override_request_table(
    conn: &Connection,
    findings: &mut Vec<Finding>,
) -> anyhow::Result<()> {
    let columns = table_columns(conn, "override_request")?;
    if !columns
        .iter()
        .any(|column| column == "status" || column == "state")
    {
        return Ok(());
    }

    let state_column = if columns.iter().any(|column| column == "status") {
        "status"
    } else {
        "state"
    };
    let sql = format!(
        "SELECT id, {state_column}
         FROM override_request
         WHERE {state_column} IN ('pending', 'requested', 'approval_required')
         ORDER BY id"
    );
    let mut stmt = conn
        .prepare(&sql)
        .context("failed to prepare override request validation")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read override request validation")?;

    findings.extend(rows.into_iter().map(|(id, state)| {
        Finding::new(
            FindingSeverity::Error,
            FindingType::PendingOverrideApproval,
            "override_request",
            Some(id),
            state_column,
            format!("override request is pending approval with state {state:?}"),
        )
    }));
    Ok(())
}

fn validate_pending_override_approval_table(
    conn: &Connection,
    findings: &mut Vec<Finding>,
) -> anyhow::Result<()> {
    let columns = table_columns(conn, "override_approval")?;
    if !(columns.iter().any(|column| column == "approved_by")
        || columns.iter().any(|column| column == "approved_at"))
    {
        return Ok(());
    }

    let mut stmt = conn
        .prepare(
            "SELECT id
             FROM override_approval
             WHERE nullif(trim(approved_by), '') IS NULL
                OR nullif(trim(approved_at), '') IS NULL
             ORDER BY id",
        )
        .context("failed to prepare override approval validation")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read override approval validation")?;

    findings.extend(rows.into_iter().map(|id| {
        Finding::new(
            FindingSeverity::Error,
            FindingType::PendingOverrideApproval,
            "override_approval",
            Some(id),
            "approved_by",
            "override approval row is missing approval data",
        )
    }));
    Ok(())
}

fn resolve_current_program(conn: &Connection, policy: &Policy) -> anyhow::Result<Option<i64>> {
    let Some(slug) = policy
        .current_program
        .as_deref()
        .filter(|slug| !slug.is_empty())
    else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT id FROM program WHERE slug = ?1",
        params![slug],
        |row| row.get(0),
    )
    .optional()
    .with_context(|| format!("failed to resolve current program {slug}"))
}

fn resolve_current_branch(conn: &Connection, policy: &Policy) -> anyhow::Result<Option<i64>> {
    let Some(program_id) = resolve_current_program(conn, policy)? else {
        return Ok(None);
    };
    let Some(slug) = policy
        .current_branch
        .as_deref()
        .filter(|slug| !slug.is_empty())
    else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT id FROM branch WHERE program_id = ?1 AND slug = ?2",
        params![program_id, slug],
        |row| row.get(0),
    )
    .optional()
    .with_context(|| format!("failed to resolve current branch {slug}"))
}

fn artifact_path_allowed(
    artifact_path: &str,
    allowed_roots: &[String],
    paths: &ValidationPaths,
) -> bool {
    let artifact_path = Path::new(artifact_path);
    let artifact_abs = normalize_project_path(&paths.project_root, artifact_path);

    allowed_roots.iter().any(|root| {
        let root_path = Path::new(root);
        let root_abs = normalize_project_path(&paths.project_root, root_path);
        artifact_abs.starts_with(root_abs)
    })
}

fn normalize_project_path(project_root: &Path, path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };

    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn table_exists(conn: &Connection, table: &str) -> anyhow::Result<bool> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table],
        |_| Ok(()),
    )
    .optional()
    .with_context(|| format!("failed to inspect table {table}"))
    .map(|value| value.is_some())
}

fn table_columns(conn: &Connection, table: &str) -> anyhow::Result<Vec<String>> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .with_context(|| format!("failed to inspect columns for table {table}"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .with_context(|| format!("failed to read columns for table {table}"))?;
    Ok(columns)
}
