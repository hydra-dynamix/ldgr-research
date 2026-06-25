#![allow(dead_code)]

use anyhow::Context;
use rusqlite::{params, Connection, OptionalExtension};

use crate::schema::INITIAL_SCHEMA_VERSION;

pub const CURRENT_SCHEMA_VERSION: i64 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedMigration {
    pub version: i64,
    pub description: &'static str,
}

struct Migration {
    version: i64,
    description: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: INITIAL_SCHEMA_VERSION,
        description: "create core research graph schema",
        sql: MIGRATION_001,
    },
    Migration {
        version: 2,
        description: "add stable UUIDs for export and central ingestion",
        sql: MIGRATION_002,
    },
    Migration {
        version: 3,
        description: "add agent bug reports",
        sql: MIGRATION_003,
    },
    Migration {
        version: 4,
        description: "add research evaluation matrices",
        sql: MIGRATION_004,
    },
];

pub fn apply_migrations(conn: &mut Connection) -> anyhow::Result<Vec<AppliedMigration>> {
    ensure_migrations_table(conn)?;

    let mut applied = Vec::new();
    for migration in MIGRATIONS {
        let already_applied: Option<i64> = conn
            .query_row(
                "SELECT version FROM schema_migrations WHERE version = ?1",
                params![migration.version],
                |row| row.get(0),
            )
            .optional()
            .with_context(|| format!("failed to check migration {}", migration.version))?;

        if already_applied.is_some() {
            continue;
        }

        let tx = conn
            .transaction()
            .with_context(|| format!("failed to start migration {}", migration.version))?;
        tx.execute_batch(migration.sql)
            .with_context(|| format!("failed to apply migration {}", migration.version))?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at, description)
             VALUES (?1, datetime('now'), ?2)",
            params![migration.version, migration.description],
        )
        .with_context(|| format!("failed to record migration {}", migration.version))?;
        tx.commit()
            .with_context(|| format!("failed to commit migration {}", migration.version))?;

        applied.push(AppliedMigration {
            version: migration.version,
            description: migration.description,
        });
    }

    Ok(applied)
}

pub fn current_schema_version(conn: &Connection) -> anyhow::Result<Option<i64>> {
    ensure_migrations_table(conn)?;
    conn.query_row("SELECT max(version) FROM schema_migrations", [], |row| {
        row.get(0)
    })
    .context("failed to read current schema version")
}

fn ensure_migrations_table(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL,
            description TEXT NOT NULL
        );",
    )
    .context("failed to ensure schema_migrations table")
}

const MIGRATION_001: &str = r#"
CREATE TABLE IF NOT EXISTS program (
    id INTEGER PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    objective TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'paused', 'complete', 'abandoned')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS branch (
    id INTEGER PRIMARY KEY,
    program_id INTEGER NOT NULL REFERENCES program(id) ON DELETE CASCADE,
    parent_branch_id INTEGER REFERENCES branch(id) ON DELETE SET NULL,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    question TEXT NOT NULL,
    rationale TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'promising', 'failed', 'blocked', 'complete', 'abandoned')),
    decision_summary TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (program_id, slug)
);

CREATE TABLE IF NOT EXISTS research_option (
    id INTEGER PRIMARY KEY,
    program_id INTEGER NOT NULL REFERENCES program(id) ON DELETE CASCADE,
    branch_id INTEGER REFERENCES branch(id) ON DELETE CASCADE,
    open_question_id INTEGER REFERENCES open_question(id) ON DELETE SET NULL,
    source_experiment_id INTEGER REFERENCES experiment(id) ON DELETE SET NULL,
    source_decision_id INTEGER REFERENCES decision(id) ON DELETE SET NULL,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    hypothesis TEXT,
    description TEXT NOT NULL,
    classification TEXT NOT NULL
        CHECK (classification IN ('main_path', 'validation', 'exploratory', 'long_running', 'blocked', 'maintenance')),
    status TEXT NOT NULL DEFAULT 'open'
        CHECK (status IN ('open', 'selected', 'in_progress', 'answered', 'rejected', 'superseded')),
    selection_rationale TEXT,
    selected_by TEXT,
    selected_at TEXT,
    review_state TEXT NOT NULL DEFAULT 'none'
        CHECK (review_state IN ('none', 'needs_review', 'reviewed', 'approval_required', 'approved', 'rejected')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (program_id, slug)
);

CREATE TABLE IF NOT EXISTS experiment (
    id INTEGER PRIMARY KEY,
    branch_id INTEGER NOT NULL REFERENCES branch(id) ON DELETE CASCADE,
    option_id INTEGER REFERENCES research_option(id) ON DELETE SET NULL,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    phase TEXT,
    mode TEXT NOT NULL CHECK (mode IN ('falsification', 'exploration')),
    hypothesis TEXT,
    setup TEXT,
    observation_goal TEXT,
    rationale TEXT,
    primary_metrics_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(primary_metrics_json)),
    secondary_metrics_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(secondary_metrics_json)),
    pass_criteria TEXT,
    fail_criteria TEXT,
    allowed_next_steps TEXT,
    blocked_next_steps TEXT,
    status TEXT NOT NULL DEFAULT 'planned'
        CHECK (status IN ('planned', 'running', 'completed', 'inconclusive', 'failed', 'superseded')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (branch_id, slug),
    CHECK (
        (mode = 'falsification' AND hypothesis IS NOT NULL AND fail_criteria IS NOT NULL)
        OR
        (mode = 'exploration' AND observation_goal IS NOT NULL)
    )
);

