use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{bail, Context};
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::policy::Policy;
use crate::schema::{
    FactStatus, NewExperiment, NewResearchOption, ResearchOptionClassification,
    ResearchOptionFilter, ResearchOptionStatus,
};

pub const BUNDLE_FORMAT: &str = "ldgr-research.hypothesis_candidates.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisBundle {
    pub format: String,
    pub created_at: String,
    pub research_goal: String,
    pub source: HypothesisSource,
    pub candidates: Vec<HypothesisCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisSource {
    pub mode: String,
    pub paper: String,
    pub branch: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisCandidate {
    pub id: String,
    pub hypothesis: String,
    pub rationale: String,
    pub expected_observation: String,
    pub falsification_test: String,
    pub required_artifacts: Vec<String>,
    pub branch_target: Option<String>,
    pub redundancy_risk: RedundancyRisk,
    pub assumptions_used: Vec<String>,
    #[serde(default)]
    pub critique: Option<HypothesisCritique>,
    #[serde(default)]
    pub score: Option<i64>,
    #[serde(default)]
    pub rank: Option<usize>,
    #[serde(default = "default_candidate_status")]
    pub status: String,
    #[serde(default)]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedundancyRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisCritique {
    pub alignment: ReviewGrade,
    pub plausibility: ReviewGrade,
    pub novelty: ReviewGrade,
    pub testability: ReviewGrade,
    pub safety: ReviewGrade,
    pub issues: Vec<String>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewGrade {
    Pass,
    Caution,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleFormat {
    Json,
    Yaml,
}

impl BundleFormat {
    pub fn from_path_or_name(path: Option<&Path>, format: Option<&str>) -> anyhow::Result<Self> {
        match format {
            Some("json") => Ok(Self::Json),
            Some("yaml") | Some("yml") => Ok(Self::Yaml),
            Some(value) => bail!("hypothesis format must be json or yaml, got `{value}`"),
            None => match path.and_then(Path::extension).and_then(|ext| ext.to_str()) {
                Some("yaml" | "yml") => Ok(Self::Yaml),
                _ => Ok(Self::Json),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateConfig {
    pub goal: String,
    pub count: usize,
    pub branch: Option<String>,
    pub include_graph_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptConfig {
    pub candidate_id: String,
    pub program: Option<String>,
    pub branch: Option<String>,
    pub classification: ResearchOptionClassification,
    pub create_experiment: bool,
    pub experiment_slug: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptResult {
    pub option_slug: String,
    pub option_id: i64,
    pub experiment_slug: Option<String>,
    pub experiment_id: Option<i64>,
}

pub fn generate(
    conn: &Connection,
    policy: &Policy,
    config: GenerateConfig,
) -> anyhow::Result<HypothesisBundle> {
    if config.goal.trim().is_empty() {
        bail!("hypothesis generate requires a non-empty --goal");
    }
    if config.count == 0 {
        bail!("hypothesis generate --count must be greater than zero");
    }
    let branch = config
        .branch
        .clone()
        .or_else(|| policy.current_branch.clone());
    let context = load_context(
        conn,
        policy,
        branch.as_deref(),
        config.include_graph_context,
    )?;
    let focus_terms = focus_terms(&config.goal, &context);
    let mut candidates = Vec::new();
    for index in 0..config.count.min(12) {
        candidates.push(candidate_from_focus(
            index,
            &config.goal,
            branch.as_deref(),
            &focus_terms,
            &context,
        ));
    }
    Ok(HypothesisBundle {
        format: BUNDLE_FORMAT.to_owned(),
        created_at: Utc::now().to_rfc3339(),
        research_goal: config.goal,
        source: HypothesisSource {
            mode: "deterministic_ldgr-research_context".to_owned(),
            paper: "docs/papers/google-co-scientist.pdf".to_owned(),
            branch,
            note: "Minimum viable Co-Scientist slice: generation, critique, ranking, evolution, and explicit accept only.".to_owned(),
        },
        candidates,
    })
}

pub fn critique(mut bundle: HypothesisBundle) -> HypothesisBundle {
    for candidate in &mut bundle.candidates {
        candidate.critique = Some(critique_candidate(candidate, &bundle.research_goal));
    }
    bundle
}

pub fn rank(mut bundle: HypothesisBundle) -> HypothesisBundle {
    for candidate in &mut bundle.candidates {
        if candidate.critique.is_none() {
            candidate.critique = Some(critique_candidate(candidate, &bundle.research_goal));
        }
        candidate.score = Some(score_candidate(candidate));
    }
    bundle.candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.id.cmp(&right.id))
    });
    for (index, candidate) in bundle.candidates.iter_mut().enumerate() {
        candidate.rank = Some(index + 1);
    }
    bundle
}

pub fn evolve(bundle: HypothesisBundle, candidate_id: &str) -> anyhow::Result<HypothesisBundle> {
    let parent = bundle
        .candidates
        .iter()
        .find(|candidate| candidate.id == candidate_id)
        .with_context(|| format!("candidate `{candidate_id}` not found"))?;
    let mut evolved = parent.clone();
    evolved.parent_id = Some(parent.id.clone());
    evolved.id = unique_candidate_id(&bundle, &format!("{}-evolved", parent.id));
    evolved.hypothesis = format!(
        "{} The evolved version narrows the claim to the smallest falsifiable ldgr-research experiment.",
        parent.hypothesis
    );
    evolved.rationale = format!(
        "{} Evolution strategy: simplify scope, make required artifacts explicit, and reduce ambiguous assumptions.",
        parent.rationale
    );
    evolved.falsification_test = format!(
        "{} Fail the evolved hypothesis if the required artifact is missing or the ranked observation is not reproducible.",
        parent.falsification_test
    );
    evolved.redundancy_risk = match parent.redundancy_risk {
        RedundancyRisk::High => RedundancyRisk::Medium,
        RedundancyRisk::Medium => RedundancyRisk::Low,
        RedundancyRisk::Low => RedundancyRisk::Low,
    };
    evolved.assumptions_used.push(
        "Evolution agent constraint: produce a new candidate, do not overwrite the parent."
            .to_owned(),
    );
    evolved.critique = None;
    evolved.score = None;
    evolved.rank = None;
    evolved.status = "evolved".to_owned();

    let mut next = bundle;
    next.candidates.push(evolved);
    Ok(next)
}

pub fn accept(
    conn: &Connection,
    policy: &Policy,
    bundle: &HypothesisBundle,
    config: AcceptConfig,
) -> anyhow::Result<AcceptResult> {
    let candidate = bundle
        .candidates
        .iter()
        .find(|candidate| candidate.id == config.candidate_id)
        .with_context(|| format!("candidate `{}` not found", config.candidate_id))?;
    let program_slug = config
        .program
        .as_deref()
        .or(policy.current_program.as_deref())
        .context("hypothesis accept requires --program or policy.current_program")?;
    let program = crate::db::get_program_by_slug(conn, program_slug)?
        .with_context(|| format!("program `{program_slug}` not found"))?;
    let branch_slug = config
        .branch
        .as_deref()
        .or(candidate.branch_target.as_deref())
        .or(policy.current_branch.as_deref());
    let branch = match branch_slug {
        Some(slug) => Some(
            crate::db::get_branch_by_slug(conn, program.id, slug)?.with_context(|| {
                format!("branch `{slug}` not found in program `{program_slug}`")
            })?,
        ),
        None => None,
    };

    let option_slug = sanitize_slug(&candidate.id);
    if crate::db::get_research_option_by_slug(conn, program.id, &option_slug)?.is_some() {
        bail!("research option `{option_slug}` already exists; refusing duplicate accept");
    }
    let description = candidate_description(candidate);
    let option = crate::db::create_research_option(
        conn,
        &NewResearchOption {
            program_id: program.id,
            branch_id: branch.as_ref().map(|branch| branch.id),
            open_question_id: None,
            source_experiment_id: None,
            source_decision_id: None,
            slug: &option_slug,
            title: &option_slug,
            hypothesis: Some(&candidate.hypothesis),
            description: &description,
            classification: config.classification,
            status: ResearchOptionStatus::Open,
        },
    )?;

    let mut experiment_slug = None;
    let mut experiment_id = None;
    if config.create_experiment {
        let branch = branch.context("hypothesis accept --create-experiment requires a branch")?;
        let slug = config
            .experiment_slug
            .unwrap_or_else(|| format!("{}-experiment", option_slug));
        if crate::db::get_experiment_by_slug(conn, branch.id, &slug)?.is_some() {
            bail!(
                "experiment `{slug}` already exists in branch `{}`",
                branch.slug
            );
        }
        let primary_metrics_json = serde_json::to_string(&vec!["hypothesis_rank_delta"])?;
        let experiment = crate::db::create_experiment(
            conn,
            &NewExperiment {
                branch_id: branch.id,
                option_id: Some(option.id),
                slug: &slug,
                title: &slug,
                phase: Some("hypothesis_engine"),
                mode: crate::schema::ExperimentMode::Falsification,
                hypothesis: Some(&candidate.hypothesis),
                setup: Some(&candidate.rationale),
                observation_goal: None,
                rationale: Some(&candidate.rationale),
                primary_metrics_json: &primary_metrics_json,
                secondary_metrics_json: "[]",
                pass_criteria: Some(&candidate.expected_observation),
                fail_criteria: Some(&candidate.falsification_test),
                allowed_next_steps: Some("[]"),
                blocked_next_steps: Some(
                    "[\"Treat generated hypothesis as accepted without evidence\"]",
                ),
                status: crate::schema::ExperimentStatus::Planned,
            },
        )?;
        experiment_slug = Some(experiment.slug);
        experiment_id = Some(experiment.id);
    }

    Ok(AcceptResult {
        option_slug: option.slug,
        option_id: option.id,
        experiment_slug,
        experiment_id,
    })
}

pub fn read_bundle(path: &Path) -> anyhow::Result<HypothesisBundle> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read hypothesis bundle {}", path.display()))?;
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("yaml" | "yml") => serde_yaml::from_str(&contents)
            .with_context(|| format!("failed to parse YAML hypothesis bundle {}", path.display())),
        _ => serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse JSON hypothesis bundle {}", path.display())),
    }
}

pub fn write_or_print(
    bundle: &HypothesisBundle,
    output: Option<&Path>,
    format: BundleFormat,
) -> anyhow::Result<()> {
    let contents = serialize_bundle(bundle, format)?;
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(path, contents)
            .with_context(|| format!("failed to write hypothesis bundle {}", path.display()))?;
    } else {
        print!("{contents}");
    }
    Ok(())
}

fn serialize_bundle(bundle: &HypothesisBundle, format: BundleFormat) -> anyhow::Result<String> {
    match format {
        BundleFormat::Json => Ok(format!("{}\n", serde_json::to_string_pretty(bundle)?)),
        BundleFormat::Yaml => Ok(serde_yaml::to_string(bundle)?),
    }
}

#[derive(Debug, Clone, Default)]
struct ContextSnapshot {
    questions: Vec<String>,
    options: Vec<String>,
    facts: Vec<String>,
    axioms: Vec<String>,
    unresolved_evidence_pressure: bool,
    evidence_pressure_option_slug: Option<String>,
    graph: Option<GraphContext>,
}

#[derive(Debug, Clone, Default)]
struct GraphContext {
    recommended_option_slug: Option<String>,
    recommendation_message: Option<String>,
    recommendation_reason: Option<String>,
    policy_reason: Option<String>,
    graph_reasons: Vec<String>,
    validation_warnings: Vec<String>,
}

fn load_context(
    conn: &Connection,
    policy: &Policy,
    branch_slug: Option<&str>,
    include_graph_context: bool,
) -> anyhow::Result<ContextSnapshot> {
    let Some(program_slug) = policy.current_program.as_deref() else {
        return Ok(ContextSnapshot::default());
    };
    let Some(program) = crate::db::get_program_by_slug(conn, program_slug)? else {
        return Ok(ContextSnapshot::default());
    };
    let branch_id = match branch_slug {
        Some(slug) => {
            crate::db::get_branch_by_slug(conn, program.id, slug)?.map(|branch| branch.id)
        }
        None => None,
    };
    let questions = crate::db::list_open_questions(
        conn,
        &crate::schema::OpenQuestionFilter {
            program_id: Some(program.id),
            branch_id,
            status: Some(crate::schema::OpenQuestionStatus::Open),
        },
    )?
    .into_iter()
    .map(|question| question.question)
    .collect();
    let unresolved_facts = unresolved_evidence_facts(conn, program.id, branch_id)?;
    let unresolved_evidence_pressure = !unresolved_facts.is_empty();
    let mut options = crate::db::list_research_options(
        conn,
        &ResearchOptionFilter {
            program_id: Some(program.id),
            branch_id,
            status: Some(ResearchOptionStatus::Open),
            classification: None,
        },
    )?;
    if unresolved_evidence_pressure {
        options.sort_by(|left, right| {
            let left_rank = evidence_pressure_option_rank(&left.classification);
            let right_rank = evidence_pressure_option_rank(&right.classification);
            left_rank
                .cmp(&right_rank)
                .then(left.created_at.cmp(&right.created_at))
                .then(left.id.cmp(&right.id))
        });
    }
    let evidence_pressure_option_slug = if unresolved_evidence_pressure {
        options
            .iter()
            .find(|option| {
                option.classification == ResearchOptionClassification::Validation.as_str()
            })
            .map(|option| option.slug.clone())
    } else {
        None
    };
    let options = options
        .into_iter()
        .map(|option| {
            format!(
                "{} [{}]: {}",
                option.slug, option.classification, option.description
            )
        })
        .collect();
    let mut facts = crate::db::list_facts(
        conn,
        &crate::schema::FactFilter {
            program_id: Some(program.id),
            branch_id,
            status: Some(crate::schema::FactStatus::Accepted),
            review_state: None,
        },
    )?
    .into_iter()
    .map(|fact| format!("accepted fact {}: {}", fact.slug, fact.statement))
    .collect::<Vec<_>>();
    facts.extend(unresolved_facts.into_iter().map(|fact| {
        format!(
            "{} fact {}: {}",
            fact.status.replace('_', " "),
            fact.slug,
            fact.statement
        )
    }));
    let axioms = crate::db::list_axioms(
        conn,
        &crate::schema::AxiomFilter {
            program_id: Some(program.id),
            branch_id: None,
            status: Some(crate::schema::AxiomStatus::Active),
            review_state: None,
        },
    )?
    .into_iter()
    .map(|axiom| axiom.statement)
    .collect();
    let graph = if include_graph_context {
        let mut graph_policy = policy.clone();
        if let Some(branch_slug) = branch_slug {
            graph_policy.current_branch = Some(branch_slug.to_owned());
        }
        let projection = crate::graph::build_projection(conn, &graph_policy)?;
        let next = crate::graph::recommend_next(
            conn,
            &graph_policy,
            &projection,
            &crate::graph::NextOptions::default(),
        )?;
        let validation_warnings =
            crate::graph::validate_projection(conn, &graph_policy, &projection)?
                .findings
                .into_iter()
                .filter(|finding| finding.severity == crate::graph::ValidationSeverity::Warning)
                .map(|finding| format!("{}: {}", finding.check_id, finding.message))
                .collect::<Vec<_>>();
        let recommendation = next.recommendation;
        Some(GraphContext {
            recommendation_message: Some(next.message),
            recommended_option_slug: recommendation.as_ref().map(|value| value.slug.clone()),
            recommendation_reason: recommendation.as_ref().map(|value| value.reason.clone()),
            policy_reason: recommendation
                .as_ref()
                .map(|value| value.policy_reason.clone()),
            graph_reasons: recommendation
                .as_ref()
                .map(|value| value.graph_reasons.clone())
                .unwrap_or_default(),
            validation_warnings,
        })
    } else {
        None
    };

    Ok(ContextSnapshot {
        questions,
        options,
        facts,
        axioms,
        unresolved_evidence_pressure,
        evidence_pressure_option_slug,
        graph,
    })
}

fn unresolved_evidence_facts(
    conn: &Connection,
    program_id: i64,
    branch_id: Option<i64>,
) -> anyhow::Result<Vec<crate::schema::Fact>> {
    let mut facts = Vec::new();
    for status in [FactStatus::Candidate, FactStatus::Contested] {
        facts.extend(
            crate::db::list_facts(
                conn,
                &crate::schema::FactFilter {
                    program_id: Some(program_id),
                    branch_id: None,
                    status: Some(status),
                    review_state: None,
                },
            )?
            .into_iter()
            .filter(|fact| {
                branch_id.is_none() || fact.branch_id.is_none() || fact.branch_id == branch_id
            }),
        );
    }
    Ok(facts)
}

fn evidence_pressure_option_rank(classification: &str) -> usize {
    match classification {
        "validation" => 0,
        "main_path" => 1,
        "exploratory" => 2,
        "maintenance" => 3,
        "long_running" => 4,
        _ => 5,
    }
}

fn focus_terms(goal: &str, context: &ContextSnapshot) -> Vec<String> {
    let mut terms = goal_terms(goal);
    if context.unresolved_evidence_pressure {
        terms.extend([
            "validation".to_owned(),
            "evidence".to_owned(),
            "audit".to_owned(),
        ]);
    }
    for value in context
        .questions
        .iter()
        .chain(context.options.iter())
        .chain(context.facts.iter())
        .chain(context.axioms.iter())
    {
        terms.extend(goal_terms(value));
    }
    if let Some(graph) = &context.graph {
        if let Some(message) = graph.recommendation_message.as_deref() {
            terms.extend(goal_terms(message));
        }
        if let Some(slug) = graph.recommended_option_slug.as_deref() {
            terms.push("graph".to_owned());
            terms.extend(goal_terms(slug));
        }
        if let Some(reason) = graph.recommendation_reason.as_deref() {
            terms.extend(goal_terms(reason));
        }
        if let Some(reason) = graph.policy_reason.as_deref() {
            terms.extend(goal_terms(reason));
        }
        for value in graph
            .graph_reasons
            .iter()
            .chain(graph.validation_warnings.iter())
        {
            terms.extend(goal_terms(value));
        }
    }
    let mut seen = BTreeSet::new();
    terms
        .into_iter()
        .filter(|term| seen.insert(term.clone()))
        .take(12)
        .collect()
}

fn candidate_from_focus(
    index: usize,
    goal: &str,
    branch: Option<&str>,
    focus_terms: &[String],
    context: &ContextSnapshot,
) -> HypothesisCandidate {
    let graph_context_active = context.graph.is_some() && index == 0;
    let graph_seed = context
        .graph
        .as_ref()
        .and_then(|graph| graph.recommended_option_slug.as_ref())
        .map(|slug| sanitize_slug(&format!("graph-{slug}")))
        .or_else(|| graph_context_active.then(|| "graph-null-state".to_owned()));
    let graph_context_candidate = graph_context_active;
    let focus = if graph_context_candidate {
        graph_seed.clone().unwrap_or_else(|| {
            focus_terms
                .get(index % focus_terms.len().max(1))
                .cloned()
                .unwrap_or_else(|| "ldgr-research".to_owned())
        })
    } else if index == 0 {
        context
            .evidence_pressure_option_slug
            .clone()
            .unwrap_or_else(|| {
                focus_terms
                    .get(index % focus_terms.len().max(1))
                    .cloned()
                    .unwrap_or_else(|| "ldgr-research".to_owned())
            })
    } else {
        focus_terms
            .get(index % focus_terms.len().max(1))
            .cloned()
            .unwrap_or_else(|| "ldgr-research".to_owned())
    };
    let strategy = if graph_context_candidate {
        "integration"
    } else {
        match index % 3 {
            0 => "generation",
            1 => "critique",
            _ => "ranking",
        }
    };
    let id = sanitize_slug(&format!("hyp-{strategy}-{focus}"));
    let evidence_anchor = context
        .questions
        .first()
        .or_else(|| context.options.first())
        .map(String::as_str)
        .unwrap_or(goal);
    let graph_note = if graph_context_candidate {
        context.graph.as_ref().map(graph_note).unwrap_or_default()
    } else {
        String::new()
    };
    let mut required_artifacts = vec![
        "hypothesis-bundle.json".to_owned(),
        "critique-and-ranking.json".to_owned(),
        "accepted-option-or-rejection-note.md".to_owned(),
    ];
    if graph_context_candidate {
        required_artifacts.push("graph-next.json".to_owned());
    }
    let mut assumptions_used = vec![
        "Scientist-in-the-loop acceptance is required before ledger mutation.".to_owned(),
        "Generated hypotheses are candidate options until accepted.".to_owned(),
        "Evaluation target is next-experiment selection quality, not full Co-Scientist parity."
            .to_owned(),
    ];
    if context.unresolved_evidence_pressure {
        assumptions_used.push(
            "Candidate or contested facts should be validated before main-path scaling work."
                .to_owned(),
        );
    }
    if graph_context_candidate {
        if let Some(graph) = &context.graph {
            if let Some(slug) = graph.recommended_option_slug.as_deref() {
                assumptions_used.push(format!(
                    "Combined mode should account for graph recommendation `{slug}` before proposing the next experiment."
                ));
            } else if let Some(message) = graph.recommendation_message.as_deref() {
                assumptions_used.push(format!(
                    "Combined mode should preserve the graph null-state (`{message}`) before proposing fallback work."
                ));
            }
            if !graph.validation_warnings.is_empty() {
                assumptions_used.push(format!(
                    "Combined mode should preserve {} graph validation warning(s) as planning context.",
                    graph.validation_warnings.len()
                ));
            }
        }
    }
    let expected_observation = if graph_context_candidate {
        "Combined-mode ranked candidates should preserve graph recommendation rationale and validation warnings while producing a more specific falsification experiment than baseline context-only selection.".to_owned()
    } else if let Some(slug) = context.evidence_pressure_option_slug.as_deref() {
        format!(
            "Ranked candidates should identify `{slug}` as the next existing ldgr-research option before main-path scaling while unresolved evidence quality remains."
        )
    } else {
        "Ranked candidate options should produce at least one more specific falsification experiment than baseline context-only selection.".to_owned()
    };
    let falsification_test = if graph_context_candidate {
        "Reject if combined mode cannot identify a concrete experiment, required artifact, or falsifiable observation that incorporates graph recommendation context beyond the hypothesis-only path.".to_owned()
    } else if let Some(slug) = context.evidence_pressure_option_slug.as_deref() {
        format!(
            "Reject if critique/ranking prefers main-path scaling over existing validation option `{slug}` while candidate or contested facts remain unresolved."
        )
    } else {
        "Reject if critique/ranking cannot identify a concrete experiment, required artifact, or falsifiable observation beyond the baseline ldgr-research option.".to_owned()
    };
    HypothesisCandidate {
        id,
        hypothesis: format!(
            "A constrained {strategy} loop focused on {focus} will improve ldgr-research next-experiment selection for: {goal}."
        ),
        rationale: format!(
            "The Co-Scientist paper decomposes hypothesis work into generation, reflection, ranking, evolution, and meta-review; this candidate isolates {strategy} against existing ldgr-research context: {evidence_anchor}.{graph_note}"
        ),
        expected_observation,
        falsification_test,
        required_artifacts,
        branch_target: branch.map(str::to_owned),
        redundancy_risk: if context.options.iter().any(|option| option.contains(&focus)) {
            RedundancyRisk::High
        } else if context.questions.iter().any(|question| question.contains(&focus)) {
            RedundancyRisk::Medium
        } else {
            RedundancyRisk::Low
        },
        assumptions_used,
        critique: None,
        score: None,
        rank: None,
        status: "candidate".to_owned(),
        parent_id: None,
    }
}

fn critique_candidate(candidate: &HypothesisCandidate, goal: &str) -> HypothesisCritique {
    let mut issues = Vec::new();
    let alignment = if contains_any(&candidate.hypothesis, &goal_terms(goal)) {
        ReviewGrade::Pass
    } else {
        issues.push("Hypothesis does not clearly reuse the research goal terms.".to_owned());
        ReviewGrade::Caution
    };
    let plausibility = if candidate.assumptions_used.is_empty() {
        issues.push("No assumptions are recorded.".to_owned());
        ReviewGrade::Fail
    } else {
        ReviewGrade::Pass
    };
    let novelty = match candidate.redundancy_risk {
        RedundancyRisk::High => {
            issues.push("High redundancy risk against existing ldgr-research context.".to_owned());
            ReviewGrade::Caution
        }
        _ => ReviewGrade::Pass,
    };
    let testability = if candidate.falsification_test.trim().is_empty()
        || candidate.required_artifacts.is_empty()
    {
        issues.push("Falsification test or required artifacts are missing.".to_owned());
        ReviewGrade::Fail
    } else {
        ReviewGrade::Pass
    };
    let safety = ReviewGrade::Pass;
    let recommendation = if issues.iter().any(|issue| issue.contains("missing")) {
        "revise before ranking".to_owned()
    } else if candidate.redundancy_risk == RedundancyRisk::High {
        "rank below lower-redundancy candidates unless evidence fit is strong".to_owned()
    } else {
        "rankable".to_owned()
    };

    HypothesisCritique {
        alignment,
        plausibility,
        novelty,
        testability,
        safety,
        issues,
        recommendation,
    }
}

fn score_candidate(candidate: &HypothesisCandidate) -> i64 {
    let mut score = 1200;
    if let Some(critique) = &candidate.critique {
        for grade in [
            critique.alignment,
            critique.plausibility,
            critique.novelty,
            critique.testability,
            critique.safety,
        ] {
            score += match grade {
                ReviewGrade::Pass => 40,
                ReviewGrade::Caution => 0,
                ReviewGrade::Fail => -120,
            };
        }
        score -= (critique.issues.len() as i64) * 20;
    }
    score += match candidate.redundancy_risk {
        RedundancyRisk::Low => 80,
        RedundancyRisk::Medium => 20,
        RedundancyRisk::High => -80,
    };
    score += (candidate.required_artifacts.len() as i64).min(4) * 10;
    score
}

fn candidate_description(candidate: &HypothesisCandidate) -> String {
    let mut fields = BTreeMap::new();
    fields.insert("rationale", candidate.rationale.clone());
    fields.insert(
        "expected_observation",
        candidate.expected_observation.clone(),
    );
    fields.insert("falsification_test", candidate.falsification_test.clone());
    fields.insert(
        "required_artifacts",
        candidate.required_artifacts.join(", "),
    );
    fields.insert(
        "redundancy_risk",
        serde_json::to_string(&candidate.redundancy_risk).unwrap_or_else(|_| "unknown".to_owned()),
    );
    fields.insert("assumptions_used", candidate.assumptions_used.join("; "));
    fields
        .into_iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn goal_terms(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|term| term.len() >= 4)
        .filter(|term| {
            !matches!(
                term.as_str(),
                "that" | "this" | "with" | "from" | "into" | "research" | "hypothesis"
            )
        })
        .collect()
}

fn contains_any(value: &str, terms: &[String]) -> bool {
    let lower = value.to_ascii_lowercase();
    terms.iter().any(|term| lower.contains(term))
}

fn sanitize_slug(value: &str) -> String {
    let slug = value
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
        .join("-");
    if slug.is_empty() {
        "hypothesis-candidate".to_owned()
    } else {
        slug
    }
}

fn unique_candidate_id(bundle: &HypothesisBundle, base: &str) -> String {
    let existing = bundle
        .candidates
        .iter()
        .map(|candidate| candidate.id.as_str())
        .collect::<BTreeSet<_>>();
    if !existing.contains(base) {
        return base.to_owned();
    }
    for index in 2.. {
        let candidate = format!("{base}-{index}");
        if !existing.contains(candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!("unbounded id suffix search should always return")
}

fn default_candidate_status() -> String {
    "candidate".to_owned()
}

fn graph_note(graph: &GraphContext) -> String {
    let mut parts = Vec::new();
    if let Some(slug) = graph.recommended_option_slug.as_deref() {
        parts.push(format!(" Graph recommendation: {slug}."));
    } else if let Some(message) = graph.recommendation_message.as_deref() {
        parts.push(format!(" Graph recommendation: none ({message})."));
    }
    if let Some(reason) = graph.recommendation_reason.as_deref() {
        parts.push(format!(" Reason: {reason}."));
    }
    if !graph.graph_reasons.is_empty() {
        parts.push(format!(
            " Graph evidence: {}.",
            graph.graph_reasons.join("; ")
        ));
    }
    if !graph.validation_warnings.is_empty() {
        parts.push(format!(
            " Validation warnings: {}.",
            graph.validation_warnings.join("; ")
        ));
    }
    parts.concat()
}
