use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path};

use anyhow::{bail, Context};
use rusqlite::Connection;
use serde::Serialize;

use crate::policy::Policy;
use crate::schema::{
    AxiomFilter, BugReportFilter, EvidenceLink, FactFilter, FactStatus, NewResearchOption,
    NewReviewItem, OpenQuestionFilter, OverrideApprovalFilter, ResearchOption,
    ResearchOptionClassification, ResearchOptionFilter, ResearchOptionStatus, ReviewItemFilter,
    ReviewItemState,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Program,
    Branch,
    Question,
    Option,
    Experiment,
    Run,
    Metric,
    Artifact,
    Decision,
    Fact,
    Axiom,
    ReviewItem,
    OverrideApproval,
    BlockedWork,
    BugReport,
    ReportAnchor,
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Program => "program",
            Self::Branch => "branch",
            Self::Question => "question",
            Self::Option => "option",
            Self::Experiment => "experiment",
            Self::Run => "run",
            Self::Metric => "metric",
            Self::Artifact => "artifact",
            Self::Decision => "decision",
            Self::Fact => "fact",
            Self::Axiom => "axiom",
            Self::ReviewItem => "review_item",
            Self::OverrideApproval => "override_approval",
            Self::BlockedWork => "blocked_work",
            Self::BugReport => "bug_report",
            Self::ReportAnchor => "report_anchor",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Contains,
    Parent,
    DependsOn,
    Answers,
    Supports,
    Contradicts,
    Refines,
    Supersedes,
    Blocks,
    RequiresContext,
    ProducedBy,
    Claims,
    NeedsReview,
    LinkedTo,
}

impl fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Contains => "contains",
            Self::Parent => "parent",
            Self::DependsOn => "depends_on",
            Self::Answers => "answers",
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
            Self::Refines => "refines",
            Self::Supersedes => "supersedes",
            Self::Blocks => "blocks",
            Self::RequiresContext => "requires_context",
            Self::ProducedBy => "produced_by",
            Self::Claims => "claims",
            Self::NeedsReview => "needs_review",
            Self::LinkedTo => "linked_to",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ObligationKind {
    DecisionRequired,
    EvidenceRequired,
    ReviewRequired,
    DependencyRequired,
    BlockedUntilApproval,
    ArtifactRequired,
    ContextRequired,
    ValidationRequired,
}

