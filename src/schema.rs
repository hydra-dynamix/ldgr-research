#![allow(dead_code)]

use std::fmt;
use std::str::FromStr;

use rusqlite::Row;
use thiserror::Error;

pub const INITIAL_SCHEMA_VERSION: i64 = 1;

pub const TABLES: &[&str] = &[
    "schema_migrations",
    "program",
    "branch",
    "experiment",
    "run",
    "metric",
    "artifact",
    "decision",
    "open_question",
    "research_option",
    "fact",
    "axiom",
    "evidence_link",
    "review_item",
    "override_approval",
    "bug_report",
    "research_matrix",
    "matrix_axis",
    "matrix_level",
    "matrix_cell",
    "matrix_cell_level",
    "event_log",
];

#[derive(Debug, Error, PartialEq, Eq)]
#[error("invalid {enum_name} value: {value}")]
pub struct ParseEnumError {
    enum_name: &'static str,
    value: String,
}

impl ParseEnumError {
    fn new(enum_name: &'static str, value: &str) -> Self {
        Self {
            enum_name,
            value: value.to_owned(),
        }
    }
}

macro_rules! string_enum {
    ($name:ident, $enum_name:literal, { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const VALUES: &'static [&'static str] = &[$($value),+];

            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = ParseEnumError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(ParseEnumError::new($enum_name, value)),
                }
            }
        }
    };
}

string_enum!(ProgramStatus, "program status", {
    Active => "active",
    Paused => "paused",
    Complete => "complete",
    Abandoned => "abandoned",
});

string_enum!(BranchStatus, "branch status", {
    Active => "active",
    Promising => "promising",
    Failed => "failed",
    Blocked => "blocked",
    Complete => "complete",
    Abandoned => "abandoned",
});

string_enum!(ExperimentStatus, "experiment status", {
    Planned => "planned",
    Running => "running",
    Completed => "completed",
    Inconclusive => "inconclusive",
    Failed => "failed",
    Superseded => "superseded",
});

string_enum!(ExperimentMode, "experiment mode", {
    Falsification => "falsification",
    Exploration => "exploration",
});

string_enum!(RunStatus, "run status", {
    Running => "running",
    Success => "success",
    Failed => "failed",
    Partial => "partial",
});

string_enum!(ArtifactKind, "artifact kind", {
    Json => "json",
    Csv => "csv",
    Audio => "audio",
    Image => "image",
    Report => "report",
    Model => "model",
    Npz => "npz",
    Midi => "midi",
    Other => "other",
});

string_enum!(DecisionKind, "decision", {
    Continue => "continue",
    Branch => "branch",
    Revise => "revise",
    Stop => "stop",
    Inconclusive => "inconclusive",
});

string_enum!(Confidence, "confidence", {
    Low => "low",
    Medium => "medium",
    High => "high",
});

string_enum!(OpenQuestionStatus, "open question status", {
    Open => "open",
    Answered => "answered",
    Rejected => "rejected",
    Superseded => "superseded",
});

string_enum!(ResearchOptionStatus, "research option status", {
    Open => "open",
    Selected => "selected",
    InProgress => "in_progress",
    Answered => "answered",
    Rejected => "rejected",
    Superseded => "superseded",
});

string_enum!(ResearchOptionClassification, "research option classification", {
    MainPath => "main_path",
    Validation => "validation",
    Exploratory => "exploratory",
    LongRunning => "long_running",
    Blocked => "blocked",
    Maintenance => "maintenance",
});

string_enum!(FactStatus, "fact status", {
    Candidate => "candidate",
    Accepted => "accepted",
    Contested => "contested",
    Rejected => "rejected",
    Superseded => "superseded",
});

string_enum!(AxiomStatus, "axiom status", {
    Active => "active",
    Validated => "validated",
    Contested => "contested",
    Retired => "retired",
});

string_enum!(EvidenceRelation, "evidence relation", {
    Supports => "supports",
    Contradicts => "contradicts",
    Refines => "refines",
    Supersedes => "supersedes",
});

string_enum!(ReviewItemState, "review item state", {
    NeedsReview => "needs_review",
    Reviewed => "reviewed",
    Dismissed => "dismissed",
});