CREATE TABLE IF NOT EXISTS run (
    id INTEGER PRIMARY KEY,
    experiment_id INTEGER NOT NULL REFERENCES experiment(id) ON DELETE CASCADE,
    command TEXT,
    environment_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(environment_json)),
    dataset TEXT,
    code_ref TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    status TEXT NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'success', 'failed', 'partial')),
    notes TEXT
);

CREATE TABLE IF NOT EXISTS metric (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES run(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    value REAL NOT NULL,
    unit TEXT,
    higher_is_better INTEGER CHECK (higher_is_better IN (0, 1)),
    split TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json))
);

CREATE TABLE IF NOT EXISTS artifact (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES run(id) ON DELETE CASCADE,
    kind TEXT NOT NULL
        CHECK (kind IN ('json', 'csv', 'audio', 'image', 'report', 'model', 'npz', 'midi', 'other')),
    path TEXT NOT NULL,
    description TEXT NOT NULL,
    checksum TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json))
);

CREATE TABLE IF NOT EXISTS decision (
    id INTEGER PRIMARY KEY,
    experiment_id INTEGER NOT NULL REFERENCES experiment(id) ON DELETE CASCADE,
    result_summary TEXT NOT NULL,
    interpretation TEXT NOT NULL,
    limitations TEXT NOT NULL,
    decision TEXT NOT NULL
        CHECK (decision IN ('continue', 'branch', 'revise', 'stop', 'inconclusive')),
    confidence TEXT NOT NULL CHECK (confidence IN ('low', 'medium', 'high')),
    next_branch_id INTEGER REFERENCES branch(id) ON DELETE SET NULL,
    next_experiment_id INTEGER REFERENCES experiment(id) ON DELETE SET NULL,
    proposed_options_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(proposed_options_json)),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS open_question (
    id INTEGER PRIMARY KEY,
    program_id INTEGER NOT NULL REFERENCES program(id) ON DELETE CASCADE,
    branch_id INTEGER REFERENCES branch(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    question TEXT NOT NULL,
    context TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open'
        CHECK (status IN ('open', 'answered', 'rejected', 'superseded')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (program_id, slug)
);

CREATE TABLE IF NOT EXISTS fact (
    id INTEGER PRIMARY KEY,
    program_id INTEGER NOT NULL REFERENCES program(id) ON DELETE CASCADE,
    branch_id INTEGER REFERENCES branch(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    statement TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'candidate'
        CHECK (status IN ('candidate', 'accepted', 'contested', 'rejected', 'superseded')),
    confidence TEXT CHECK (confidence IN ('low', 'medium', 'high')),
    created_from_experiment_id INTEGER REFERENCES experiment(id) ON DELETE SET NULL,
    created_from_decision_id INTEGER REFERENCES decision(id) ON DELETE SET NULL,
    review_state TEXT NOT NULL DEFAULT 'none'
        CHECK (review_state IN ('none', 'needs_review', 'reviewed')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (program_id, slug)
);

CREATE TABLE IF NOT EXISTS axiom (
    id INTEGER PRIMARY KEY,
    program_id INTEGER NOT NULL REFERENCES program(id) ON DELETE CASCADE,
    branch_id INTEGER REFERENCES branch(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    statement TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'validated', 'contested', 'retired')),
    created_by_actor TEXT,
    created_by_agent INTEGER NOT NULL DEFAULT 0 CHECK (created_by_agent IN (0, 1)),
    review_state TEXT NOT NULL DEFAULT 'none'
        CHECK (review_state IN ('none', 'needs_review', 'reviewed')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (program_id, slug)
);

CREATE TABLE IF NOT EXISTS evidence_link (
    id INTEGER PRIMARY KEY,
    subject_type TEXT NOT NULL CHECK (subject_type IN ('fact', 'axiom')),
    subject_id INTEGER NOT NULL,
    relation TEXT NOT NULL
        CHECK (relation IN ('supports', 'contradicts', 'refines', 'supersedes')),
    experiment_id INTEGER REFERENCES experiment(id) ON DELETE SET NULL,
    run_id INTEGER REFERENCES run(id) ON DELETE SET NULL,
    metric_id INTEGER REFERENCES metric(id) ON DELETE SET NULL,
    artifact_id INTEGER REFERENCES artifact(id) ON DELETE SET NULL,
    decision_id INTEGER REFERENCES decision(id) ON DELETE SET NULL,
    report_path TEXT,
    report_anchor TEXT,
    summary TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (
        experiment_id IS NOT NULL OR run_id IS NOT NULL OR metric_id IS NOT NULL
        OR artifact_id IS NOT NULL OR decision_id IS NOT NULL OR report_path IS NOT NULL
    )
);

CREATE TABLE IF NOT EXISTS review_item (
    id INTEGER PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id INTEGER NOT NULL,
    reason TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'needs_review'
        CHECK (state IN ('needs_review', 'reviewed', 'dismissed')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    reviewed_at TEXT,
    reviewed_by TEXT,
    notes TEXT
);

CREATE TABLE IF NOT EXISTS override_approval (
    id INTEGER PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id INTEGER NOT NULL,
    blocked_work TEXT NOT NULL,
    requested_action TEXT NOT NULL,
    justification TEXT NOT NULL,
    approved_by TEXT NOT NULL,
    approved_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS event_log (
    id INTEGER PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id INTEGER NOT NULL,
    action TEXT NOT NULL,
    payload_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(payload_json)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    actor TEXT
);

CREATE INDEX IF NOT EXISTS idx_program_slug ON program(slug);
CREATE INDEX IF NOT EXISTS idx_branch_slug ON branch(slug);
CREATE INDEX IF NOT EXISTS idx_branch_program ON branch(program_id);
CREATE INDEX IF NOT EXISTS idx_branch_parent ON branch(parent_branch_id);
CREATE INDEX IF NOT EXISTS idx_experiment_slug ON experiment(slug);
CREATE INDEX IF NOT EXISTS idx_experiment_branch ON experiment(branch_id);
CREATE INDEX IF NOT EXISTS idx_run_experiment ON run(experiment_id);
CREATE INDEX IF NOT EXISTS idx_metric_run ON metric(run_id);
CREATE INDEX IF NOT EXISTS idx_artifact_run ON artifact(run_id);
CREATE INDEX IF NOT EXISTS idx_decision_experiment ON decision(experiment_id);
CREATE INDEX IF NOT EXISTS idx_open_question_program ON open_question(program_id);
CREATE INDEX IF NOT EXISTS idx_open_question_branch ON open_question(branch_id);
CREATE INDEX IF NOT EXISTS idx_research_option_program ON research_option(program_id);
CREATE INDEX IF NOT EXISTS idx_research_option_branch ON research_option(branch_id);
CREATE INDEX IF NOT EXISTS idx_research_option_status ON research_option(status);
CREATE INDEX IF NOT EXISTS idx_research_option_classification ON research_option(classification);
CREATE INDEX IF NOT EXISTS idx_fact_program ON fact(program_id);
CREATE INDEX IF NOT EXISTS idx_fact_status ON fact(status);
CREATE INDEX IF NOT EXISTS idx_fact_review_state ON fact(review_state);
CREATE INDEX IF NOT EXISTS idx_axiom_program ON axiom(program_id);
CREATE INDEX IF NOT EXISTS idx_axiom_status ON axiom(status);
CREATE INDEX IF NOT EXISTS idx_axiom_review_state ON axiom(review_state);
CREATE INDEX IF NOT EXISTS idx_evidence_subject ON evidence_link(subject_type, subject_id);
"#;

const MIGRATION_002: &str = r#"
ALTER TABLE program ADD COLUMN uuid TEXT;
UPDATE program SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_program_uuid ON program(uuid);
CREATE TRIGGER IF NOT EXISTS trg_program_uuid_after_insert
AFTER INSERT ON program
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE program
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE branch ADD COLUMN uuid TEXT;
UPDATE branch SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_branch_uuid ON branch(uuid);
CREATE TRIGGER IF NOT EXISTS trg_branch_uuid_after_insert
AFTER INSERT ON branch
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE branch
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE research_option ADD COLUMN uuid TEXT;
UPDATE research_option SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_research_option_uuid ON research_option(uuid);
CREATE TRIGGER IF NOT EXISTS trg_research_option_uuid_after_insert
AFTER INSERT ON research_option
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE research_option
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE experiment ADD COLUMN uuid TEXT;
UPDATE experiment SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_experiment_uuid ON experiment(uuid);
CREATE TRIGGER IF NOT EXISTS trg_experiment_uuid_after_insert
AFTER INSERT ON experiment
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE experiment
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE run ADD COLUMN uuid TEXT;
UPDATE run SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_run_uuid ON run(uuid);
CREATE TRIGGER IF NOT EXISTS trg_run_uuid_after_insert
AFTER INSERT ON run
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE run
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE metric ADD COLUMN uuid TEXT;
UPDATE metric SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_metric_uuid ON metric(uuid);
CREATE TRIGGER IF NOT EXISTS trg_metric_uuid_after_insert
AFTER INSERT ON metric
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE metric
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE artifact ADD COLUMN uuid TEXT;
UPDATE artifact SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_artifact_uuid ON artifact(uuid);
CREATE TRIGGER IF NOT EXISTS trg_artifact_uuid_after_insert
AFTER INSERT ON artifact
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE artifact
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE decision ADD COLUMN uuid TEXT;
UPDATE decision SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_decision_uuid ON decision(uuid);
CREATE TRIGGER IF NOT EXISTS trg_decision_uuid_after_insert
AFTER INSERT ON decision
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE decision
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE open_question ADD COLUMN uuid TEXT;
UPDATE open_question SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_open_question_uuid ON open_question(uuid);
CREATE TRIGGER IF NOT EXISTS trg_open_question_uuid_after_insert
AFTER INSERT ON open_question
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE open_question
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE fact ADD COLUMN uuid TEXT;
UPDATE fact SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_fact_uuid ON fact(uuid);
CREATE TRIGGER IF NOT EXISTS trg_fact_uuid_after_insert
AFTER INSERT ON fact
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE fact
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE axiom ADD COLUMN uuid TEXT;
UPDATE axiom SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_axiom_uuid ON axiom(uuid);
CREATE TRIGGER IF NOT EXISTS trg_axiom_uuid_after_insert
AFTER INSERT ON axiom
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE axiom
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE evidence_link ADD COLUMN uuid TEXT;
UPDATE evidence_link SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_evidence_link_uuid ON evidence_link(uuid);
CREATE TRIGGER IF NOT EXISTS trg_evidence_link_uuid_after_insert
AFTER INSERT ON evidence_link
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE evidence_link
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE review_item ADD COLUMN uuid TEXT;
UPDATE review_item SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_review_item_uuid ON review_item(uuid);
CREATE TRIGGER IF NOT EXISTS trg_review_item_uuid_after_insert
AFTER INSERT ON review_item
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE review_item
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE override_approval ADD COLUMN uuid TEXT;
UPDATE override_approval SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_override_approval_uuid ON override_approval(uuid);
CREATE TRIGGER IF NOT EXISTS trg_override_approval_uuid_after_insert
AFTER INSERT ON override_approval
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE override_approval
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

ALTER TABLE event_log ADD COLUMN uuid TEXT;
UPDATE event_log SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_event_log_uuid ON event_log(uuid);
CREATE TRIGGER IF NOT EXISTS trg_event_log_uuid_after_insert
AFTER INSERT ON event_log
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE event_log
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;
"#;

const MIGRATION_003: &str = r#"
CREATE TABLE IF NOT EXISTS bug_report (
    id INTEGER PRIMARY KEY,
    uuid TEXT,
    program_id INTEGER REFERENCES program(id) ON DELETE SET NULL,
    branch_id INTEGER REFERENCES branch(id) ON DELETE SET NULL,
    experiment_id INTEGER REFERENCES experiment(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'medium'
        CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    status TEXT NOT NULL DEFAULT 'open'
        CHECK (status IN ('open', 'triaged', 'resolved', 'dismissed')),
    command TEXT,
    error TEXT,
    reproduction TEXT,
    log_path TEXT,
    log_excerpt TEXT,
    reported_by TEXT,
    resolution_notes TEXT,
    resolved_by TEXT,
    resolved_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

UPDATE bug_report SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_bug_report_uuid ON bug_report(uuid);
CREATE INDEX IF NOT EXISTS idx_bug_report_status ON bug_report(status);
CREATE INDEX IF NOT EXISTS idx_bug_report_program ON bug_report(program_id);
CREATE INDEX IF NOT EXISTS idx_bug_report_branch ON bug_report(branch_id);
CREATE INDEX IF NOT EXISTS idx_bug_report_experiment ON bug_report(experiment_id);
CREATE TRIGGER IF NOT EXISTS trg_bug_report_uuid_after_insert
AFTER INSERT ON bug_report
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE bug_report
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;
"#;

const MIGRATION_004: &str = r#"
CREATE TABLE IF NOT EXISTS research_matrix (
    id INTEGER PRIMARY KEY,
    uuid TEXT,
    program_id INTEGER NOT NULL REFERENCES program(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'complete', 'archived')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (program_id, slug)
);

CREATE TABLE IF NOT EXISTS matrix_axis (
    id INTEGER PRIMARY KEY,
    uuid TEXT,
    matrix_id INTEGER NOT NULL REFERENCES research_matrix(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (matrix_id, slug),
    UNIQUE (matrix_id, position)
);

CREATE TABLE IF NOT EXISTS matrix_level (
    id INTEGER PRIMARY KEY,
    uuid TEXT,
    axis_id INTEGER NOT NULL REFERENCES matrix_axis(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (axis_id, slug),
    UNIQUE (axis_id, position)
);

CREATE TABLE IF NOT EXISTS matrix_cell (
    id INTEGER PRIMARY KEY,
    uuid TEXT,
    matrix_id INTEGER NOT NULL REFERENCES research_matrix(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    coordinates_json TEXT NOT NULL CHECK (json_valid(coordinates_json)),
    experiment_id INTEGER REFERENCES experiment(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'planned'
        CHECK (status IN ('planned', 'running', 'completed', 'blocked', 'skipped')),
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (matrix_id, slug),
    UNIQUE (matrix_id, coordinates_json)
);

CREATE TABLE IF NOT EXISTS matrix_cell_level (
    cell_id INTEGER NOT NULL REFERENCES matrix_cell(id) ON DELETE CASCADE,
    axis_id INTEGER NOT NULL REFERENCES matrix_axis(id) ON DELETE CASCADE,
    level_id INTEGER NOT NULL REFERENCES matrix_level(id) ON DELETE CASCADE,
    PRIMARY KEY (cell_id, axis_id),
    UNIQUE (cell_id, level_id)
);

UPDATE research_matrix SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
UPDATE matrix_axis SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
UPDATE matrix_level SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;
UPDATE matrix_cell SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))) WHERE uuid IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_research_matrix_uuid ON research_matrix(uuid);
CREATE INDEX IF NOT EXISTS idx_research_matrix_program ON research_matrix(program_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_matrix_axis_uuid ON matrix_axis(uuid);
CREATE INDEX IF NOT EXISTS idx_matrix_axis_matrix ON matrix_axis(matrix_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_matrix_level_uuid ON matrix_level(uuid);
CREATE INDEX IF NOT EXISTS idx_matrix_level_axis ON matrix_level(axis_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_matrix_cell_uuid ON matrix_cell(uuid);
CREATE INDEX IF NOT EXISTS idx_matrix_cell_matrix ON matrix_cell(matrix_id);
CREATE INDEX IF NOT EXISTS idx_matrix_cell_experiment ON matrix_cell(experiment_id);
CREATE INDEX IF NOT EXISTS idx_matrix_cell_status ON matrix_cell(status);

CREATE TRIGGER IF NOT EXISTS trg_research_matrix_uuid_after_insert
AFTER INSERT ON research_matrix
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE research_matrix
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_matrix_axis_uuid_after_insert
AFTER INSERT ON matrix_axis
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE matrix_axis
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_matrix_level_uuid_after_insert
AFTER INSERT ON matrix_level
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE matrix_level
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_matrix_cell_uuid_after_insert
AFTER INSERT ON matrix_cell
WHEN NEW.uuid IS NULL
BEGIN
    UPDATE matrix_cell
    SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))
    WHERE id = NEW.id;
END;
"#;