impl fmt::Display for ObligationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::DecisionRequired => "decision_required",
            Self::EvidenceRequired => "evidence_required",
            Self::ReviewRequired => "review_required",
            Self::DependencyRequired => "dependency_required",
            Self::BlockedUntilApproval => "blocked_until_approval",
            Self::ArtifactRequired => "artifact_required",
            Self::ContextRequired => "context_required",
            Self::ValidationRequired => "validation_required",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    pub status: Option<String>,
    pub source_table: String,
    pub source_id: Option<i64>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphEdge {
    pub id: String,
    pub kind: EdgeKind,
    pub from: String,
    pub to: String,
    pub source: String,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphObligation {
    pub id: String,
    pub kind: ObligationKind,
    pub subject: String,
    pub related_nodes: Vec<String>,
    pub source: String,
    pub status: String,
    pub gloss: String,
    pub approval_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphProjection {
    pub format: String,
    pub canonical_state: String,
    pub generated_from: String,
    pub current_program: Option<String>,
    pub current_branch: Option<String>,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub obligations: Vec<GraphObligation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeCount {
    pub kind: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphSummary {
    pub node_count: usize,
    pub edge_count: usize,
    pub obligation_count: usize,
    pub nodes_by_kind: Vec<NodeCount>,
    pub open_options: Vec<String>,
    pub blocked_paths: Vec<String>,
    pub review_needs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
}

impl fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => f.write_str("info"),
            Self::Warning => f.write_str("warning"),
            Self::Error => f.write_str("error"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationFinding {
    pub check_id: String,
    pub severity: ValidationSeverity,
    pub node_id: Option<String>,
    pub message: String,
    pub repair: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphValidationReport {
    pub status: String,
    pub findings: Vec<ValidationFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NextWorkRecommendation {
    pub option_id: String,
    pub slug: String,
    pub classification: String,
    pub status: String,
    pub reason: String,
    pub policy_reason: String,
    pub graph_reasons: Vec<String>,
    pub validation_warnings: Vec<String>,
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NextWorkReport {
    pub recommendation: Option<NextWorkRecommendation>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalKind {
    CreateReviewItem,
    CreateValidationOption,
    SupersedeAnsweredOption,
}

impl fmt::Display for ProposalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateReviewItem => f.write_str("create_review_item"),
            Self::CreateValidationOption => f.write_str("create_validation_option"),
            Self::SupersedeAnsweredOption => f.write_str("supersede_answered_option"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphProposal {
    pub id: String,
    pub kind: ProposalKind,
    pub affected_nodes: Vec<String>,
    pub rationale: String,
    pub expected_operation: String,
    pub validation_facts: Vec<String>,
    pub approval_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApplyResult {
    pub proposal_id: String,
    pub operation: String,
    pub changed_entity_type: String,
    pub changed_entity_id: i64,
}

#[derive(Debug, Clone, Default)]
pub struct NextOptions {
    pub include_long_running: bool,
}

pub fn build_projection(conn: &Connection, policy: &Policy) -> anyhow::Result<GraphProjection> {
    let mut builder = ProjectionBuilder::new(policy);
    builder.load(conn)?;
    Ok(builder.finish())
}

pub fn summarize_projection(projection: &GraphProjection) -> GraphSummary {
    let mut by_kind = BTreeMap::<String, usize>::new();
    for node in &projection.nodes {
        *by_kind.entry(node.kind.to_string()).or_default() += 1;
    }

    let nodes_by_kind = by_kind
        .into_iter()
        .map(|(kind, count)| NodeCount { kind, count })
        .collect::<Vec<_>>();

    let open_options = projection
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Option && node.status.as_deref() == Some("open"))
        .map(|node| node.id.clone())
        .collect();
    let blocked_paths = projection
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::BlockedWork)
        .map(|node| node.label.clone())
        .collect();
    let review_needs = projection
        .edges
        .iter()
        .filter(|edge| edge.kind == EdgeKind::NeedsReview)
        .map(|edge| edge.from.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    GraphSummary {
        node_count: projection.nodes.len(),
        edge_count: projection.edges.len(),
        obligation_count: projection.obligations.len(),
        nodes_by_kind,
        open_options,
        blocked_paths,
        review_needs,
    }
}

pub fn validate_projection(
    conn: &Connection,
    policy: &Policy,
    projection: &GraphProjection,
) -> anyhow::Result<GraphValidationReport> {
    let mut findings = Vec::new();
    let nodes = projection
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();

    for edge in &projection.edges {
        if !nodes.contains(edge.from.as_str()) {
            findings.push(error(
                "edge_source_missing",
                Some(edge.from.clone()),
                format!("edge {} has missing source node {}", edge.id, edge.from),
                "rebuild the graph projection and inspect the source record that produced the edge",
            ));
        }
        if !nodes.contains(edge.to.as_str()) {
            findings.push(error(
                "edge_target_missing",
                Some(edge.to.clone()),
                format!("edge {} has missing target node {}", edge.id, edge.to),
                "repair or remove the source record that references the missing entity",
            ));
        }
    }

    let mut seen_edges = BTreeSet::new();
    for edge in &projection.edges {
        if !seen_edges.insert(edge.id.clone()) {
            findings.push(error(
                "duplicate_edge_id",
                None,
                format!("duplicate edge id {}", edge.id),
                "make edge id derivation include enough source metadata to be unique",
            ));
        }
    }

    for obligation in &projection.obligations {
        if !nodes.contains(obligation.subject.as_str()) {
            findings.push(error(
                "obligation_subject_missing",
                Some(obligation.subject.clone()),
                format!("obligation {} has missing subject", obligation.id),
                "repair the source record or obligation derivation",
            ));
        }
        for related in &obligation.related_nodes {
            if !nodes.contains(related.as_str()) {
                findings.push(error(
                    "obligation_related_node_missing",
                    Some(related.clone()),
                    format!(
                        "obligation {} references missing related node {}",
                        obligation.id, related
                    ),
                    "repair the source record or obligation derivation",
                ));
            }
        }
    }

    validate_current_policy_refs(projection, &mut findings);
    validate_selected_options(conn, &mut findings)?;
    validate_completed_experiments(conn, policy, &mut findings)?;
    validate_candidate_facts(conn, &mut findings)?;
    validate_review_surfaces(conn, projection, &mut findings)?;
    validate_artifact_roots(conn, policy, &mut findings)?;

    let status = if findings
        .iter()
        .any(|finding| finding.severity == ValidationSeverity::Error)
    {
        "error"
    } else if findings
        .iter()
        .any(|finding| finding.severity == ValidationSeverity::Warning)
    {
        "warning"
    } else {
        "ok"
    }
    .to_owned();

    Ok(GraphValidationReport { status, findings })
}

pub fn has_validation_errors(report: &GraphValidationReport) -> bool {
    report
        .findings
        .iter()
        .any(|finding| finding.severity == ValidationSeverity::Error)
}

pub fn recommend_next(
    conn: &Connection,
    policy: &Policy,
    projection: &GraphProjection,
    options: &NextOptions,
) -> anyhow::Result<NextWorkReport> {
    let Some(program_slug) = policy.current_program.as_deref() else {
        return Ok(NextWorkReport {
            recommendation: None,
            message: "no current program is set".to_owned(),
        });
    };
    let Some(program) = crate::db::get_program_by_slug(conn, program_slug)? else {
        return Ok(NextWorkReport {
            recommendation: None,
            message: format!("current program `{program_slug}` is missing"),
        });
    };
    let branch_id = match policy.current_branch.as_deref() {
        Some(branch_slug) => {
            crate::db::get_branch_by_slug(conn, program.id, branch_slug)?.map(|branch| branch.id)
        }
        None => None,
    };

    let mut candidates = crate::db::list_research_options(
        conn,
        &ResearchOptionFilter {
            program_id: Some(program.id),
            branch_id,
            status: Some(ResearchOptionStatus::Open),
            classification: None,
        },
    )?;
    candidates.retain(|option| option.classification != "blocked");
    if !options.include_long_running {
        candidates.retain(|option| option.classification != "long_running");
    }
    candidates
        .retain(|option| !terminal_experiment_exists_for_option(conn, option).unwrap_or(true));

    if candidates.is_empty() {
        return Ok(NextWorkReport {
            recommendation: None,
            message: "no open unblocked graph-valid options are available".to_owned(),
        });
    }

    let base_preference = classification_preference(policy);
    let validation_pressure = has_unresolved_evidence_pressure(conn, program.id, branch_id)?
        && candidates.iter().any(|option| {
            option.classification == ResearchOptionClassification::Validation.as_str()
        });
    let preference = if validation_pressure {
        validation_pressure_preference(&base_preference)
    } else {
        base_preference.clone()
    };
    candidates.sort_by(|left, right| {
        let left_rank = preference_rank(&preference, &left.classification);
        let right_rank = preference_rank(&preference, &right.classification);
        left_rank
            .cmp(&right_rank)
            .then(left.created_at.cmp(&right.created_at))
            .then(left.id.cmp(&right.id))
    });
    let selected = candidates.remove(0);
    let alternatives = candidates
        .iter()
        .take(5)
        .map(|option| format!("{} [{}]", option.slug, option.classification))
        .collect::<Vec<_>>();
    let validation_warnings = validate_projection(conn, policy, projection)?
        .findings
        .into_iter()
        .filter(|finding| finding.severity == ValidationSeverity::Warning)
        .map(|finding| format!("{}: {}", finding.check_id, finding.message))
        .collect();

    let option_id = option_node_id(&selected);
    let mut graph_reasons = graph_reasons_for_option(projection, &option_id);
    if validation_pressure {
        graph_reasons.push(
            "unresolved candidate or contested facts create validation pressure before main-path scaling"
                .to_owned(),
        );
    }
    Ok(NextWorkReport {
        message: format!("recommended {}", selected.slug),
        recommendation: Some(NextWorkRecommendation {
            option_id: option_id.clone(),
            slug: selected.slug.clone(),
            classification: selected.classification.clone(),
            status: selected.status.clone(),
            reason: if validation_pressure {
                "open validation option selected because unresolved evidence quality gates main-path work"
                    .to_owned()
            } else {
                "open option selected by graph topology and policy classification order".to_owned()
            },
            policy_reason: if validation_pressure {
                format!(
                    "unresolved evidence pressure overrides default classifications: {}",
                    base_preference.join(", ")
                )
            } else {
                format!("preferred classifications: {}", preference.join(", "))
            },
            graph_reasons,
            validation_warnings,
            alternatives,
        }),
    })
}

pub fn propose(
    conn: &Connection,
    policy: &Policy,
    projection: &GraphProjection,
) -> anyhow::Result<Vec<GraphProposal>> {
    let mut proposals = Vec::new();
    propose_missing_review_items(conn, &mut proposals)?;
    propose_validation_options(conn, &mut proposals)?;
    propose_superseded_options(conn, &mut proposals)?;
    proposals.sort_by(|left, right| left.id.cmp(&right.id));
    proposals.dedup_by(|left, right| left.id == right.id);

    let validation = validate_projection(conn, policy, projection)?;
    let blocking = validation
        .findings
        .into_iter()
        .filter(|finding| finding.severity == ValidationSeverity::Error)
        .map(|finding| format!("{}: {}", finding.check_id, finding.message))
        .collect::<Vec<_>>();
    for proposal in &mut proposals {
        proposal.validation_facts.extend(blocking.clone());
    }
    Ok(proposals)
}

pub fn apply_proposal(
    conn: &Connection,
    policy: &Policy,
    proposal_id: &str,
) -> anyhow::Result<ApplyResult> {
    let projection = build_projection(conn, policy)?;
    let proposals = propose(conn, policy, &projection)?;
    let proposal = proposals
        .into_iter()
        .find(|proposal| proposal.id == proposal_id)
        .with_context(|| format!("proposal `{proposal_id}` is unknown or stale"))?;

    match proposal.kind {
        ProposalKind::CreateReviewItem => apply_create_review_item(conn, proposal),
        ProposalKind::CreateValidationOption => apply_create_validation_option(conn, proposal),
        ProposalKind::SupersedeAnsweredOption => apply_supersede_option(conn, proposal),
    }
}

pub fn format_summary(projection: &GraphProjection) -> String {
    let summary = summarize_projection(projection);
    let mut output = String::new();
    output.push_str("Graph Projection:\n");
    output.push_str(&format!("format: {}\n", projection.format));
    output.push_str(&format!(
        "canonical_state: {}\n",
        projection.canonical_state
    ));
    output.push_str(&format!("nodes: {}\n", summary.node_count));
    output.push_str(&format!("edges: {}\n", summary.edge_count));
    output.push_str(&format!("obligations: {}\n", summary.obligation_count));
    output.push_str("nodes_by_kind:\n");
    for count in summary.nodes_by_kind {
        output.push_str(&format!("- {}: {}\n", count.kind, count.count));
    }
    output.push_str("open_options:\n");
    push_list(&mut output, &summary.open_options);
    output.push_str("blocked_paths:\n");
    push_list(&mut output, &summary.blocked_paths);
    output.push_str("review_needs:\n");
    push_list(&mut output, &summary.review_needs);
    output
}

pub fn format_show(projection: &GraphProjection) -> String {
    let mut output = format_summary(projection);
    output.push_str("edges:\n");
    for edge in projection.edges.iter().take(80) {
        output.push_str(&format!("- {} --{}--> {}\n", edge.from, edge.kind, edge.to));
    }
    if projection.edges.len() > 80 {
        output.push_str(&format!(
            "- ... {} more edges\n",
            projection.edges.len() - 80
        ));
    }
    output.push_str("obligations:\n");
    for obligation in &projection.obligations {
        output.push_str(&format!(
            "- {} [{}] {}: {}\n",
            obligation.id, obligation.status, obligation.kind, obligation.gloss
        ));
    }
    output
}

pub fn format_validation(report: &GraphValidationReport) -> String {
    if report.findings.is_empty() {
        return "ok: no graph validation findings\n".to_owned();
    }
    let mut output = String::new();
    for finding in &report.findings {
        let node = finding
            .node_id
            .as_deref()
            .map(|node| format!(" {node}"))
            .unwrap_or_default();
        output.push_str(&format!(
            "{} [{}]{}: {}\n  repair: {}\n",
            finding.severity, finding.check_id, node, finding.message, finding.repair
        ));
    }
    output
}

pub fn format_next(report: &NextWorkReport) -> String {
    let Some(recommendation) = report.recommendation.as_ref() else {
        return format!("Recommended Next Option:\nnone\nwhy: {}\n", report.message);
    };
    let mut output = String::new();
    output.push_str("Recommended Next Option:\n");
    output.push_str(&format!(
        "- {} [{}]\n",
        recommendation.slug, recommendation.classification
    ));
    output.push_str(&format!("  why: {}\n", recommendation.reason));
    output.push_str(&format!("  policy: {}\n", recommendation.policy_reason));
    output.push_str("  graph:\n");
    push_indented_list(&mut output, &recommendation.graph_reasons, "    ");
    if !recommendation.validation_warnings.is_empty() {
        output.push_str("  validation_warnings:\n");
        push_indented_list(&mut output, &recommendation.validation_warnings, "    ");
    }
    output.push_str("Alternatives:\n");
    push_list(&mut output, &recommendation.alternatives);
    output
}

pub fn format_proposals(proposals: &[GraphProposal]) -> String {
    if proposals.is_empty() {
        return "No graph proposals.\n".to_owned();
    }
    let mut output = String::new();
    for proposal in proposals {
        output.push_str(&format!("Proposal: {}\n", proposal.id));
        output.push_str(&format!("kind: {}\n", proposal.kind));
        output.push_str(&format!("rationale: {}\n", proposal.rationale));
        output.push_str(&format!("operation: {}\n", proposal.expected_operation));
        output.push_str(&format!(
            "approval_required: {}\n",
            proposal.approval_required
        ));
        output.push_str("affected_nodes:\n");
        push_list(&mut output, &proposal.affected_nodes);
    }
    output
}

struct ProjectionBuilder<'a> {
    policy: &'a Policy,
    nodes: BTreeMap<String, GraphNode>,
    edges: BTreeMap<String, GraphEdge>,
    obligations: BTreeMap<String, GraphObligation>,
}

impl<'a> ProjectionBuilder<'a> {
    fn new(policy: &'a Policy) -> Self {
        Self {
            policy,
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            obligations: BTreeMap::new(),
        }
    }

    fn load(&mut self, conn: &Connection) -> anyhow::Result<()> {
        let programs = crate::db::list_programs(conn)?;
        for program in &programs {
            self.add_node(GraphNode {
                id: program_node_id(program),
                kind: NodeKind::Program,
                label: program.title.clone(),
                status: Some(program.status.clone()),
                source_table: "program".to_owned(),
                source_id: Some(program.id),
                metadata: map([
                    ("slug", program.slug.as_str()),
                    ("objective", program.objective.as_str()),
                ]),
            });
        }

        for program in &programs {
            let branches = crate::db::list_branches(conn, program.id)?;
            for branch in &branches {
                let branch_id = branch_node_id(branch);
                self.add_node(GraphNode {
                    id: branch_id.clone(),
                    kind: NodeKind::Branch,
                    label: branch.title.clone(),
                    status: Some(branch.status.clone()),
                    source_table: "branch".to_owned(),
                    source_id: Some(branch.id),
                    metadata: map([
                        ("slug", branch.slug.as_str()),
                        ("question", branch.question.as_str()),
                    ]),
                });
                self.add_edge(
                    EdgeKind::Contains,
                    program_node_id(program),
                    branch_id.clone(),
                    format!("branch:{}", branch.id),
                );
                if let Some(parent_branch_id) = branch.parent_branch_id {
                    self.add_edge(
                        EdgeKind::Parent,
                        branch_id.clone(),
                        id_node("branch", parent_branch_id),
                        format!("branch:{}:parent", branch.id),
                    );
                }
                self.load_experiments(conn, branch)?;
            }
        }

        self.load_questions_options_knowledge(conn)?;
        self.load_review_overrides_bugs(conn)?;
        self.load_policy_nodes();
        Ok(())
    }

    fn load_experiments(
        &mut self,
        conn: &Connection,
        branch: &crate::schema::Branch,
    ) -> anyhow::Result<()> {
        let experiments = crate::db::list_experiments(conn, branch.id)?;
        for experiment in &experiments {
            let experiment_id = experiment_node_id(experiment);
            self.add_node(GraphNode {
                id: experiment_id.clone(),
                kind: NodeKind::Experiment,
                label: experiment.title.clone(),
                status: Some(experiment.status.clone()),
                source_table: "experiment".to_owned(),
                source_id: Some(experiment.id),
                metadata: map([
                    ("slug", experiment.slug.as_str()),
                    ("mode", experiment.mode.as_str()),
                ]),
            });
            self.add_edge(
                EdgeKind::Contains,
                branch_node_id(branch),
                experiment_id.clone(),
                format!("experiment:{}", experiment.id),
            );
            if let Some(option_id) = experiment.option_id {
                self.add_edge(
                    EdgeKind::Answers,
                    experiment_id.clone(),
                    id_node("research_option", option_id),
                    format!("experiment:{}:option", experiment.id),
                );
            }
            if self.policy.required_decision_after_experiment
                && terminal_experiment_status(&experiment.status)
            {
                let has_decision = crate::db::experiment_has_decision(conn, experiment.id)?;
                self.add_obligation(GraphObligation {
                    id: format!("obligation:decision_required:experiment:{}", experiment.id),
                    kind: ObligationKind::DecisionRequired,
                    subject: experiment_id.clone(),
                    related_nodes: Vec::new(),
                    source: "policy.required_decision_after_experiment".to_owned(),
                    status: if has_decision { "satisfied" } else { "open" }.to_owned(),
                    gloss: "terminal experiment requires a decision".to_owned(),
                    approval_required: false,
                });
            }

            for run in crate::db::list_runs_by_experiment(conn, experiment.id)? {
                let run_id = run_node_id(&run);
                self.add_node(GraphNode {
                    id: run_id.clone(),
                    kind: NodeKind::Run,
                    label: run
                        .command
                        .clone()
                        .unwrap_or_else(|| format!("run {}", run.id)),
                    status: Some(run.status.clone()),
                    source_table: "run".to_owned(),
                    source_id: Some(run.id),
                    metadata: BTreeMap::new(),
                });
                self.add_edge(
                    EdgeKind::ProducedBy,
                    run_id.clone(),
                    experiment_id.clone(),
                    format!("run:{}", run.id),
                );
                for metric in crate::db::list_metrics_by_run(conn, run.id)? {
                    let metric_id = metric_node_id(&metric);
                    self.add_node(GraphNode {
                        id: metric_id.clone(),
                        kind: NodeKind::Metric,
                        label: format!("{}={}", metric.name, metric.value),
                        status: None,
                        source_table: "metric".to_owned(),
                        source_id: Some(metric.id),
                        metadata: map([
                            ("name", metric.name.as_str()),
                            ("value", &metric.value.to_string()),
                        ]),
                    });
                    self.add_edge(
                        EdgeKind::ProducedBy,
                        metric_id,
                        run_id.clone(),
                        format!("metric:{}", metric.id),
                    );
                }
                for artifact in crate::db::list_artifacts_by_run(conn, run.id)? {
                    let artifact_id = artifact_node_id(&artifact);
                    self.add_node(GraphNode {
                        id: artifact_id.clone(),
                        kind: NodeKind::Artifact,
                        label: artifact.path.clone(),
                        status: Some(artifact.kind.clone()),
                        source_table: "artifact".to_owned(),
                        source_id: Some(artifact.id),
                        metadata: map([("path", artifact.path.as_str())]),
                    });
                    self.add_edge(
                        EdgeKind::ProducedBy,
                        artifact_id,
                        run_id.clone(),
                        format!("artifact:{}", artifact.id),
                    );
                }
            }

            for decision in crate::db::list_decisions_by_experiment(conn, experiment.id)? {
                let decision_id = decision_node_id(&decision);
                self.add_node(GraphNode {
                    id: decision_id.clone(),
                    kind: NodeKind::Decision,
                    label: decision.decision.clone(),
                    status: Some(decision.confidence.clone()),
                    source_table: "decision".to_owned(),
                    source_id: Some(decision.id),
                    metadata: map([("result", decision.result_summary.as_str())]),
                });
                self.add_edge(
                    EdgeKind::Claims,
                    decision_id.clone(),
                    experiment_id.clone(),
                    format!("decision:{}", decision.id),
                );
                if let Some(next_branch_id) = decision.next_branch_id {
                    self.add_edge(
                        EdgeKind::DependsOn,
                        id_node("branch", next_branch_id),
                        decision_id.clone(),
                        format!("decision:{}:next_branch", decision.id),
                    );
                }
                if let Some(next_experiment_id) = decision.next_experiment_id {
                    self.add_edge(
                        EdgeKind::DependsOn,
                        id_node("experiment", next_experiment_id),
                        decision_id,
                        format!("decision:{}:next_experiment", decision.id),
                    );
                }
            }
        }
        Ok(())
    }

    fn load_questions_options_knowledge(&mut self, conn: &Connection) -> anyhow::Result<()> {
        let questions = crate::db::list_open_questions(conn, &OpenQuestionFilter::default())?;
        for question in &questions {
            let question_id = question_node_id(question);
            self.add_node(GraphNode {
                id: question_id.clone(),
                kind: NodeKind::Question,
                label: question.question.clone(),
                status: Some(question.status.clone()),
                source_table: "open_question".to_owned(),
                source_id: Some(question.id),
                metadata: map([("slug", question.slug.as_str())]),
            });
            self.add_edge(
                EdgeKind::Contains,
                id_node("program", question.program_id),
                question_id.clone(),
                format!("open_question:{}", question.id),
            );
            if let Some(branch_id) = question.branch_id {
                self.add_edge(
                    EdgeKind::RequiresContext,
                    question_id,
                    id_node("branch", branch_id),
                    format!("open_question:{}:branch", question.id),
                );
            }
        }

        let options = crate::db::list_research_options(conn, &ResearchOptionFilter::default())?;
        for option in &options {
            let option_id = option_node_id(option);
            self.add_node(GraphNode {
                id: option_id.clone(),
                kind: NodeKind::Option,
                label: option.title.clone(),
                status: Some(option.status.clone()),
                source_table: "research_option".to_owned(),
                source_id: Some(option.id),
                metadata: map([
                    ("slug", option.slug.as_str()),
                    ("classification", option.classification.as_str()),
                    ("review_state", option.review_state.as_str()),
                ]),
            });
            self.add_edge(
                EdgeKind::Contains,
                id_node("program", option.program_id),
                option_id.clone(),
                format!("research_option:{}", option.id),
            );
            if let Some(branch_id) = option.branch_id {
                self.add_edge(
                    EdgeKind::RequiresContext,
                    option_id.clone(),
                    id_node("branch", branch_id),
                    format!("research_option:{}:branch", option.id),
                );
            }
            if let Some(question_id) = option.open_question_id {
                self.add_edge(
                    EdgeKind::Answers,
                    option_id.clone(),
                    id_node("open_question", question_id),
                    format!("research_option:{}:question", option.id),
                );
            }
            if let Some(experiment_id) = option.source_experiment_id {
                self.add_edge(
                    EdgeKind::ProducedBy,
                    option_id.clone(),
                    id_node("experiment", experiment_id),
                    format!("research_option:{}:source_experiment", option.id),
                );
            }
            if let Some(decision_id) = option.source_decision_id {
                self.add_edge(
                    EdgeKind::ProducedBy,
                    option_id.clone(),
                    id_node("decision", decision_id),
                    format!("research_option:{}:source_decision", option.id),
                );
            }
            if option.classification == "blocked" {
                self.add_obligation(GraphObligation {
                    id: format!("obligation:blocked_until_approval:option:{}", option.id),
                    kind: ObligationKind::BlockedUntilApproval,
                    subject: option_id,
                    related_nodes: Vec::new(),
                    source: "research_option.classification".to_owned(),
                    status: "open".to_owned(),
                    gloss: "blocked option requires explicit approval before selection".to_owned(),
                    approval_required: true,
                });
            }
        }

        self.load_facts_axioms(conn)?;
        Ok(())
    }

    fn load_facts_axioms(&mut self, conn: &Connection) -> anyhow::Result<()> {
        let facts = crate::db::list_facts(conn, &FactFilter::default())?;
        for fact in &facts {
            let fact_id = fact_node_id(fact);
            self.add_node(GraphNode {
                id: fact_id.clone(),
                kind: NodeKind::Fact,
                label: fact.statement.clone(),
                status: Some(fact.status.clone()),
                source_table: "fact".to_owned(),
                source_id: Some(fact.id),
                metadata: map([
                    ("slug", fact.slug.as_str()),
                    ("review_state", fact.review_state.as_str()),
                ]),
            });
            self.add_edge(
                EdgeKind::Contains,
                id_node("program", fact.program_id),
                fact_id.clone(),
                format!("fact:{}", fact.id),
            );
            if let Some(branch_id) = fact.branch_id {
                self.add_edge(
                    EdgeKind::RequiresContext,
                    fact_id.clone(),
                    id_node("branch", branch_id),
                    format!("fact:{}:branch", fact.id),
                );
            }
            if let Some(experiment_id) = fact.created_from_experiment_id {
                self.add_edge(
                    EdgeKind::ProducedBy,
                    fact_id.clone(),
                    id_node("experiment", experiment_id),
                    format!("fact:{}:created_from_experiment", fact.id),
                );
            }
            if let Some(decision_id) = fact.created_from_decision_id {
                self.add_edge(
                    EdgeKind::ProducedBy,
                    fact_id.clone(),
                    id_node("decision", decision_id),
                    format!("fact:{}:created_from_decision", fact.id),
                );
            }
            self.load_evidence_links(conn, "fact", fact.id, &fact_id)?;
            if fact.status == "candidate" {
                self.add_obligation(GraphObligation {
                    id: format!("obligation:validation_required:fact:{}", fact.id),
                    kind: ObligationKind::ValidationRequired,
                    subject: fact_id,
                    related_nodes: Vec::new(),
                    source: "fact.status".to_owned(),
                    status: "open".to_owned(),
                    gloss: "candidate fact needs validation or review before becoming hard context"
                        .to_owned(),
                    approval_required: false,
                });
            }
        }

        let axioms = crate::db::list_axioms(conn, &AxiomFilter::default())?;
        for axiom in &axioms {
            let axiom_id = axiom_node_id(axiom);
            self.add_node(GraphNode {
                id: axiom_id.clone(),
                kind: NodeKind::Axiom,
                label: axiom.statement.clone(),
                status: Some(axiom.status.clone()),
                source_table: "axiom".to_owned(),
                source_id: Some(axiom.id),
                metadata: map([
                    ("slug", axiom.slug.as_str()),
                    ("review_state", axiom.review_state.as_str()),
                ]),
            });
            self.add_edge(
                EdgeKind::Contains,
                id_node("program", axiom.program_id),
                axiom_id.clone(),
                format!("axiom:{}", axiom.id),
            );
            if let Some(branch_id) = axiom.branch_id {
                self.add_edge(
                    EdgeKind::RequiresContext,
                    axiom_id.clone(),
                    id_node("branch", branch_id),
                    format!("axiom:{}:branch", axiom.id),
                );
            }
            self.load_evidence_links(conn, "axiom", axiom.id, &axiom_id)?;
        }
        Ok(())
    }

    fn load_evidence_links(
        &mut self,
        conn: &Connection,
        subject_type: &str,
        subject_id: i64,
        subject_node: &str,
    ) -> anyhow::Result<()> {
        for link in crate::db::list_evidence_links(conn, subject_type, subject_id)? {
            if let Some(report_path) = link.report_path.as_deref() {
                let anchor = link.report_anchor.as_deref().unwrap_or("");
                let node_id = report_anchor_node_id(report_path, anchor);
                self.add_node(GraphNode {
                    id: node_id,
                    kind: NodeKind::ReportAnchor,
                    label: if anchor.is_empty() {
                        report_path.to_owned()
                    } else {
                        format!("{report_path}#{anchor}")
                    },
                    status: None,
                    source_table: "evidence_link.report".to_owned(),
                    source_id: Some(link.id),
                    metadata: map([("path", report_path), ("anchor", anchor)]),
                });
            }
            let kind = evidence_edge_kind(&link);
            for target in evidence_targets(&link) {
                self.add_edge(
                    kind.clone(),
                    subject_node.to_owned(),
                    target,
                    format!("evidence_link:{}", link.id),
                );
            }
        }
        Ok(())
    }

    fn load_review_overrides_bugs(&mut self, conn: &Connection) -> anyhow::Result<()> {
        for review in crate::db::list_review_items(conn, &ReviewItemFilter::default())? {
            let review_id = review_node_id(&review);
            let target = entity_node_id(&review.entity_type, review.entity_id);
            self.add_node(GraphNode {
                id: review_id.clone(),
                kind: NodeKind::ReviewItem,
                label: review.reason.clone(),
                status: Some(review.state.clone()),
                source_table: "review_item".to_owned(),
                source_id: Some(review.id),
                metadata: map([
                    ("entity_type", review.entity_type.as_str()),
                    ("entity_id", &review.entity_id.to_string()),
                ]),
            });
            if review.state == ReviewItemState::NeedsReview.as_str() {
                self.add_edge(
                    EdgeKind::NeedsReview,
                    target,
                    review_id,
                    format!("review_item:{}", review.id),
                );
            }
        }

        for approval in crate::db::list_override_approvals(
            conn,
            &OverrideApprovalFilter {
                entity_type: None,
                entity_id: None,
                status: None,
            },
        )? {
            let approval_id = override_node_id(&approval);
            self.add_node(GraphNode {
                id: approval_id.clone(),
                kind: NodeKind::OverrideApproval,
                label: approval.requested_action.clone(),
                status: Some(approval.status.to_string()),
                source_table: "override_approval".to_owned(),
                source_id: Some(approval.id),
                metadata: map([("blocked_work", approval.blocked_work.as_str())]),
            });
            let blocked_node_id = if approval.entity_type == "blocked_work" {
                let node_id = format!("blocked_work:{}", stable_text_key(&approval.blocked_work));
                if !self.nodes.contains_key(&node_id) {
                    self.add_node(GraphNode {
                        id: node_id.clone(),
                        kind: NodeKind::BlockedWork,
                        label: approval.blocked_work.clone(),
                        status: Some("override_requested".to_owned()),
                        source_table: "override_approval.blocked_work".to_owned(),
                        source_id: Some(approval.id),
                        metadata: map([("override_approval", &approval.id.to_string())]),
                    });
                }
                node_id
            } else {
                entity_node_id(&approval.entity_type, approval.entity_id)
            };
            self.add_edge(
                EdgeKind::Blocks,
                blocked_node_id,
                approval_id,
                format!("override_approval:{}", approval.id),
            );
        }

        for bug in crate::db::list_bug_reports(conn, &BugReportFilter::default())? {
            let bug_id = bug_node_id(&bug);
            self.add_node(GraphNode {
                id: bug_id.clone(),
                kind: NodeKind::BugReport,
                label: bug.title.clone(),
                status: Some(bug.status.clone()),
                source_table: "bug_report".to_owned(),
                source_id: Some(bug.id),
                metadata: map([("severity", bug.severity.as_str())]),
            });
            if let Some(program_id) = bug.program_id {
                self.add_edge(
                    EdgeKind::LinkedTo,
                    bug_id.clone(),
                    id_node("program", program_id),
                    format!("bug_report:{}:program", bug.id),
                );
            }
            if let Some(branch_id) = bug.branch_id {
                self.add_edge(
                    EdgeKind::LinkedTo,
                    bug_id.clone(),
                    id_node("branch", branch_id),
                    format!("bug_report:{}:branch", bug.id),
                );
            }
            if let Some(experiment_id) = bug.experiment_id {
                self.add_edge(
                    EdgeKind::LinkedTo,
                    bug_id,
                    id_node("experiment", experiment_id),
                    format!("bug_report:{}:experiment", bug.id),
                );
            }
        }
        Ok(())
    }

    fn load_policy_nodes(&mut self) {
        for (index, blocked) in self.policy.blocked_work.iter().enumerate() {
            let node_id = format!("blocked_work:{}", stable_text_key(blocked));
            self.add_node(GraphNode {
                id: node_id.clone(),
                kind: NodeKind::BlockedWork,
                label: blocked.clone(),
                status: Some("active".to_owned()),
                source_table: "policy.blocked_work".to_owned(),
                source_id: None,
                metadata: map([("index", &index.to_string())]),
            });
            self.add_obligation(GraphObligation {
                id: format!("obligation:blocked_policy:{index}"),
                kind: ObligationKind::BlockedUntilApproval,
                subject: node_id,
                related_nodes: Vec::new(),
                source: "policy.blocked_work".to_owned(),
                status: "open".to_owned(),
                gloss: blocked.clone(),
                approval_required: self.policy.require_human_approval_for_blocked_overrides,
            });
        }
    }

    fn add_node(&mut self, node: GraphNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    fn add_edge(&mut self, kind: EdgeKind, from: String, to: String, source: String) {
        let id = format!("edge:{kind}:{from}:{to}:{source}");
        self.edges.entry(id.clone()).or_insert(GraphEdge {
            id,
            kind,
            from,
            to,
            source,
            metadata: BTreeMap::new(),
        });
    }

    fn add_obligation(&mut self, obligation: GraphObligation) {
        self.obligations.insert(obligation.id.clone(), obligation);
    }

    fn finish(self) -> GraphProjection {
        GraphProjection {
            format: "ldgr-research.graph_projection.v1".to_owned(),
            canonical_state: ".ldgr/research/research.db".to_owned(),
            generated_from: "ledger".to_owned(),
            current_program: self.policy.current_program.clone(),
            current_branch: self.policy.current_branch.clone(),
            nodes: self.nodes.into_values().collect(),
            edges: self.edges.into_values().collect(),
            obligations: self.obligations.into_values().collect(),
        }
    }
}

fn validate_current_policy_refs(
    projection: &GraphProjection,
    findings: &mut Vec<ValidationFinding>,
) {
    let nodes = projection
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(slug) = projection.current_program.as_deref() {
        if !projection.nodes.iter().any(|node| {
            node.kind == NodeKind::Program
                && node.metadata.get("slug").map(String::as_str) == Some(slug)
        }) {
            findings.push(error(
                "current_program_missing",
                Some(format!("program:{slug}")),
                format!("current program `{slug}` is not present in the graph"),
                "create the program or update policy.current_program",
            ));
        }
    }
    if let Some(slug) = projection.current_branch.as_deref() {
        let exists = projection.nodes.iter().any(|node| {
            node.kind == NodeKind::Branch
                && node.metadata.get("slug").map(String::as_str) == Some(slug)
        });
        if !exists || nodes.is_empty() {
            findings.push(error(
                "current_branch_missing",
                Some(format!("branch:{slug}")),
                format!("current branch `{slug}` is not present in the graph"),
                "create the branch or update policy.current_branch",
            ));
        }
    }
}

fn validate_selected_options(
    conn: &Connection,
    findings: &mut Vec<ValidationFinding>,
) -> anyhow::Result<()> {
    let options = crate::db::list_research_options(
        conn,
        &ResearchOptionFilter {
            status: Some(ResearchOptionStatus::Selected),
            ..Default::default()
        },
    )?;
    for option in options {
        if option
            .selection_rationale
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            findings.push(error(
                "selected_option_missing_rationale",
                Some(option_node_id(&option)),
                format!("selected option `{}` has no selection rationale", option.slug),
                "run `ldgr-research option select <slug> --rationale <why>` with a non-empty rationale",
            ));
        }
    }
    Ok(())
}

fn validate_completed_experiments(
    conn: &Connection,
    policy: &Policy,
    findings: &mut Vec<ValidationFinding>,
) -> anyhow::Result<()> {
    if !policy.required_decision_after_experiment {
        return Ok(());
    }
    for program in crate::db::list_programs(conn)? {
        for branch in crate::db::list_branches(conn, program.id)? {
            for experiment in crate::db::list_experiments(conn, branch.id)? {
                if terminal_experiment_status(&experiment.status)
                    && !crate::db::experiment_has_decision(conn, experiment.id)?
                {
                    findings.push(error(
                        "terminal_experiment_missing_decision",
                        Some(experiment_node_id(&experiment)),
                        format!("terminal experiment `{}` has no decision", experiment.slug),
                        "record a decision with `ldgr-research decision add` before treating the experiment as complete",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_candidate_facts(
    conn: &Connection,
    findings: &mut Vec<ValidationFinding>,
) -> anyhow::Result<()> {
    for fact in crate::db::list_facts(
        conn,
        &FactFilter {
            status: Some(crate::schema::FactStatus::Candidate),
            ..Default::default()
        },
    )? {
        let links = crate::db::list_evidence_links(conn, "fact", fact.id)?;
        if links.is_empty() {
            findings.push(error(
                "candidate_fact_missing_evidence",
                Some(fact_node_id(&fact)),
                format!("candidate fact `{}` has no evidence links", fact.slug),
                "add supporting or contradicting evidence before relying on this fact",
            ));
        } else {
            findings.push(warning(
                "candidate_fact_needs_validation",
                Some(fact_node_id(&fact)),
                format!(
                    "candidate fact `{}` still needs validation or review",
                    fact.slug
                ),
                "review the fact or create a validation option before promoting it to hard context",
            ));
        }
    }
    Ok(())
}

fn validate_review_surfaces(
    conn: &Connection,
    projection: &GraphProjection,
    findings: &mut Vec<ValidationFinding>,
) -> anyhow::Result<()> {
    let review_edges = projection
        .edges
        .iter()
        .filter(|edge| edge.kind == EdgeKind::NeedsReview)
        .map(|edge| edge.from.as_str())
        .collect::<BTreeSet<_>>();

    for option in crate::db::list_research_options(conn, &ResearchOptionFilter::default())? {
        let option_id = option_node_id(&option);
        if option.review_state == "needs_review" && !review_edges.contains(option_id.as_str()) {
            findings.push(warning(
                "review_state_missing_review_item",
                Some(option_id),
                format!(
                    "option `{}` has review_state needs_review without an open review item",
                    option.slug
                ),
                "create a review item or clear review_state after review",
            ));
        }
    }
    for fact in crate::db::list_facts(conn, &FactFilter::default())? {
        let fact_id = fact_node_id(&fact);
        if fact.review_state == "needs_review" && !review_edges.contains(fact_id.as_str()) {
            findings.push(warning(
                "review_state_missing_review_item",
                Some(fact_id),
                format!(
                    "fact `{}` has review_state needs_review without an open review item",
                    fact.slug
                ),
                "create a review item or mark the fact reviewed",
            ));
        }
    }
    Ok(())
}

fn validate_artifact_roots(
    conn: &Connection,
    policy: &Policy,
    findings: &mut Vec<ValidationFinding>,
) -> anyhow::Result<()> {
    if policy.allowed_artifact_roots.is_empty() {
        return Ok(());
    }
    for program in crate::db::list_programs(conn)? {
        for branch in crate::db::list_branches(conn, program.id)? {
            for experiment in crate::db::list_experiments(conn, branch.id)? {
                for artifact in crate::db::list_artifacts_by_experiment(conn, experiment.id)? {
                    if !path_allowed(&artifact.path, &policy.allowed_artifact_roots) {
                        findings.push(error(
                            "artifact_outside_allowed_roots",
                            Some(artifact_node_id(&artifact)),
                            format!("artifact `{}` is outside allowed roots", artifact.path),
                            format!(
                                "move the artifact under one of: {}",
                                policy.allowed_artifact_roots.join(", ")
                            ),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn propose_missing_review_items(
    conn: &Connection,
    proposals: &mut Vec<GraphProposal>,
) -> anyhow::Result<()> {
    for option in crate::db::list_research_options(conn, &ResearchOptionFilter::default())? {
        if option.review_state == "needs_review"
            && !has_open_review_item(conn, "research_option", option.id)?
        {
            proposals.push(GraphProposal {
                id: format!("proposal:create_review_item:research_option:{}", option.id),
                kind: ProposalKind::CreateReviewItem,
                affected_nodes: vec![option_node_id(&option)],
                rationale: format!(
                    "option `{}` is marked needs_review without a review item",
                    option.slug
                ),
                expected_operation: format!("create review_item for research_option {}", option.id),
                validation_facts: vec!["review_state_missing_review_item".to_owned()],
                approval_required: false,
            });
        }
        if option.classification == "blocked"
            && !has_open_review_item(conn, "research_option", option.id)?
        {
            proposals.push(GraphProposal {
                id: format!("proposal:create_review_item:blocked_option:{}", option.id),
                kind: ProposalKind::CreateReviewItem,
                affected_nodes: vec![option_node_id(&option)],
                rationale: format!(
                    "blocked option `{}` should be visible for review",
                    option.slug
                ),
                expected_operation: format!("create review_item for research_option {}", option.id),
                validation_facts: vec!["blocked_option_requires_review".to_owned()],
                approval_required: false,
            });
        }
    }
    for fact in crate::db::list_facts(conn, &FactFilter::default())? {
        if fact.review_state == "needs_review" && !has_open_review_item(conn, "fact", fact.id)? {
            proposals.push(GraphProposal {
                id: format!("proposal:create_review_item:fact:{}", fact.id),
                kind: ProposalKind::CreateReviewItem,
                affected_nodes: vec![fact_node_id(&fact)],
                rationale: format!(
                    "fact `{}` is marked needs_review without a review item",
                    fact.slug
                ),
                expected_operation: format!("create review_item for fact {}", fact.id),
                validation_facts: vec!["review_state_missing_review_item".to_owned()],
                approval_required: false,
            });
        }
    }
    Ok(())
}

fn propose_validation_options(
    conn: &Connection,
    proposals: &mut Vec<GraphProposal>,
) -> anyhow::Result<()> {
    for fact in crate::db::list_facts(
        conn,
        &FactFilter {
            status: Some(crate::schema::FactStatus::Candidate),
            ..Default::default()
        },
    )? {
        let slug = validation_option_slug(&fact.slug);
        if crate::db::get_research_option_by_slug(conn, fact.program_id, &slug)?.is_none() {
            proposals.push(GraphProposal {
                id: format!("proposal:create_validation_option:fact:{}", fact.id),
                kind: ProposalKind::CreateValidationOption,
                affected_nodes: vec![fact_node_id(&fact)],
                rationale: format!("candidate fact `{}` needs a validation path", fact.slug),
                expected_operation: format!("create validation option `{slug}`"),
                validation_facts: vec!["candidate_fact_needs_validation".to_owned()],
                approval_required: false,
            });
        }
    }
    Ok(())
}

fn propose_superseded_options(
    conn: &Connection,
    proposals: &mut Vec<GraphProposal>,
) -> anyhow::Result<()> {
    for option in crate::db::list_research_options(
        conn,
        &ResearchOptionFilter {
            status: Some(ResearchOptionStatus::Open),
            ..Default::default()
        },
    )? {
        if terminal_experiment_exists_for_option(conn, &option)? {
            proposals.push(GraphProposal {
                id: format!("proposal:supersede_answered_option:{}", option.id),
                kind: ProposalKind::SupersedeAnsweredOption,
                affected_nodes: vec![option_node_id(&option)],
                rationale: format!(
                    "option `{}` already has a terminal linked experiment",
                    option.slug
                ),
                expected_operation: format!(
                    "update research_option {} status to superseded",
                    option.id
                ),
                validation_facts: vec!["open_option_has_terminal_experiment".to_owned()],
                approval_required: false,
            });
        }
    }
    Ok(())
}

fn apply_create_review_item(
    conn: &Connection,
    proposal: GraphProposal,
) -> anyhow::Result<ApplyResult> {
    let (entity_type, entity_id) = parse_review_item_proposal(&proposal)?;
    let reason = proposal.rationale.clone();
    let item = crate::db::create_review_item(
        conn,
        &NewReviewItem {
            entity_type,
            entity_id,
            reason: &reason,
            state: ReviewItemState::NeedsReview,
        },
    )?;
    Ok(ApplyResult {
        proposal_id: proposal.id,
        operation: proposal.expected_operation,
        changed_entity_type: "review_item".to_owned(),
        changed_entity_id: item.id,
    })
}

fn apply_create_validation_option(
    conn: &Connection,
    proposal: GraphProposal,
) -> anyhow::Result<ApplyResult> {
    let fact_id = parse_trailing_i64(&proposal.id)?;
    let fact = crate::db::get_fact_by_id(conn, fact_id)?;
    let slug = validation_option_slug(&fact.slug);
    if crate::db::get_research_option_by_slug(conn, fact.program_id, &slug)?.is_some() {
        bail!("validation option `{slug}` already exists; proposal is stale");
    }
    let description = format!(
        "Validate candidate fact `{}`: {}",
        fact.slug, fact.statement
    );
    let option = crate::db::create_research_option(
        conn,
        &NewResearchOption {
            program_id: fact.program_id,
            branch_id: fact.branch_id,
            open_question_id: None,
            source_experiment_id: fact.created_from_experiment_id,
            source_decision_id: fact.created_from_decision_id,
            slug: &slug,
            title: &slug,
            hypothesis: Some(&fact.statement),
            description: &description,
            classification: ResearchOptionClassification::Validation,
            status: ResearchOptionStatus::Open,
        },
    )?;
    Ok(ApplyResult {
        proposal_id: proposal.id,
        operation: proposal.expected_operation,
        changed_entity_type: "research_option".to_owned(),
        changed_entity_id: option.id,
    })
}

fn apply_supersede_option(
    conn: &Connection,
    proposal: GraphProposal,
) -> anyhow::Result<ApplyResult> {
    let option_id = parse_trailing_i64(&proposal.id)?;
    let option = crate::db::supersede_research_option(conn, option_id)?;
    Ok(ApplyResult {
        proposal_id: proposal.id,
        operation: proposal.expected_operation,
        changed_entity_type: "research_option".to_owned(),
        changed_entity_id: option.id,
    })
}

fn has_open_review_item(
    conn: &Connection,
    entity_type: &str,
    entity_id: i64,
) -> anyhow::Result<bool> {
    Ok(!crate::db::list_review_items(
        conn,
        &ReviewItemFilter {
            entity_type: Some(entity_type.to_owned()),
            entity_id: Some(entity_id),
            state: Some(ReviewItemState::NeedsReview),
        },
    )?
    .is_empty())
}

fn terminal_experiment_exists_for_option(
    conn: &Connection,
    option: &ResearchOption,
) -> anyhow::Result<bool> {
    let mut stmt = conn.prepare(
        "SELECT COUNT(*)
         FROM experiment
         WHERE option_id = ?1
           AND status IN ('completed', 'inconclusive', 'failed', 'superseded')",
    )?;
    let linked_count: i64 = stmt.query_row([option.id], |row| row.get(0))?;
    Ok(linked_count > 0)
}

fn graph_reasons_for_option(projection: &GraphProjection, option_id: &str) -> Vec<String> {
    let mut reasons = Vec::new();
    for edge in &projection.edges {
        if edge.from == option_id || edge.to == option_id {
            reasons.push(format!(
                "{} edge from {} to {}",
                edge.kind, edge.from, edge.to
            ));
        }
    }
    if reasons.is_empty() {
        reasons.push("option is open and has no blocking graph edges".to_owned());
    }
    reasons.truncate(6);
    reasons
}

fn classification_preference(policy: &Policy) -> Vec<String> {
    if policy.recommendation.prefer_classifications.is_empty() {
        vec![
            "main_path".to_owned(),
            "validation".to_owned(),
            "exploratory".to_owned(),
            "maintenance".to_owned(),
            "long_running".to_owned(),
        ]
    } else {
        policy.recommendation.prefer_classifications.clone()
    }
}

fn validation_pressure_preference(base_preference: &[String]) -> Vec<String> {
    let mut preference = vec![ResearchOptionClassification::Validation.as_str().to_owned()];
    for classification in base_preference {
        if !preference.iter().any(|value| value == classification) {
            preference.push(classification.clone());
        }
    }
    preference
}

fn has_unresolved_evidence_pressure(
    conn: &Connection,
    program_id: i64,
    branch_id: Option<i64>,
) -> anyhow::Result<bool> {
    for status in [FactStatus::Candidate, FactStatus::Contested] {
        let facts = crate::db::list_facts(
            conn,
            &FactFilter {
                program_id: Some(program_id),
                branch_id: None,
                status: Some(status),
                review_state: None,
            },
        )?;
        if facts.iter().any(|fact| {
            branch_id.is_none() || fact.branch_id.is_none() || fact.branch_id == branch_id
        }) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn preference_rank(preference: &[String], classification: &str) -> usize {
    preference
        .iter()
        .position(|value| value == classification)
        .unwrap_or(preference.len() + 1)
}

fn evidence_edge_kind(link: &EvidenceLink) -> EdgeKind {
    match link.relation.as_str() {
        "supports" => EdgeKind::Supports,
        "contradicts" => EdgeKind::Contradicts,
        "refines" => EdgeKind::Refines,
        "supersedes" => EdgeKind::Supersedes,
        _ => EdgeKind::LinkedTo,
    }
}

fn evidence_targets(link: &EvidenceLink) -> Vec<String> {
    let mut targets = Vec::new();
    if let Some(id) = link.experiment_id {
        targets.push(id_node("experiment", id));
    }
    if let Some(id) = link.run_id {
        targets.push(id_node("run", id));
    }
    if let Some(id) = link.metric_id {
        targets.push(id_node("metric", id));
    }
    if let Some(id) = link.artifact_id {
        targets.push(id_node("artifact", id));
    }
    if let Some(id) = link.decision_id {
        targets.push(id_node("decision", id));
    }
    if let Some(report_path) = link.report_path.as_deref() {
        let anchor = link.report_anchor.as_deref().unwrap_or("");
        targets.push(report_anchor_node_id(report_path, anchor));
    }
    targets
}

fn path_allowed(path: &str, allowed_roots: &[String]) -> bool {
    allowed_roots.is_empty()
        || allowed_roots
            .iter()
            .any(|root| relative_path_within_root(path, root))
}

fn relative_path_within_root(path: &str, root: &str) -> bool {
    let Some(path_components) = clean_relative_components(path) else {
        return false;
    };
    let Some(root_components) = clean_relative_components(root) else {
        return false;
    };
    !root_components.is_empty()
        && path_components.len() >= root_components.len()
        && path_components
            .iter()
            .zip(root_components.iter())
            .all(|(path_part, root_part)| path_part == root_part)
}

fn clean_relative_components(value: &str) -> Option<Vec<String>> {
    Path::new(value).components().try_fold(
        Vec::new(),
        |mut components, component| match component {
            Component::Normal(part) => {
                components.push(part.to_string_lossy().to_string());
                Some(components)
            }
            Component::CurDir => Some(components),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => None,
        },
    )
}

fn terminal_experiment_status(status: &str) -> bool {
    matches!(
        status,
        "completed" | "inconclusive" | "failed" | "superseded"
    )
}

fn program_node_id(program: &crate::schema::Program) -> String {
    id_node("program", program.id)
}

fn branch_node_id(branch: &crate::schema::Branch) -> String {
    id_node("branch", branch.id)
}

fn question_node_id(question: &crate::schema::OpenQuestion) -> String {
    id_node("open_question", question.id)
}

fn option_node_id(option: &ResearchOption) -> String {
    id_node("research_option", option.id)
}

fn experiment_node_id(experiment: &crate::schema::Experiment) -> String {
    id_node("experiment", experiment.id)
}

fn run_node_id(run: &crate::schema::Run) -> String {
    id_node("run", run.id)
}

fn metric_node_id(metric: &crate::schema::Metric) -> String {
    id_node("metric", metric.id)
}

fn artifact_node_id(artifact: &crate::schema::Artifact) -> String {
    id_node("artifact", artifact.id)
}

fn decision_node_id(decision: &crate::schema::Decision) -> String {
    id_node("decision", decision.id)
}

fn fact_node_id(fact: &crate::schema::Fact) -> String {
    id_node("fact", fact.id)
}

fn axiom_node_id(axiom: &crate::schema::Axiom) -> String {
    id_node("axiom", axiom.id)
}

fn review_node_id(review: &crate::schema::ReviewItem) -> String {
    id_node("review_item", review.id)
}

fn override_node_id(approval: &crate::schema::OverrideApproval) -> String {
    id_node("override_approval", approval.id)
}

fn bug_node_id(bug: &crate::schema::BugReport) -> String {
    id_node("bug_report", bug.id)
}

fn report_anchor_node_id(path: &str, anchor: &str) -> String {
    format!(
        "report_anchor:{}",
        stable_text_key(&format!("{path}#{anchor}"))
    )
}

fn id_node(kind: &str, id: i64) -> String {
    match kind {
        "program" => format!("program:{id}"),
        "branch" => format!("branch:{id}"),
        "open_question" => format!("question:{id}"),
        "research_option" => format!("option:{id}"),
        "experiment" => format!("experiment:{id}"),
        "run" => format!("run:{id}"),
        "metric" => format!("metric:{id}"),
        "artifact" => format!("artifact:{id}"),
        "decision" => format!("decision:{id}"),
        "fact" => format!("fact:{id}"),
        "axiom" => format!("axiom:{id}"),
        "review_item" => format!("review_item:{id}"),
        "override_approval" => format!("override_approval:{id}"),
        "bug_report" => format!("bug_report:{id}"),
        _ => format!("{kind}:{id}"),
    }
}

fn entity_node_id(entity_type: &str, entity_id: i64) -> String {
    id_node(entity_type, entity_id)
}

fn stable_text_key(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn validation_option_slug(fact_slug: &str) -> String {
    format!("validate-{fact_slug}")
}

fn map<const N: usize>(values: [(&str, &str); N]) -> BTreeMap<String, String> {
    values
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

fn error(
    check_id: &str,
    node_id: Option<String>,
    message: impl Into<String>,
    repair: impl Into<String>,
) -> ValidationFinding {
    ValidationFinding {
        check_id: check_id.to_owned(),
        severity: ValidationSeverity::Error,
        node_id,
        message: message.into(),
        repair: repair.into(),
    }
}

fn warning(
    check_id: &str,
    node_id: Option<String>,
    message: impl Into<String>,
    repair: impl Into<String>,
) -> ValidationFinding {
    ValidationFinding {
        check_id: check_id.to_owned(),
        severity: ValidationSeverity::Warning,
        node_id,
        message: message.into(),
        repair: repair.into(),
    }
}

fn parse_review_item_proposal(proposal: &GraphProposal) -> anyhow::Result<(&str, i64)> {
    let suffix = proposal
        .id
        .strip_prefix("proposal:create_review_item:")
        .context("proposal is not a review-item proposal")?;
    let mut parts = suffix.rsplitn(2, ':');
    let id = parts
        .next()
        .context("review-item proposal missing entity id")?
        .parse::<i64>()
        .context("review-item proposal has invalid entity id")?;
    let marker = parts
        .next()
        .context("review-item proposal missing entity marker")?;
    let entity_type = match marker {
        "blocked_option" => "research_option",
        other => other,
    };
    Ok((entity_type, id))
}

fn parse_trailing_i64(value: &str) -> anyhow::Result<i64> {
    value
        .rsplit(':')
        .next()
        .context("identifier has no trailing id")?
        .parse::<i64>()
        .with_context(|| format!("identifier `{value}` has invalid trailing id"))
}

fn push_list(output: &mut String, values: &[String]) {
    if values.is_empty() {
        output.push_str("- none\n");
    } else {
        for value in values {
            output.push_str(&format!("- {value}\n"));
        }
    }
}

fn push_indented_list(output: &mut String, values: &[String], indent: &str) {
    if values.is_empty() {
        output.push_str(&format!("{indent}- none\n"));
    } else {
        for value in values {
            output.push_str(&format!("{indent}- {value}\n"));
        }
    }
}