string_enum!(ReviewState, "review state", {
    None => "none",
    NeedsReview => "needs_review",
    Reviewed => "reviewed",
    ApprovalRequired => "approval_required",
    Approved => "approved",
    Rejected => "rejected",
});

string_enum!(OverrideApprovalStatus, "override approval status", {
    Pending => "pending",
    Approved => "approved",
    Rejected => "rejected",
});

string_enum!(BugReportSeverity, "bug report severity", {
    Low => "low",
    Medium => "medium",
    High => "high",
    Critical => "critical",
});

string_enum!(BugReportStatus, "bug report status", {
    Open => "open",
    Triaged => "triaged",
    Resolved => "resolved",
    Dismissed => "dismissed",
});

string_enum!(MatrixStatus, "matrix status", {
    Active => "active",
    Complete => "complete",
    Archived => "archived",
});

string_enum!(MatrixCellStatus, "matrix cell status", {
    Planned => "planned",
    Running => "running",
    Completed => "completed",
    Blocked => "blocked",
    Skipped => "skipped",
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub objective: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Program {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            slug: row.get("slug")?,
            title: row.get("title")?,
            objective: row.get("objective")?,
            status: row.get("status")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProgram<'a> {
    pub slug: &'a str,
    pub title: &'a str,
    pub objective: &'a str,
    pub status: ProgramStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub id: i64,
    pub program_id: i64,
    pub parent_branch_id: Option<i64>,
    pub slug: String,
    pub title: String,
    pub question: String,
    pub rationale: String,
    pub status: String,
    pub decision_summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Branch {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            program_id: row.get("program_id")?,
            parent_branch_id: row.get("parent_branch_id")?,
            slug: row.get("slug")?,
            title: row.get("title")?,
            question: row.get("question")?,
            rationale: row.get("rationale")?,
            status: row.get("status")?,
            decision_summary: row.get("decision_summary")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBranch<'a> {
    pub program_id: i64,
    pub parent_branch_id: Option<i64>,
    pub slug: &'a str,
    pub title: &'a str,
    pub question: &'a str,
    pub rationale: &'a str,
    pub status: BranchStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Experiment {
    pub id: i64,
    pub branch_id: i64,
    pub option_id: Option<i64>,
    pub slug: String,
    pub title: String,
    pub phase: Option<String>,
    pub mode: String,
    pub hypothesis: Option<String>,
    pub setup: Option<String>,
    pub observation_goal: Option<String>,
    pub rationale: Option<String>,
    pub primary_metrics_json: String,
    pub secondary_metrics_json: String,
    pub pass_criteria: Option<String>,
    pub fail_criteria: Option<String>,
    pub allowed_next_steps: Option<String>,
    pub blocked_next_steps: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Experiment {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
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
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewExperiment<'a> {
    pub branch_id: i64,
    pub option_id: Option<i64>,
    pub slug: &'a str,
    pub title: &'a str,
    pub phase: Option<&'a str>,
    pub mode: ExperimentMode,
    pub hypothesis: Option<&'a str>,
    pub setup: Option<&'a str>,
    pub observation_goal: Option<&'a str>,
    pub rationale: Option<&'a str>,
    pub primary_metrics_json: &'a str,
    pub secondary_metrics_json: &'a str,
    pub pass_criteria: Option<&'a str>,
    pub fail_criteria: Option<&'a str>,
    pub allowed_next_steps: Option<&'a str>,
    pub blocked_next_steps: Option<&'a str>,
    pub status: ExperimentStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentUpdate<'a> {
    pub option_id: Option<Option<i64>>,
    pub title: Option<&'a str>,
    pub phase: Option<Option<&'a str>>,
    pub mode: Option<ExperimentMode>,
    pub hypothesis: Option<Option<&'a str>>,
    pub setup: Option<Option<&'a str>>,
    pub observation_goal: Option<Option<&'a str>>,
    pub rationale: Option<Option<&'a str>>,
    pub primary_metrics_json: Option<&'a str>,
    pub secondary_metrics_json: Option<&'a str>,
    pub pass_criteria: Option<Option<&'a str>>,
    pub fail_criteria: Option<Option<&'a str>>,
    pub allowed_next_steps: Option<Option<&'a str>>,
    pub blocked_next_steps: Option<Option<&'a str>>,
    pub status: Option<ExperimentStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub id: i64,
    pub experiment_id: i64,
    pub command: Option<String>,
    pub environment_json: String,
    pub dataset: Option<String>,
    pub code_ref: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: String,
    pub notes: Option<String>,
}

impl Run {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
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
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRun<'a> {
    pub experiment_id: i64,
    pub command: Option<&'a str>,
    pub environment_json: &'a str,
    pub dataset: Option<&'a str>,
    pub code_ref: Option<&'a str>,
    pub notes: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStatusUpdate<'a> {
    pub status: RunStatus,
    pub notes: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metric {
    pub id: i64,
    pub run_id: i64,
    pub name: String,
    pub value: f64,
    pub unit: Option<String>,
    pub higher_is_better: Option<bool>,
    pub split: Option<String>,
    pub metadata_json: String,
}

impl Metric {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let higher_is_better: Option<i64> = row.get("higher_is_better")?;
        Ok(Self {
            id: row.get("id")?,
            run_id: row.get("run_id")?,
            name: row.get("name")?,
            value: row.get("value")?,
            unit: row.get("unit")?,
            higher_is_better: higher_is_better.map(|value| value != 0),
            split: row.get("split")?,
            metadata_json: row.get("metadata_json")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewMetric<'a> {
    pub run_id: i64,
    pub name: &'a str,
    pub value: f64,
    pub unit: Option<&'a str>,
    pub higher_is_better: Option<bool>,
    pub split: Option<&'a str>,
    pub metadata_json: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub id: i64,
    pub run_id: i64,
    pub kind: String,
    pub path: String,
    pub description: String,
    pub checksum: Option<String>,
    pub metadata_json: String,
}

impl Artifact {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            run_id: row.get("run_id")?,
            kind: row.get("kind")?,
            path: row.get("path")?,
            description: row.get("description")?,
            checksum: row.get("checksum")?,
            metadata_json: row.get("metadata_json")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewArtifact<'a> {
    pub run_id: i64,
    pub kind: ArtifactKind,
    pub path: &'a str,
    pub description: &'a str,
    pub checksum: Option<&'a str>,
    pub metadata_json: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub id: i64,
    pub experiment_id: i64,
    pub result_summary: String,
    pub interpretation: String,
    pub limitations: String,
    pub decision: String,
    pub confidence: String,
    pub next_branch_id: Option<i64>,
    pub next_experiment_id: Option<i64>,
    pub proposed_options_json: String,
    pub created_at: String,
}

impl Decision {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
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
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDecision<'a> {
    pub experiment_id: i64,
    pub result_summary: &'a str,
    pub interpretation: &'a str,
    pub limitations: &'a str,
    pub decision: DecisionKind,
    pub confidence: Confidence,
    pub next_branch_id: Option<i64>,
    pub next_experiment_id: Option<i64>,
    pub proposed_options_json: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenQuestion {
    pub id: i64,
    pub program_id: i64,
    pub branch_id: Option<i64>,
    pub slug: String,
    pub question: String,
    pub context: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl OpenQuestion {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            program_id: row.get("program_id")?,
            branch_id: row.get("branch_id")?,
            slug: row.get("slug")?,
            question: row.get("question")?,
            context: row.get("context")?,
            status: row.get("status")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewOpenQuestion<'a> {
    pub program_id: i64,
    pub branch_id: Option<i64>,
    pub slug: &'a str,
    pub question: &'a str,
    pub context: &'a str,
    pub status: OpenQuestionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenQuestionUpdate<'a> {
    pub branch_id: Option<Option<i64>>,
    pub question: Option<&'a str>,
    pub context: Option<&'a str>,
    pub status: Option<OpenQuestionStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpenQuestionFilter {
    pub program_id: Option<i64>,
    pub branch_id: Option<i64>,
    pub status: Option<OpenQuestionStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchOption {
    pub id: i64,
    pub program_id: i64,
    pub branch_id: Option<i64>,
    pub open_question_id: Option<i64>,
    pub source_experiment_id: Option<i64>,
    pub source_decision_id: Option<i64>,
    pub slug: String,
    pub title: String,
    pub hypothesis: Option<String>,
    pub description: String,
    pub classification: String,
    pub status: String,
    pub selection_rationale: Option<String>,
    pub selected_by: Option<String>,
    pub selected_at: Option<String>,
    pub review_state: String,
    pub created_at: String,
    pub updated_at: String,
}

impl ResearchOption {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            program_id: row.get("program_id")?,
            branch_id: row.get("branch_id")?,
            open_question_id: row.get("open_question_id")?,
            source_experiment_id: row.get("source_experiment_id")?,
            source_decision_id: row.get("source_decision_id")?,
            slug: row.get("slug")?,
            title: row.get("title")?,
            hypothesis: row.get("hypothesis")?,
            description: row.get("description")?,
            classification: row.get("classification")?,
            status: row.get("status")?,
            selection_rationale: row.get("selection_rationale")?,
            selected_by: row.get("selected_by")?,
            selected_at: row.get("selected_at")?,
            review_state: row.get("review_state")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewResearchOption<'a> {
    pub program_id: i64,
    pub branch_id: Option<i64>,
    pub open_question_id: Option<i64>,
    pub source_experiment_id: Option<i64>,
    pub source_decision_id: Option<i64>,
    pub slug: &'a str,
    pub title: &'a str,
    pub hypothesis: Option<&'a str>,
    pub description: &'a str,
    pub classification: ResearchOptionClassification,
    pub status: ResearchOptionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchOptionUpdate<'a> {
    pub branch_id: Option<Option<i64>>,
    pub open_question_id: Option<Option<i64>>,
    pub source_experiment_id: Option<Option<i64>>,
    pub source_decision_id: Option<Option<i64>>,
    pub title: Option<&'a str>,
    pub hypothesis: Option<Option<&'a str>>,
    pub description: Option<&'a str>,
    pub classification: Option<ResearchOptionClassification>,
    pub status: Option<ResearchOptionStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResearchOptionFilter {
    pub program_id: Option<i64>,
    pub branch_id: Option<i64>,
    pub status: Option<ResearchOptionStatus>,
    pub classification: Option<ResearchOptionClassification>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub id: i64,
    pub program_id: i64,
    pub branch_id: Option<i64>,
    pub slug: String,
    pub statement: String,
    pub status: String,
    pub confidence: Option<String>,
    pub created_from_experiment_id: Option<i64>,
    pub created_from_decision_id: Option<i64>,
    pub review_state: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Fact {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            program_id: row.get("program_id")?,
            branch_id: row.get("branch_id")?,
            slug: row.get("slug")?,
            statement: row.get("statement")?,
            status: row.get("status")?,
            confidence: row.get("confidence")?,
            created_from_experiment_id: row.get("created_from_experiment_id")?,
            created_from_decision_id: row.get("created_from_decision_id")?,
            review_state: row.get("review_state")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFact<'a> {
    pub program_id: i64,
    pub branch_id: Option<i64>,
    pub slug: &'a str,
    pub statement: &'a str,
    pub status: FactStatus,
    pub confidence: Option<Confidence>,
    pub created_from_experiment_id: Option<i64>,
    pub created_from_decision_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactUpdate<'a> {
    pub branch_id: Option<Option<i64>>,
    pub statement: Option<&'a str>,
    pub status: Option<FactStatus>,
    pub confidence: Option<Option<Confidence>>,
    pub created_from_experiment_id: Option<Option<i64>>,
    pub created_from_decision_id: Option<Option<i64>>,
    pub review_state: Option<ReviewState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FactFilter {
    pub program_id: Option<i64>,
    pub branch_id: Option<i64>,
    pub status: Option<FactStatus>,
    pub review_state: Option<ReviewState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Axiom {
    pub id: i64,
    pub program_id: i64,
    pub branch_id: Option<i64>,
    pub slug: String,
    pub statement: String,
    pub status: String,
    pub created_by_actor: Option<String>,
    pub created_by_agent: bool,
    pub review_state: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Axiom {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let created_by_agent: i64 = row.get("created_by_agent")?;
        Ok(Self {
            id: row.get("id")?,
            program_id: row.get("program_id")?,
            branch_id: row.get("branch_id")?,
            slug: row.get("slug")?,
            statement: row.get("statement")?,
            status: row.get("status")?,
            created_by_actor: row.get("created_by_actor")?,
            created_by_agent: created_by_agent != 0,
            review_state: row.get("review_state")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAxiom<'a> {
    pub program_id: i64,
    pub branch_id: Option<i64>,
    pub slug: &'a str,
    pub statement: &'a str,
    pub status: AxiomStatus,
    pub created_by_actor: Option<&'a str>,
    pub created_by_agent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxiomUpdate<'a> {
    pub branch_id: Option<Option<i64>>,
    pub statement: Option<&'a str>,
    pub status: Option<AxiomStatus>,
    pub created_by_actor: Option<Option<&'a str>>,
    pub review_state: Option<ReviewState>,
    pub approved_by: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AxiomFilter {
    pub program_id: Option<i64>,
    pub branch_id: Option<i64>,
    pub status: Option<AxiomStatus>,
    pub review_state: Option<ReviewState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceLink {
    pub id: i64,
    pub subject_type: String,
    pub subject_id: i64,
    pub relation: String,
    pub experiment_id: Option<i64>,
    pub run_id: Option<i64>,
    pub metric_id: Option<i64>,
    pub artifact_id: Option<i64>,
    pub decision_id: Option<i64>,
    pub report_path: Option<String>,
    pub report_anchor: Option<String>,
    pub summary: String,
    pub created_at: String,
}

impl EvidenceLink {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            subject_type: row.get("subject_type")?,
            subject_id: row.get("subject_id")?,
            relation: row.get("relation")?,
            experiment_id: row.get("experiment_id")?,
            run_id: row.get("run_id")?,
            metric_id: row.get("metric_id")?,
            artifact_id: row.get("artifact_id")?,
            decision_id: row.get("decision_id")?,
            report_path: row.get("report_path")?,
            report_anchor: row.get("report_anchor")?,
            summary: row.get("summary")?,
            created_at: row.get("created_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEvidenceLink<'a> {
    pub relation: EvidenceRelation,
    pub experiment_id: Option<i64>,
    pub run_id: Option<i64>,
    pub metric_id: Option<i64>,
    pub artifact_id: Option<i64>,
    pub decision_id: Option<i64>,
    pub report_path: Option<&'a str>,
    pub report_anchor: Option<&'a str>,
    pub summary: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewItem {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub reason: String,
    pub state: String,
    pub created_at: String,
    pub reviewed_at: Option<String>,
    pub reviewed_by: Option<String>,
    pub notes: Option<String>,
}

impl ReviewItem {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            entity_type: row.get("entity_type")?,
            entity_id: row.get("entity_id")?,
            reason: row.get("reason")?,
            state: row.get("state")?,
            created_at: row.get("created_at")?,
            reviewed_at: row.get("reviewed_at")?,
            reviewed_by: row.get("reviewed_by")?,
            notes: row.get("notes")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewReviewItem<'a> {
    pub entity_type: &'a str,
    pub entity_id: i64,
    pub reason: &'a str,
    pub state: ReviewItemState,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReviewItemFilter {
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub state: Option<ReviewItemState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverrideApproval {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub blocked_work: String,
    pub requested_action: String,
    pub justification: String,
    pub status: OverrideApprovalStatus,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
    pub created_at: String,
}

impl OverrideApproval {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let approved_by = non_empty(row.get::<_, String>("approved_by")?);
        let approved_at = non_empty(row.get::<_, String>("approved_at")?);
        let (status, approved_by) = match approved_by {
            Some(value) if value.starts_with("rejected:") => (
                OverrideApprovalStatus::Rejected,
                non_empty(value.trim_start_matches("rejected:").to_owned()),
            ),
            Some(value) if approved_at.is_some() => (OverrideApprovalStatus::Approved, Some(value)),
            value => (OverrideApprovalStatus::Pending, value),
        };

        Ok(Self {
            id: row.get("id")?,
            entity_type: row.get("entity_type")?,
            entity_id: row.get("entity_id")?,
            blocked_work: row.get("blocked_work")?,
            requested_action: row.get("requested_action")?,
            justification: row.get("justification")?,
            status,
            approved_by,
            approved_at,
            created_at: row.get("created_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewOverrideApprovalRequest<'a> {
    pub entity_type: &'a str,
    pub entity_id: i64,
    pub blocked_work: &'a str,
    pub requested_action: &'a str,
    pub justification: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OverrideApprovalFilter {
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub status: Option<OverrideApprovalStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BugReport {
    pub id: i64,
    pub program_id: Option<i64>,
    pub branch_id: Option<i64>,
    pub experiment_id: Option<i64>,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub status: String,
    pub command: Option<String>,
    pub error: Option<String>,
    pub reproduction: Option<String>,
    pub log_path: Option<String>,
    pub log_excerpt: Option<String>,
    pub reported_by: Option<String>,
    pub resolution_notes: Option<String>,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl BugReport {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            program_id: row.get("program_id")?,
            branch_id: row.get("branch_id")?,
            experiment_id: row.get("experiment_id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            severity: row.get("severity")?,
            status: row.get("status")?,
            command: row.get("command")?,
            error: row.get("error")?,
            reproduction: row.get("reproduction")?,
            log_path: row.get("log_path")?,
            log_excerpt: row.get("log_excerpt")?,
            reported_by: row.get("reported_by")?,
            resolution_notes: row.get("resolution_notes")?,
            resolved_by: row.get("resolved_by")?,
            resolved_at: row.get("resolved_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBugReport<'a> {
    pub program_id: Option<i64>,
    pub branch_id: Option<i64>,
    pub experiment_id: Option<i64>,
    pub title: &'a str,
    pub description: &'a str,
    pub severity: BugReportSeverity,
    pub command: Option<&'a str>,
    pub error: Option<&'a str>,
    pub reproduction: Option<&'a str>,
    pub log_path: Option<&'a str>,
    pub log_excerpt: Option<&'a str>,
    pub reported_by: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BugReportFilter {
    pub program_id: Option<i64>,
    pub branch_id: Option<i64>,
    pub experiment_id: Option<i64>,
    pub status: Option<BugReportStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchMatrix {
    pub id: i64,
    pub program_id: i64,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl ResearchMatrix {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            program_id: row.get("program_id")?,
            slug: row.get("slug")?,
            title: row.get("title")?,
            description: row.get("description")?,
            status: row.get("status")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewResearchMatrix<'a> {
    pub program_id: i64,
    pub slug: &'a str,
    pub title: &'a str,
    pub description: &'a str,
    pub status: MatrixStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResearchMatrixUpdate<'a> {
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub status: Option<MatrixStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixAxis {
    pub id: i64,
    pub matrix_id: i64,
    pub slug: String,
    pub title: String,
    pub position: i64,
    pub created_at: String,
}

impl MatrixAxis {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            matrix_id: row.get("matrix_id")?,
            slug: row.get("slug")?,
            title: row.get("title")?,
            position: row.get("position")?,
            created_at: row.get("created_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMatrixAxis<'a> {
    pub matrix_id: i64,
    pub slug: &'a str,
    pub title: &'a str,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixLevel {
    pub id: i64,
    pub axis_id: i64,
    pub slug: String,
    pub title: String,
    pub position: i64,
    pub created_at: String,
}

impl MatrixLevel {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            axis_id: row.get("axis_id")?,
            slug: row.get("slug")?,
            title: row.get("title")?,
            position: row.get("position")?,
            created_at: row.get("created_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMatrixLevel<'a> {
    pub axis_id: i64,
    pub slug: &'a str,
    pub title: &'a str,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixCell {
    pub id: i64,
    pub matrix_id: i64,
    pub slug: String,
    pub title: String,
    pub coordinates_json: String,
    pub experiment_id: Option<i64>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl MatrixCell {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            matrix_id: row.get("matrix_id")?,
            slug: row.get("slug")?,
            title: row.get("title")?,
            coordinates_json: row.get("coordinates_json")?,
            experiment_id: row.get("experiment_id")?,
            status: row.get("status")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMatrixCell<'a> {
    pub matrix_id: i64,
    pub slug: &'a str,
    pub title: &'a str,
    pub coordinates_json: &'a str,
    pub level_ids_by_axis: &'a [(i64, i64)],
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MatrixCellUpdate<'a> {
    pub experiment_id: Option<Option<i64>>,
    pub status: Option<MatrixCellStatus>,
    pub notes: Option<Option<&'a str>>,
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}
