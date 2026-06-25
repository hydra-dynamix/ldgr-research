#![allow(clippy::large_enum_variant)]

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{bail, Context};
use clap::{Args, Parser, Subcommand, ValueEnum};
use sha2::{Digest, Sha256};

const DEFAULT_DB: &str = ".ldgr/research/research.db";
const DEFAULT_POLICY: &str = ".ldgr/research/policy.yaml";
const DEFAULT_TOOLS: &str = ".ldgr/research/tools.yaml";

#[derive(Debug, Parser)]
#[command(name = "ldgr-research")]
#[command(about = "LDGR research adapter and local-first research ledger CLI.")]
#[command(version)]
pub struct Cli {
    /// Path to the local research SQLite database.
    #[arg(long, global = true, default_value = DEFAULT_DB)]
    pub db: PathBuf,

    /// Path to the project research policy file.
    #[arg(long, global = true, default_value = DEFAULT_POLICY)]
    pub policy: PathBuf,

    /// Path to the project reusable research tool registry.
    #[arg(long, global = true, default_value = DEFAULT_TOOLS)]
    pub tools: PathBuf,

    /// Enable the derived graph reasoning membrane for this invocation.
    #[arg(long, global = true)]
    pub enable_graph_reasoning: bool,

    /// Enable the hypothesis engine for this invocation.
    #[arg(long, global = true)]
    pub enable_hypothesis_engine: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize research ledger storage for the current project.
    Init,
    /// Print the active research cockpit.
    Context,
    /// Manage research programs.
    Program(ProgramArgs),
    /// Manage research branches.
    Branch(BranchArgs),
    /// Manage experiments.
    Experiment(ExperimentArgs),
    /// Manage experiment runs.
    Run(RunArgs),
    /// Manage run metrics.
    Metric(MetricArgs),
    /// Manage run artifacts.
    Artifact(ArtifactArgs),
    /// Record structured experiment decisions.
    Decision(DecisionArgs),
    /// Manage open research questions.
    Question(QuestionArgs),
    /// Manage selectable research options.
    Option(OptionArgs),
    /// Manage evaluation matrices.
    Matrix(MatrixArgs),
    /// Manage evidence-backed facts.
    Fact(FactArgs),
    /// Manage manually governed axioms.
    Axiom(AxiomArgs),
    /// Manage human review items.
    Review(ReviewArgs),
    /// Manage blocked-work override approvals.
    Override(OverrideArgs),
    /// Report and track bugs encountered by agents or humans.
    Bug(BugArgs),
    /// Inspect reusable research tool registry entries.
    Tool(ToolArgs),
    /// Inspect and act on the derived research graph membrane.
    Graph(GraphArgs),
    /// Generate local research dashboard artifacts.
    Dashboard(DashboardArgs),
    /// Generate, critique, rank, evolve, and explicitly accept candidate hypotheses.
    Hypothesis(HypothesisArgs),
    /// Print compact research status.
    Status,
    /// Print the research tree.
    Tree(TreeArgs),
    /// Show a typed research entity.
    Show(ShowArgs),
    /// Generate reports.
    Report(ReportArgs),
    /// Export research data.
    Export(ExportArgs),
    /// Import research data.
    Import(ImportArgs),
    /// Run guard checks.
    Guard,
    /// Lint research ledger state.
    Lint,
    /// Run database migrations.
    Migrate,
    /// Diagnose local research ledger setup.
    Doctor,
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => init_project(&cli.db, &cli.policy, &cli.tools),
        Command::Context => handle_context(&cli.db, &cli.policy, cli.enable_graph_reasoning),
        Command::Program(args) => handle_program(&cli.db, &cli.policy, args),
        Command::Branch(args) => handle_branch(&cli.db, &cli.policy, args),
        Command::Experiment(args) => handle_experiment(&cli.db, &cli.policy, args),
        Command::Run(args) => handle_run(&cli.db, &cli.policy, args),
        Command::Metric(args) => handle_metric(&cli.db, &cli.policy, args),
        Command::Artifact(args) => handle_artifact(&cli.db, &cli.policy, args),
        Command::Decision(args) => handle_decision(&cli.db, &cli.policy, args),
        Command::Question(args) => handle_question(&cli.db, &cli.policy, args),
        Command::Option(args) => handle_option(&cli.db, &cli.policy, args),
        Command::Matrix(args) => handle_matrix(&cli.db, &cli.policy, args),
        Command::Fact(args) => handle_fact(&cli.db, &cli.policy, args),
        Command::Axiom(args) => handle_axiom(&cli.db, &cli.policy, args),
        Command::Review(args) => handle_review(&cli.db, args),
        Command::Override(args) => handle_override(&cli.db, args),
        Command::Bug(args) => handle_bug(&cli.db, &cli.policy, args),
        Command::Tool(args) => handle_tool(&cli.tools, args),
        Command::Graph(args) => {
            ensure_graph_reasoning_enabled(cli.enable_graph_reasoning)?;
            handle_graph(&cli.db, &cli.policy, args)
        }
        Command::Dashboard(args) => handle_dashboard(&cli.db, args),
        Command::Hypothesis(args) => {
            ensure_hypothesis_engine_enabled(cli.enable_hypothesis_engine)?;
            handle_hypothesis(&cli.db, &cli.policy, cli.enable_graph_reasoning, args)
        }
        Command::Status => handle_status(&cli.db),
        Command::Tree(args) => handle_tree(&cli.db, args),
        Command::Show(args) => handle_show(&cli.db, &cli.policy, args),
        Command::Report(args) => handle_report(&cli.db, &cli.policy, args),
        Command::Export(args) => handle_export(&cli.db, args),
        Command::Import(_) => bail!("import is not implemented yet"),
        Command::Guard => handle_guard(&cli.db, &cli.policy, true),
        Command::Lint => handle_guard(&cli.db, &cli.policy, false),
        Command::Migrate => handle_migrate(&cli.db),
        Command::Doctor => handle_doctor(&cli.db, &cli.policy),
    }
}

fn init_project(db: &Path, policy: &Path, tools: &Path) -> anyhow::Result<()> {
    let result = crate::db::init_project(db, policy)?;
    let tools_written = crate::tools::write_starter_registry_if_missing(tools)?;
    println!("initialized research store");
    println!("database: {}", result.db_path.display());
    println!("policy: {}", result.policy_path.display());
    println!("tools: {}", tools.display());
    if result.policy_written {
        println!("created starter policy");
    }
    if tools_written {
        println!("created starter tool registry");
    }
    if !result.applied_migrations.is_empty() {
        let versions = result
            .applied_migrations
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        println!("applied migrations: {versions}");
    }
    Ok(())
}

fn handle_program(db: &Path, policy: &Path, args: ProgramArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        ProgramCommand::Create(args) => {
            let status = parse_enum::<crate::schema::ProgramStatus>(
                args.status.as_deref().unwrap_or("active"),
                "program status",
            )?;
            let program = crate::db::create_program(
                &conn,
                &crate::schema::NewProgram {
                    slug: &args.slug,
                    title: &args.title,
                    objective: &args.objective,
                    status,
                },
            )?;
            println!("created program {}", program.slug);
        }
        ProgramCommand::List => {
            let programs = crate::db::list_programs(&conn)?;
            if programs.is_empty() {
                println!("No programs.");
            } else {
                for program in programs {
                    println!("{} [{}] {}", program.slug, program.status, program.title);
                }
            }
        }
        ProgramCommand::Show(args) => {
            let program = require_program(&conn, &args.slug)?;
            println!("Program: {}", program.slug);
            println!("id: {}", program.id);
            println!("title: {}", program.title);
            println!("objective: {}", program.objective);
            println!("status: {}", program.status);
            println!("created_at: {}", program.created_at);
            println!("updated_at: {}", program.updated_at);
        }
        ProgramCommand::SetCurrent(args) => {
            let program = require_program(&conn, &args.slug)?;
            crate::policy::set_current_program(policy, &program.slug)?;
            println!("current program: {}", program.slug);
        }
        ProgramCommand::Update(args) => {
            let program = require_program(&conn, &args.slug)?;
            if let Some(status) = args.status {
                let status = parse_enum::<crate::schema::ProgramStatus>(&status, "program status")?;
                let program = crate::db::update_program_status(&conn, program.id, status)?;
                println!("updated program {} [{}]", program.slug, program.status);
            } else {
                println!("no program updates requested");
            }
        }
    }
    Ok(())
}

fn handle_branch(db: &Path, policy: &Path, args: BranchArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        BranchCommand::Create(args) => {
            let program = require_program(&conn, &args.program)?;
            let parent_branch_id = match args.parent {
                Some(parent_slug) => {
                    let parent = require_branch(&conn, program.id, &parent_slug)?;
                    Some(parent.id)
                }
                None => None,
            };
            let status = parse_enum::<crate::schema::BranchStatus>(
                args.status.as_deref().unwrap_or("active"),
                "branch status",
            )?;
            let title = args.title.unwrap_or_else(|| args.slug.clone());
            let rationale = args.rationale.unwrap_or_default();
            let branch = crate::db::create_branch(
                &conn,
                &crate::schema::NewBranch {
                    program_id: program.id,
                    parent_branch_id,
                    slug: &args.slug,
                    title: &title,
                    question: &args.question,
                    rationale: &rationale,
                    status,
                },
            )?;
            println!("created branch {}", branch.slug);
        }
        BranchCommand::List => {
            let policy_doc = crate::policy::load_policy(policy)?;
            let program_slug = policy_doc.current_program.as_deref().context(
                "no current program set; run `ldgr-research program set-current <slug>`",
            )?;
            let program = require_program(&conn, program_slug)?;
            let branches = crate::db::list_branches(&conn, program.id)?;
            if branches.is_empty() {
                println!("No branches for program {}.", program.slug);
            } else {
                for branch in branches {
                    println!("{} [{}] {}", branch.slug, branch.status, branch.title);
                }
            }
        }
        BranchCommand::Show(args) => {
            let (program, branch) =
                resolve_current_program_branch(&conn, policy, Some(&args.slug))?;
            println!("Branch: {}", branch.slug);
            println!("id: {}", branch.id);
            println!("program: {}", program.slug);
            if let Some(parent_id) = branch.parent_branch_id {
                println!("parent_branch_id: {parent_id}");
            }
            println!("title: {}", branch.title);
            println!("question: {}", branch.question);
            println!("rationale: {}", branch.rationale);
            println!("status: {}", branch.status);
            println!("created_at: {}", branch.created_at);
            println!("updated_at: {}", branch.updated_at);
        }
        BranchCommand::SetCurrent(args) => {
            let (_program, branch) =
                resolve_current_program_branch(&conn, policy, Some(&args.slug))?;
            crate::policy::set_current_branch(policy, &branch.slug)?;
            println!("current branch: {}", branch.slug);
        }
        BranchCommand::Update(args) => {
            let (_program, branch) =
                resolve_current_program_branch(&conn, policy, Some(&args.slug))?;
            if let Some(status) = args.status {
                let status = parse_enum::<crate::schema::BranchStatus>(&status, "branch status")?;
                let branch = crate::db::update_branch_status(&conn, branch.id, status)?;
                println!("updated branch {} [{}]", branch.slug, branch.status);
            } else {
                println!("no branch updates requested");
            }
        }
    }
    Ok(())
}

fn ensure_graph_reasoning_enabled(enabled: bool) -> anyhow::Result<()> {
    if !enabled {
        bail!("graph reasoning is disabled; rerun with --enable-graph-reasoning");
    }
    Ok(())
}

fn ensure_hypothesis_engine_enabled(enabled: bool) -> anyhow::Result<()> {
    if !enabled {
        bail!("hypothesis engine is disabled; rerun with --enable-hypothesis-engine");
    }
    Ok(())
}

fn handle_context(db: &Path, policy: &Path, enable_graph_reasoning: bool) -> anyhow::Result<()> {
    let policy_doc = crate::policy::load_policy(policy)?;
    let conn = crate::db::open_database(db)?;

    println!("Current Program:");
    let current_program = match policy_doc.current_program.as_deref() {
        Some(slug) => match crate::db::get_program_by_slug(&conn, slug)? {
            Some(program) => {
                println!("{} [{}]", program.title, program.status);
                println!("slug: {}", program.slug);
                println!("objective: {}", program.objective);
                Some(program)
            }
            None => {
                println!("{slug} (missing in database)");
                None
            }
        },
        None => {
            println!("not set");
            println!(
                "warning: run `ldgr-research program set-current <slug>` after creating a program"
            );
            None
        }
    };

    println!();
    println!("Current Branch:");
    let current_branch = match (&current_program, policy_doc.current_branch.as_deref()) {
        (Some(program), Some(slug)) => {
            match crate::db::get_branch_by_slug(&conn, program.id, slug)? {
                Some(branch) => {
                    println!("{} [{}]", branch.title, branch.status);
                    println!("slug: {}", branch.slug);
                    println!("question: {}", branch.question);
                    Some(branch)
                }
                None => {
                    println!("{slug} (missing in current program)");
                    None
                }
            }
        }
        (_, Some(slug)) => {
            println!("{slug} (program unresolved)");
            None
        }
        (_, None) => {
            println!("not set");
            println!(
                "warning: run `ldgr-research branch set-current <slug>` after creating a branch"
            );
            None
        }
    };

    println!();
    println!("Research Options:");
    if let Some(program) = current_program.as_ref() {
        let branch_id = current_branch.as_ref().map(|branch| branch.id);
        let recommendations = crate::db::list_recommended_research_options(
            &conn,
            &crate::schema::ResearchOptionFilter {
                program_id: Some(program.id),
                branch_id,
                status: None,
                classification: None,
            },
        )?;
        if let Some(option) = recommendations.first() {
            println!("Recommended Next Option:");
            println!("- {} [{}]", option.slug, option.classification);
            println!("  why: unblocked open option matching the preferred classification order");
        } else {
            println!("Recommended Next Option:");
            println!("none");
        }

        let options = crate::db::list_research_options(
            &conn,
            &crate::schema::ResearchOptionFilter {
                program_id: Some(program.id),
                branch_id,
                status: Some(crate::schema::ResearchOptionStatus::Open),
                classification: None,
            },
        )?;
        if options.is_empty() {
            println!("Open Options: none");
        } else {
            println!("Open Options:");
            for option in options {
                println!(
                    "- {} [{}, {}]",
                    option.slug, option.status, option.classification
                );
            }
        }
    } else {
        println!("none recorded yet");
    }

    println!();
    println!("Open Questions:");
    if let Some(program) = current_program.as_ref() {
        let questions = crate::db::list_open_questions(
            &conn,
            &crate::schema::OpenQuestionFilter {
                program_id: Some(program.id),
                branch_id: current_branch.as_ref().map(|branch| branch.id),
                status: Some(crate::schema::OpenQuestionStatus::Open),
            },
        )?;
        if questions.is_empty() {
            println!("none");
        } else {
            for question in questions {
                println!(
                    "- {} [{}] {}",
                    question.slug, question.status, question.question
                );
            }
        }
    } else {
        println!("none recorded yet");
    }

    println!();
    println!("Hard Facts:");
    if let Some(program) = current_program.as_ref() {
        let facts = crate::db::list_facts(
            &conn,
            &crate::schema::FactFilter {
                program_id: Some(program.id),
                branch_id: current_branch.as_ref().map(|branch| branch.id),
                status: Some(crate::schema::FactStatus::Accepted),
                review_state: None,
            },
        )?;
        if facts.is_empty() {
            println!("none");
        } else {
            for fact in facts {
                let links = crate::db::list_evidence_links(&conn, "fact", fact.id)?;
                println!(
                    "- {} [{}] {} (evidence: {})",
                    fact.slug,
                    fact.status,
                    fact.statement,
                    links.len()
                );
            }
        }
    } else {
        println!("none recorded yet");
    }

    println!();
    println!("Axioms:");
    if let Some(program) = current_program.as_ref() {
        let axioms = crate::db::list_axioms(
            &conn,
            &crate::schema::AxiomFilter {
                program_id: Some(program.id),
                branch_id: None,
                status: Some(crate::schema::AxiomStatus::Active),
                review_state: None,
            },
        )?;
        if axioms.is_empty() {
            println!("none");
        } else {
            for axiom in axioms {
                println!("- {} [{}] {}", axiom.slug, axiom.status, axiom.statement);
            }
        }
    } else {
        println!("none recorded yet");
    }

    println!();
    println!("Active Experiments:");
    if let Some(branch) = current_branch {
        let experiments = crate::db::list_experiments(&conn, branch.id)?;
        let active = experiments
            .iter()
            .filter(|experiment| experiment.status == "planned" || experiment.status == "running")
            .collect::<Vec<_>>();
        if active.is_empty() {
            println!("none");
        } else {
            for experiment in active {
                println!(
                    "{} [{}] {}",
                    experiment.slug, experiment.status, experiment.title
                );
            }
        }
    } else {
        println!("none");
    }

    println!();
    println!("Allowed Work:");
    print_list_or_none(&policy_doc.allowed_work);

    println!();
    println!("Blocked Work:");
    print_list_or_none(&policy_doc.blocked_work);

    println!();
    println!("Attention Needed:");
    let mut wrote_attention = false;
    if current_program.is_none() || policy_doc.current_branch.is_none() {
        println!("- active program or branch is not fully set");
        wrote_attention = true;
    }
    let review_items = crate::db::list_review_items(
        &conn,
        &crate::schema::ReviewItemFilter {
            state: Some(crate::schema::ReviewItemState::NeedsReview),
            ..Default::default()
        },
    )?;
    for review in review_items {
        println!(
            "- review {} {}: {}",
            review.entity_type, review.entity_id, review.reason
        );
        wrote_attention = true;
    }
    let bug_reports = crate::db::list_bug_reports(
        &conn,
        &crate::schema::BugReportFilter {
            program_id: current_program.as_ref().map(|program| program.id),
            status: Some(crate::schema::BugReportStatus::Open),
            ..Default::default()
        },
    )?;
    for bug in bug_reports {
        println!("- bug {} [{}] {}", bug.id, bug.severity, bug.title);
        wrote_attention = true;
    }
    if !wrote_attention {
        println!("none");
    }

    if enable_graph_reasoning {
        println!();
        println!("Graph Membrane:");
        let projection = crate::graph::build_projection(&conn, &policy_doc)?;
        let summary = crate::graph::summarize_projection(&projection);
        let validation = crate::graph::validate_projection(&conn, &policy_doc, &projection)?;
        println!(
            "projection: {} nodes, {} edges, {} obligations",
            summary.node_count, summary.edge_count, summary.obligation_count
        );
        println!("validation: {}", validation.status);
        let next = crate::graph::recommend_next(
            &conn,
            &policy_doc,
            &projection,
            &crate::graph::NextOptions::default(),
        )?;
        if let Some(recommendation) = next.recommendation {
            println!(
                "next: {} [{}]",
                recommendation.slug, recommendation.classification
            );
        } else {
            println!("next: none");
        }
    }

    Ok(())
}

fn handle_graph(db: &Path, policy: &Path, args: GraphArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    let policy_doc = crate::policy::load_policy(policy)?;
    let projection = crate::graph::build_projection(&conn, &policy_doc)?;

    match args.command {
        GraphCommand::Build(args) => {
            if args.json {
                println!("{}", serde_json::to_string_pretty(&projection)?);
            } else {
                print!("{}", crate::graph::format_summary(&projection));
            }
        }
        GraphCommand::Show(args) => {
            if args.json {
                println!("{}", serde_json::to_string_pretty(&projection)?);
            } else {
                print!("{}", crate::graph::format_show(&projection));
            }
        }
        GraphCommand::Validate(args) => {
            let report = crate::graph::validate_projection(&conn, &policy_doc, &projection)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", crate::graph::format_validation(&report));
            }
            if crate::graph::has_validation_errors(&report) {
                bail!("graph validation failed");
            }
        }
        GraphCommand::Next(args) => {
            let report = crate::graph::recommend_next(
                &conn,
                &policy_doc,
                &projection,
                &crate::graph::NextOptions {
                    include_long_running: args.include_long_running,
                },
            )?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", crate::graph::format_next(&report));
            }
        }
        GraphCommand::Propose(args) => {
            let proposals = crate::graph::propose(&conn, &policy_doc, &projection)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&proposals)?);
            } else {
                print!("{}", crate::graph::format_proposals(&proposals));
            }
        }
        GraphCommand::Apply(args) => {
            let result = crate::graph::apply_proposal(&conn, &policy_doc, &args.proposal_id)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("applied proposal {}", result.proposal_id);
                println!("operation: {}", result.operation);
                println!(
                    "changed: {} {}",
                    result.changed_entity_type, result.changed_entity_id
                );
            }
        }
    }

    Ok(())
}

fn handle_hypothesis(
    db: &Path,
    policy: &Path,
    enable_graph_reasoning: bool,
    args: HypothesisArgs,
) -> anyhow::Result<()> {
    match args.command {
        HypothesisCommand::Generate(args) => {
            let conn = crate::db::open_database(db)?;
            let policy_doc = crate::policy::load_policy(policy)?;
            let bundle = crate::hypothesis::generate(
                &conn,
                &policy_doc,
                crate::hypothesis::GenerateConfig {
                    goal: args.goal,
                    count: args.count,
                    branch: args.branch,
                    include_graph_context: enable_graph_reasoning,
                },
            )?;
            let format = crate::hypothesis::BundleFormat::from_path_or_name(
                args.output.as_deref(),
                args.format.as_deref(),
            )?;
            crate::hypothesis::write_or_print(&bundle, args.output.as_deref(), format)?;
        }
        HypothesisCommand::Critique(args) => {
            let bundle = crate::hypothesis::read_bundle(&args.file)?;
            let bundle = crate::hypothesis::critique(bundle);
            let format = crate::hypothesis::BundleFormat::from_path_or_name(
                args.output.as_deref().or(Some(args.file.as_path())),
                args.format.as_deref(),
            )?;
            crate::hypothesis::write_or_print(&bundle, args.output.as_deref(), format)?;
        }
        HypothesisCommand::Rank(args) => {
            let bundle = crate::hypothesis::read_bundle(&args.file)?;
            let bundle = crate::hypothesis::rank(bundle);
            let format = crate::hypothesis::BundleFormat::from_path_or_name(
                args.output.as_deref().or(Some(args.file.as_path())),
                args.format.as_deref(),
            )?;
            crate::hypothesis::write_or_print(&bundle, args.output.as_deref(), format)?;
        }
        HypothesisCommand::Evolve(args) => {
            let bundle = crate::hypothesis::read_bundle(&args.file)?;
            let bundle = crate::hypothesis::evolve(bundle, &args.candidate)?;
            let format = crate::hypothesis::BundleFormat::from_path_or_name(
                args.output.as_deref().or(Some(args.file.as_path())),
                args.format.as_deref(),
            )?;
            crate::hypothesis::write_or_print(&bundle, args.output.as_deref(), format)?;
        }
        HypothesisCommand::Accept(args) => {
            let conn = crate::db::open_database(db)?;
            let policy_doc = crate::policy::load_policy(policy)?;
            let bundle = crate::hypothesis::read_bundle(&args.file)?;
            let result = crate::hypothesis::accept(
                &conn,
                &policy_doc,
                &bundle,
                crate::hypothesis::AcceptConfig {
                    candidate_id: args.candidate,
                    program: args.program,
                    branch: args.branch,
                    classification: args.classification.into_schema(),
                    create_experiment: args.create_experiment,
                    experiment_slug: args.experiment_slug,
                },
            )?;
            println!("accepted hypothesis as option {}", result.option_slug);
            println!("option_id: {}", result.option_id);
            if let Some(experiment_slug) = result.experiment_slug {
                println!("experiment: {experiment_slug}");
            }
            if let Some(experiment_id) = result.experiment_id {
                println!("experiment_id: {experiment_id}");
            }
        }
    }
    Ok(())
}

fn handle_matrix(db: &Path, policy: &Path, args: MatrixArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        MatrixCommand::Create(args) => {
            let program = require_program(&conn, &args.program)?;
            let title = args.title.unwrap_or_else(|| args.slug.clone());
            let description = args.description.unwrap_or_default();
            let matrix = crate::db::create_research_matrix(
                &conn,
                &crate::schema::NewResearchMatrix {
                    program_id: program.id,
                    slug: &args.slug,
                    title: &title,
                    description: &description,
                    status: crate::schema::MatrixStatus::Active,
                },
            )?;
            println!("created matrix {}", matrix.slug);
        }
        MatrixCommand::List => {
            let program = require_current_program(&conn, policy)?;
            let matrices = crate::db::list_research_matrices(&conn, program.id)?;
            if matrices.is_empty() {
                println!("No matrices.");
            } else {
                for matrix in matrices {
                    println!("{} [{}] {}", matrix.slug, matrix.status, matrix.title);
                }
            }
        }
        MatrixCommand::Show(args) => {
            let (program, matrix) = require_matrix_in_current_program(&conn, policy, &args.slug)?;
            let axes = matrix_axes_with_levels(&conn, matrix.id)?;
            let cells = crate::db::list_matrix_cells(&conn, matrix.id)?;
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&matrix_json(&program, &matrix, &axes, &cells)?)?
                );
            } else {
                print_matrix_show(&matrix, &axes, &cells);
            }
        }
        MatrixCommand::Update(args) => {
            let (_program, matrix) = require_matrix_in_current_program(&conn, policy, &args.slug)?;
            let updated = crate::db::update_research_matrix(
                &conn,
                matrix.id,
                &crate::schema::ResearchMatrixUpdate {
                    title: args.title.as_deref(),
                    description: args.description.as_deref(),
                    status: args.status.map(MatrixStatusArg::into_schema),
                },
            )?;
            println!("updated matrix {} [{}]", updated.slug, updated.status);
        }
        MatrixCommand::Axis(args) => handle_matrix_axis(&conn, policy, args)?,
        MatrixCommand::Level(args) => handle_matrix_level(&conn, policy, args)?,
        MatrixCommand::Instantiate(args) => {
            let (_program, matrix) = require_matrix_in_current_program(&conn, policy, &args.slug)?;
            let created = instantiate_matrix_cells(&conn, matrix.id)?;
            println!("instantiated {} matrix cells", created);
        }
        MatrixCommand::Cell(args) => handle_matrix_cell(&conn, policy, args)?,
        MatrixCommand::Compare(args) => {
            let (_program, matrix) = require_matrix_in_current_program(&conn, policy, &args.slug)?;
            let cells = crate::db::list_matrix_cells(&conn, matrix.id)?;
            print_matrix_comparison(&conn, &matrix, &cells, args.metric.as_deref())?;
        }
    }
    Ok(())
}

fn handle_matrix_axis(
    conn: &rusqlite::Connection,
    policy: &Path,
    args: MatrixAxisArgs,
) -> anyhow::Result<()> {
    match args.command {
        MatrixAxisCommand::Add(args) => {
            let (_program, matrix) = require_matrix_in_current_program(conn, policy, &args.matrix)?;
            let title = args.title.unwrap_or_else(|| args.slug.clone());
            let position = match args.position {
                Some(position) => position,
                None => crate::db::next_matrix_axis_position(conn, matrix.id)?,
            };
            let axis = crate::db::create_matrix_axis(
                conn,
                &crate::schema::NewMatrixAxis {
                    matrix_id: matrix.id,
                    slug: &args.slug,
                    title: &title,
                    position,
                },
            )?;
            println!("created matrix axis {}", axis.slug);
        }
        MatrixAxisCommand::List(args) => {
            let (_program, matrix) = require_matrix_in_current_program(conn, policy, &args.matrix)?;
            let axes = matrix_axes_with_levels(conn, matrix.id)?;
            if axes.is_empty() {
                println!("No axes.");
            } else {
                for (axis, levels) in axes {
                    println!("{} [{}] {}", axis.slug, axis.position, axis.title);
                    for level in levels {
                        println!("  - {} [{}] {}", level.slug, level.position, level.title);
                    }
                }
            }
        }
    }
    Ok(())
}

fn handle_matrix_level(
    conn: &rusqlite::Connection,
    policy: &Path,
    args: MatrixLevelArgs,
) -> anyhow::Result<()> {
    match args.command {
        MatrixLevelCommand::Add(args) => {
            let (_program, matrix) = require_matrix_in_current_program(conn, policy, &args.matrix)?;
            let axis = crate::db::get_matrix_axis_by_slug(conn, matrix.id, &args.axis)?
                .with_context(|| format!("matrix axis `{}` not found", args.axis))?;
            let title = args.title.unwrap_or_else(|| args.slug.clone());
            let position = match args.position {
                Some(position) => position,
                None => crate::db::next_matrix_level_position(conn, axis.id)?,
            };
            let level = crate::db::create_matrix_level(
                conn,
                &crate::schema::NewMatrixLevel {
                    axis_id: axis.id,
                    slug: &args.slug,
                    title: &title,
                    position,
                },
            )?;
            println!("created matrix level {}.{}", axis.slug, level.slug);
        }
        MatrixLevelCommand::List(args) => {
            let (_program, matrix) = require_matrix_in_current_program(conn, policy, &args.matrix)?;
            let axis = crate::db::get_matrix_axis_by_slug(conn, matrix.id, &args.axis)?
                .with_context(|| format!("matrix axis `{}` not found", args.axis))?;
            let levels = crate::db::list_matrix_levels(conn, axis.id)?;
            if levels.is_empty() {
                println!("No levels.");
            } else {
                for level in levels {
                    println!("{} [{}] {}", level.slug, level.position, level.title);
                }
            }
        }
    }
    Ok(())
}

fn handle_matrix_cell(
    conn: &rusqlite::Connection,
    policy: &Path,
    args: MatrixCellArgs,
) -> anyhow::Result<()> {
    match args.command {
        MatrixCellCommand::List(args) => {
            let (_program, matrix) = require_matrix_in_current_program(conn, policy, &args.matrix)?;
            let cells = crate::db::list_matrix_cells(conn, matrix.id)?;
            if cells.is_empty() {
                println!("No cells.");
            } else {
                for cell in cells {
                    println!(
                        "{} [{}] experiment={} {}",
                        cell.slug,
                        cell.status,
                        cell.experiment_id
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "none".to_owned()),
                        cell.title
                    );
                }
            }
        }
        MatrixCellCommand::Link(args) => {
            let (program, matrix) = require_matrix_in_current_program(conn, policy, &args.matrix)?;
            let cell = crate::db::get_matrix_cell_by_slug(conn, matrix.id, &args.cell)?
                .with_context(|| format!("matrix cell `{}` not found", args.cell))?;
            let experiment =
                crate::db::find_experiment_by_slug_in_program(conn, program.id, &args.experiment)?
                    .with_context(|| {
                        format!("experiment `{}` not found in program", args.experiment)
                    })?;
            let cell = crate::db::update_matrix_cell(
                conn,
                cell.id,
                &crate::schema::MatrixCellUpdate {
                    experiment_id: Some(Some(experiment.id)),
                    status: Some(matrix_cell_status_from_experiment(&experiment.status)),
                    notes: None,
                },
            )?;
            println!(
                "linked matrix cell {} to experiment {}",
                cell.slug, experiment.slug
            );
        }
        MatrixCellCommand::Mark(args) => {
            let (_program, matrix) = require_matrix_in_current_program(conn, policy, &args.matrix)?;
            let cell = crate::db::get_matrix_cell_by_slug(conn, matrix.id, &args.cell)?
                .with_context(|| format!("matrix cell `{}` not found", args.cell))?;
            let cell = crate::db::update_matrix_cell(
                conn,
                cell.id,
                &crate::schema::MatrixCellUpdate {
                    experiment_id: None,
                    status: Some(args.status.into_schema()),
                    notes: args.notes.as_deref().map(Some),
                },
            )?;
            println!("marked matrix cell {} [{}]", cell.slug, cell.status);
        }
    }
    Ok(())
}

fn handle_question(db: &Path, policy: &Path, args: QuestionArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        QuestionCommand::Add(args) => {
            let program = require_program(&conn, &args.program)?;
            let branch_id = match args.branch {
                Some(branch_slug) => Some(require_branch(&conn, program.id, &branch_slug)?.id),
                None => None,
            };
            let context = args.context.unwrap_or_default();
            let question = crate::db::create_open_question(
                &conn,
                &crate::schema::NewOpenQuestion {
                    program_id: program.id,
                    branch_id,
                    slug: &args.slug,
                    question: &args.question,
                    context: &context,
                    status: crate::schema::OpenQuestionStatus::Open,
                },
            )?;
            println!("created question {}", question.slug);
        }
        QuestionCommand::List(args) => {
            let filter = question_filter_from_args(&conn, policy, args.program, args.branch, None)?;
            let questions = crate::db::list_open_questions(&conn, &filter)?;
            if questions.is_empty() {
                println!("No questions.");
            } else {
                for question in questions {
                    println!(
                        "{} [{}] {}",
                        question.slug, question.status, question.question
                    );
                }
            }
        }
        QuestionCommand::Show(args) => {
            let question = require_question_in_current_program(&conn, policy, &args.slug)?;
            print_question(&question);
        }
        QuestionCommand::Update(args) => {
            let question = require_question_in_current_program(&conn, policy, &args.slug)?;
            if let Some(status) = args.status {
                let status =
                    parse_enum::<crate::schema::OpenQuestionStatus>(&status, "question status")?;
                let question = crate::db::update_open_question_status(&conn, question.id, status)?;
                println!("updated question {} [{}]", question.slug, question.status);
            } else {
                println!("no question updates requested");
            }
        }
        QuestionCommand::Answer(args) => {
            let question = require_question_in_current_program(&conn, policy, &args.slug)?;
            let question = crate::db::answer_open_question(&conn, question.id)?;
            println!("answered question {}", question.slug);
            println!("summary: {}", args.summary);
        }
        QuestionCommand::Reject(args) => {
            let question = require_question_in_current_program(&conn, policy, &args.slug)?;
            let question = crate::db::reject_open_question(&conn, question.id)?;
            println!("rejected question {}", question.slug);
        }
        QuestionCommand::Supersede(args) => {
            let question = require_question_in_current_program(&conn, policy, &args.slug)?;
            let question = crate::db::supersede_open_question(&conn, question.id)?;
            println!("superseded question {}", question.slug);
        }
    }
    Ok(())
}

fn handle_option(db: &Path, policy: &Path, args: OptionArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        OptionCommand::Add(args) => {
            let program = require_program(&conn, &args.program)?;
            let branch_id = match args.branch {
                Some(branch_slug) => Some(require_branch(&conn, program.id, &branch_slug)?.id),
                None => None,
            };
            let open_question_id = match args.open_question {
                Some(question_slug) => {
                    Some(require_question(&conn, program.id, &question_slug)?.id)
                }
                None => None,
            };
            let title = args.title.unwrap_or_else(|| args.slug.clone());
            let option = crate::db::create_research_option(
                &conn,
                &crate::schema::NewResearchOption {
                    program_id: program.id,
                    branch_id,
                    open_question_id,
                    source_experiment_id: None,
                    source_decision_id: None,
                    slug: &args.slug,
                    title: &title,
                    hypothesis: args.hypothesis.as_deref(),
                    description: &args.description,
                    classification: args.classification.into_schema(),
                    status: crate::schema::ResearchOptionStatus::Open,
                },
            )?;
            println!("created option {}", option.slug);
        }
        OptionCommand::List(args) => {
            let policy_doc = crate::policy::load_policy(policy)?;
            let program_id = match policy_doc.current_program.as_deref() {
                Some(slug) => Some(require_program(&conn, slug)?.id),
                None => None,
            };
            let options = crate::db::list_research_options(
                &conn,
                &crate::schema::ResearchOptionFilter {
                    program_id,
                    branch_id: None,
                    status: args.status.map(OptionStatus::into_schema),
                    classification: args.classification.map(OptionClassification::into_schema),
                },
            )?;
            if options.is_empty() {
                println!("No options.");
            } else {
                for option in options {
                    println!(
                        "{} [{}, {}] {}",
                        option.slug, option.status, option.classification, option.title
                    );
                }
            }
        }
        OptionCommand::Show(args) => {
            let option = require_option_in_current_program(&conn, policy, &args.slug)?;
            print_option(&option);
        }
        OptionCommand::Select(args) => {
            let option = require_option_in_current_program(&conn, policy, &args.slug)?;
            let option = crate::db::select_research_option(
                &conn,
                option.id,
                &args.rationale,
                args.by.as_deref(),
            )?;
            println!("selected option {}", option.slug);
            if option.review_state != "none" {
                println!("review_state: {}", option.review_state);
            }
        }
        OptionCommand::Update(args) => {
            let option = require_option_in_current_program(&conn, policy, &args.slug)?;
            if args.status.is_some()
                || args.title.is_some()
                || args.hypothesis.is_some()
                || args.description.is_some()
                || args.classification.is_some()
            {
                let option = crate::db::update_research_option(
                    &conn,
                    option.id,
                    &crate::schema::ResearchOptionUpdate {
                        branch_id: None,
                        open_question_id: None,
                        source_experiment_id: None,
                        source_decision_id: None,
                        title: args.title.as_deref(),
                        hypothesis: args.hypothesis.as_ref().map(|value| value.as_deref()),
                        description: args.description.as_deref(),
                        classification: args.classification.map(OptionClassification::into_schema),
                        status: args.status.map(OptionStatus::into_schema),
                    },
                )?;
                println!("updated option {} [{}]", option.slug, option.status);
            } else {
                println!("no option updates requested");
            }
        }
        OptionCommand::Reject(args) => {
            let option = require_option_in_current_program(&conn, policy, &args.slug)?;
            let option = crate::db::reject_research_option(&conn, option.id)?;
            println!("rejected option {}", option.slug);
        }
        OptionCommand::Supersede(args) => {
            let option = require_option_in_current_program(&conn, policy, &args.slug)?;
            let option = crate::db::supersede_research_option(&conn, option.id)?;
            println!("superseded option {}", option.slug);
        }
    }
    Ok(())
}

fn handle_experiment(db: &Path, policy: &Path, args: ExperimentArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        ExperimentCommand::Create(args) => {
            let policy_doc = crate::policy::load_policy(policy)?;
            let program_slug = policy_doc.current_program.as_deref().context(
                "no current program set; run `ldgr-research program set-current <slug>`",
            )?;
            let program = require_program(&conn, program_slug)?;
            let branch = require_branch(&conn, program.id, &args.branch)?;
            let option_id = match args.option {
                Some(option_slug) => {
                    let option = require_option_in_program(&conn, program.id, &option_slug)?;
                    Some(option.id)
                }
                None => None,
            };
            let mode = args.mode.into_schema();
            validate_experiment_mode_inputs(
                mode,
                args.hypothesis.as_deref(),
                args.fail_criteria.as_slice(),
                args.observation_goal.as_deref(),
                args.rationale.as_deref(),
            )?;
            let title = args.title.unwrap_or_else(|| args.slug.clone());
            let primary_metrics_json = serde_json::to_string(&args.primary_metrics)?;
            let secondary_metrics_json = serde_json::to_string(&args.secondary_metrics)?;
            let pass_criteria = join_json_array(args.pass_criteria)?;
            let fail_criteria = join_json_array(args.fail_criteria)?;
            let allowed_next_steps = join_json_array(args.allowed_next)?;
            let blocked_next_steps = join_json_array(args.blocked_next)?;
            let experiment = crate::db::create_experiment(
                &conn,
                &crate::schema::NewExperiment {
                    branch_id: branch.id,
                    option_id,
                    slug: &args.slug,
                    title: &title,
                    phase: args.phase.as_deref(),
                    mode,
                    hypothesis: args.hypothesis.as_deref(),
                    setup: args.setup.as_deref(),
                    observation_goal: args.observation_goal.as_deref(),
                    rationale: args.rationale.as_deref(),
                    primary_metrics_json: &primary_metrics_json,
                    secondary_metrics_json: &secondary_metrics_json,
                    pass_criteria: pass_criteria.as_deref(),
                    fail_criteria: fail_criteria.as_deref(),
                    allowed_next_steps: allowed_next_steps.as_deref(),
                    blocked_next_steps: blocked_next_steps.as_deref(),
                    status: crate::schema::ExperimentStatus::Planned,
                },
            )?;
            println!("created experiment {}", experiment.slug);
        }
        ExperimentCommand::List(args) => {
            let (_program, branch) =
                resolve_current_program_branch(&conn, policy, args.branch.as_deref())?;
            let experiments = crate::db::list_experiments(&conn, branch.id)?;
            if experiments.is_empty() {
                println!("No experiments.");
            } else {
                for experiment in experiments {
                    println!(
                        "{} [{}] {}",
                        experiment.slug, experiment.status, experiment.title
                    );
                }
            }
        }
        ExperimentCommand::Show(args) => {
            let experiment = require_experiment_in_current_branch(&conn, policy, &args.slug)?;
            print_experiment(&experiment);
        }
        ExperimentCommand::Submit(args) => {
            handle_experiment_submit(&conn, policy, args)?;
        }
        ExperimentCommand::Update(args) => {
            let experiment = require_experiment_in_current_branch(&conn, policy, &args.slug)?;
            if experiment_update_empty(&args) {
                println!("no experiment updates requested");
            } else {
                let mode = args.mode.map(ExperimentMode::into_schema);
                let status = args.status.map(ExperimentStatus::into_schema);
                if let Some(status) = status {
                    validate_experiment_status_transition(&experiment.status, status)?;
                }
                let primary_metrics_json = if args.primary_metrics.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(&args.primary_metrics)?)
                };
                let secondary_metrics_json = if args.secondary_metrics.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(&args.secondary_metrics)?)
                };
                let pass_criteria = join_json_array(args.pass_criteria)?;
                let fail_criteria = join_json_array(args.fail_criteria)?;
                let allowed_next_steps = join_json_array(args.allowed_next)?;
                let blocked_next_steps = join_json_array(args.blocked_next)?;
                let experiment = crate::db::update_experiment(
                    &conn,
                    experiment.id,
                    &crate::schema::ExperimentUpdate {
                        option_id: None,
                        title: args.title.as_deref(),
                        phase: args.phase.as_deref().map(Some),
                        mode,
                        hypothesis: args.hypothesis.as_deref().map(Some),
                        setup: args.setup.as_deref().map(Some),
                        observation_goal: args.observation_goal.as_deref().map(Some),
                        rationale: args.rationale.as_deref().map(Some),
                        primary_metrics_json: primary_metrics_json.as_deref(),
                        secondary_metrics_json: secondary_metrics_json.as_deref(),
                        pass_criteria: pass_criteria.as_deref().map(Some),
                        fail_criteria: fail_criteria.as_deref().map(Some),
                        allowed_next_steps: allowed_next_steps.as_deref().map(Some),
                        blocked_next_steps: blocked_next_steps.as_deref().map(Some),
                        status,
                    },
                )?;
                println!(
                    "updated experiment {} [{}]",
                    experiment.slug, experiment.status
                );
            }
        }
        ExperimentCommand::Complete(args) => {
            let policy_doc = crate::policy::load_policy(policy)?;
            let experiment = require_experiment_in_current_branch(&conn, policy, &args.slug)?;
            validate_experiment_completion(&conn, &policy_doc, &experiment)?;
            let experiment = crate::db::update_experiment_status(
                &conn,
                experiment.id,
                crate::schema::ExperimentStatus::Completed,
            )?;
            if let Some(option_id) = experiment.option_id {
                let option = crate::db::get_research_option_by_id(&conn, option_id)?;
                if matches!(option.status.as_str(), "open" | "selected" | "in_progress") {
                    let option = crate::db::update_research_option_status(
                        &conn,
                        option.id,
                        crate::schema::ResearchOptionStatus::Answered,
                    )?;
                    println!("answered option {}", option.slug);
                }
            }
            write_experiment_outputs(&conn, policy, &experiment, None)?;
            println!("completed experiment {}", experiment.slug);
        }
    }
    Ok(())
}

fn handle_run(db: &Path, policy: &Path, args: RunArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        RunCommand::Start(args) => {
            let experiment = require_experiment_in_current_branch(&conn, policy, &args.experiment)?;
            let environment_json = environment_json(args.env)?;
            let run = crate::db::create_run(
                &conn,
                &crate::schema::NewRun {
                    experiment_id: experiment.id,
                    command: args.command.as_deref(),
                    environment_json: &environment_json,
                    dataset: args.dataset.as_deref(),
                    code_ref: args.code_ref.as_deref(),
                    notes: args.notes.as_deref(),
                },
            )?;
            println!("started run {}", run.id);
        }
        RunCommand::Finish(args) => {
            let run_id = parse_id(&args.run_id, "run id")?;
            let status = args.status.into_schema();
            if status == crate::schema::RunStatus::Running {
                bail!("run finish requires a terminal status");
            }
            let run = crate::db::finish_run(&conn, run_id, status, args.notes.as_deref())?;
            println!("finished run {} [{}]", run.id, run.status);
        }
        RunCommand::Fail(args) => {
            let run_id = parse_id(&args.run_id, "run id")?;
            let run = crate::db::fail_run(&conn, run_id, args.notes.as_deref())?;
            println!("failed run {} [{}]", run.id, run.status);
        }
        RunCommand::List(args) => {
            let experiment = require_experiment_in_current_branch(
                &conn,
                policy,
                args.experiment
                    .as_deref()
                    .context("run list requires --experiment <slug>")?,
            )?;
            let runs = crate::db::list_runs_by_experiment(&conn, experiment.id)?;
            if runs.is_empty() {
                println!("No runs.");
            } else {
                for run in runs {
                    println!("{} [{}] {}", run.id, run.status, run.started_at);
                }
            }
        }
    }
    Ok(())
}

fn handle_metric(db: &Path, policy: &Path, args: MetricArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        MetricCommand::Add(args) => {
            let run_id = parse_id(&args.run_id, "run id")?;
            let metric = crate::db::create_metric(
                &conn,
                &crate::schema::NewMetric {
                    run_id,
                    name: &args.name,
                    value: args.value,
                    unit: args.unit.as_deref(),
                    higher_is_better: args.higher_is_better,
                    split: args.split.as_deref(),
                    metadata_json: "{}",
                },
            )?;
            println!("added metric {} to run {}", metric.name, metric.run_id);
        }
        MetricCommand::Trend(args) => {
            if args.group_by.as_deref() != Some("experiment") {
                bail!("metric trend currently supports --by experiment");
            }
            let metrics = crate::db::list_metrics_by_name(&conn, &args.name)?;
            if metrics.is_empty() {
                println!("No metrics named {}.", args.name);
            } else {
                for metric in metrics {
                    println!("run {} {}={}", metric.run_id, metric.name, metric.value);
                }
            }
        }
        MetricCommand::List(args) => {
            let experiment = require_experiment_in_current_branch(
                &conn,
                policy,
                args.experiment
                    .as_deref()
                    .context("metric list requires --experiment <slug>")?,
            )?;
            let metrics = crate::db::list_metrics_by_experiment(&conn, experiment.id)?;
            if metrics.is_empty() {
                println!("No metrics.");
            } else {
                for metric in metrics {
                    println!("run {} {}={}", metric.run_id, metric.name, metric.value);
                }
            }
        }
    }
    Ok(())
}

fn handle_artifact(db: &Path, policy: &Path, args: ArtifactArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        ArtifactCommand::Add(args) => {
            let policy_doc = crate::policy::load_policy(policy)?;
            validate_artifact_path(&args.path, &policy_doc.allowed_artifact_roots)?;
            let run_id = parse_id(&args.run_id, "run id")?;
            let checksum = if args.checksum {
                compute_sha256(&args.path)?
            } else {
                None
            };
            let description = args.description.unwrap_or_default();
            let artifact = crate::db::create_artifact(
                &conn,
                &crate::schema::NewArtifact {
                    run_id,
                    kind: args.kind.into_schema(),
                    path: &args.path.to_string_lossy(),
                    description: &description,
                    checksum: checksum.as_deref(),
                    metadata_json: "{}",
                },
            )?;
            println!(
                "added artifact {} to run {}",
                artifact.path, artifact.run_id
            );
            if let Some(checksum) = artifact.checksum.as_deref() {
                println!("checksum: {checksum}");
            }
        }
        ArtifactCommand::List(args) => {
            let experiment = require_experiment_in_current_branch(
                &conn,
                policy,
                args.experiment
                    .as_deref()
                    .context("artifact list requires --experiment <slug>")?,
            )?;
            let artifacts = crate::db::list_artifacts_by_experiment(&conn, experiment.id)?;
            if artifacts.is_empty() {
                println!("No artifacts.");
            } else {
                for artifact in artifacts {
                    println!(
                        "run {} [{}] {}",
                        artifact.run_id, artifact.kind, artifact.path
                    );
                }
            }
        }
    }
    Ok(())
}

fn handle_decision(db: &Path, policy: &Path, args: DecisionArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        DecisionCommand::Add(args) => {
            let (program, branch) = resolve_current_program_branch(&conn, policy, None)?;
            let experiment = crate::db::get_experiment_by_slug(&conn, branch.id, &args.experiment)?
                .with_context(|| {
                    format!(
                        "experiment `{}` not found in current branch `{}`",
                        args.experiment, branch.slug
                    )
                })?;
            let next_branch_id = match args.next_branch.as_deref() {
                Some(slug) => Some(require_branch(&conn, program.id, slug)?.id),
                None => None,
            };
            let next_experiment_id = match args.next_experiment.as_deref() {
                Some(slug) => Some(
                    crate::db::get_experiment_by_slug(&conn, branch.id, slug)?
                        .with_context(|| format!("next experiment `{slug}` not found"))?
                        .id,
                ),
                None => None,
            };
            let proposed_options = parse_proposed_options(&args.propose_options)?;
            let proposed_options_json = serde_json::to_string(&proposed_options)?;
            let decision = crate::db::create_decision(
                &conn,
                &crate::schema::NewDecision {
                    experiment_id: experiment.id,
                    result_summary: &args.result,
                    interpretation: &args.interpretation,
                    limitations: &args.limitations,
                    decision: args.decision.into_schema(),
                    confidence: args.confidence.into_schema(),
                    next_branch_id,
                    next_experiment_id,
                    proposed_options_json: &proposed_options_json,
                },
            )?;
            println!(
                "added decision {} to experiment {}",
                decision.id, experiment.slug
            );
        }
    }
    Ok(())
}

fn handle_fact(db: &Path, policy: &Path, args: FactArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        FactCommand::Add(args) => {
            let program = require_program(&conn, &args.program)?;
            let branch_id = resolve_branch_for_program(&conn, policy, program.id)?.map(|b| b.id);
            let evidence = evidence_from_creation_args(
                &conn,
                policy,
                program.id,
                args.evidence_experiment.as_deref(),
                args.evidence_artifact.as_deref(),
                args.evidence_report.as_deref(),
                &args.statement,
            )?;
            let fact = crate::db::create_fact(
                &conn,
                &crate::schema::NewFact {
                    program_id: program.id,
                    branch_id,
                    slug: &args.slug,
                    statement: &args.statement,
                    status: args.status.into_schema(),
                    confidence: None,
                    created_from_experiment_id: evidence.experiment_id,
                    created_from_decision_id: evidence.decision_id,
                },
                &[evidence],
            )?;
            println!("created fact {} [{}]", fact.slug, fact.status);
            if fact.review_state != "none" {
                println!("review_state: {}", fact.review_state);
            }
        }
        FactCommand::List => {
            let policy_doc = crate::policy::load_policy(policy)?;
            let program_id = match policy_doc.current_program.as_deref() {
                Some(slug) => Some(require_program(&conn, slug)?.id),
                None => None,
            };
            let facts = crate::db::list_facts(
                &conn,
                &crate::schema::FactFilter {
                    program_id,
                    branch_id: None,
                    status: None,
                    review_state: None,
                },
            )?;
            if facts.is_empty() {
                println!("No facts.");
            } else {
                for fact in facts {
                    println!("{} [{}] {}", fact.slug, fact.status, fact.statement);
                }
            }
        }
        FactCommand::Show(args) => {
            let fact = require_fact_in_current_program(&conn, policy, &args.slug)?;
            print_fact(&conn, &fact)?;
        }
        FactCommand::Update(args) => {
            let fact = require_fact_in_current_program(&conn, policy, &args.slug)?;
            let review_state = if args.reviewed_by.is_some() {
                Some(crate::schema::ReviewState::Reviewed)
            } else {
                None
            };
            let updated = crate::db::update_fact(
                &conn,
                fact.id,
                &crate::schema::FactUpdate {
                    branch_id: None,
                    statement: None,
                    status: args.status.map(FactStatus::into_schema),
                    confidence: None,
                    created_from_experiment_id: None,
                    created_from_decision_id: None,
                    review_state,
                },
            )?;
            println!("updated fact {} [{}]", updated.slug, updated.status);
        }
        FactCommand::Evidence(args) => match args.command {
            FactEvidenceCommand::Add(args) => {
                let fact = require_fact_in_current_program(&conn, policy, &args.slug)?;
                let evidence = evidence_from_args(&conn, policy, fact.program_id, &args)?;
                let link = crate::db::add_fact_evidence(&conn, fact.id, &evidence)?;
                println!("added evidence {} to fact {}", link.id, fact.slug);
            }
        },
    }
    Ok(())
}

fn handle_axiom(db: &Path, policy: &Path, args: AxiomArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        AxiomCommand::Add(args) => {
            let program = require_program(&conn, &args.program)?;
            let branch_id = resolve_branch_for_program(&conn, policy, program.id)?.map(|b| b.id);
            let created_by_agent = args
                .by
                .as_deref()
                .is_some_and(|actor| actor.eq_ignore_ascii_case("agent"));
            let axiom = crate::db::create_axiom(
                &conn,
                &crate::schema::NewAxiom {
                    program_id: program.id,
                    branch_id,
                    slug: &args.slug,
                    statement: &args.statement,
                    status: crate::schema::AxiomStatus::Active,
                    created_by_actor: args.by.as_deref(),
                    created_by_agent,
                },
            )?;
            println!("created axiom {} [{}]", axiom.slug, axiom.status);
            if axiom.review_state != "none" {
                println!("review_state: {}", axiom.review_state);
            }
        }
        AxiomCommand::List => {
            let policy_doc = crate::policy::load_policy(policy)?;
            let program_id = match policy_doc.current_program.as_deref() {
                Some(slug) => Some(require_program(&conn, slug)?.id),
                None => None,
            };
            let axioms = crate::db::list_axioms(
                &conn,
                &crate::schema::AxiomFilter {
                    program_id,
                    branch_id: None,
                    status: None,
                    review_state: None,
                },
            )?;
            if axioms.is_empty() {
                println!("No axioms.");
            } else {
                for axiom in axioms {
                    println!("{} [{}] {}", axiom.slug, axiom.status, axiom.statement);
                }
            }
        }
        AxiomCommand::Show(args) => {
            let axiom = require_axiom_in_current_program(&conn, policy, &args.slug)?;
            print_axiom(&conn, &axiom)?;
        }
        AxiomCommand::Update(args) => {
            let axiom = require_axiom_in_current_program(&conn, policy, &args.slug)?;
            let updated = crate::db::update_axiom(
                &conn,
                axiom.id,
                &crate::schema::AxiomUpdate {
                    branch_id: None,
                    statement: None,
                    status: args.status.map(AxiomStatus::into_schema),
                    created_by_actor: None,
                    review_state: args
                        .approved_by
                        .as_ref()
                        .map(|_| crate::schema::ReviewState::Reviewed),
                    approved_by: args.approved_by.as_deref(),
                },
            )?;
            println!("updated axiom {} [{}]", updated.slug, updated.status);
        }
        AxiomCommand::Evidence(args) => match args.command {
            AxiomEvidenceCommand::Add(args) => {
                let axiom = require_axiom_in_current_program(&conn, policy, &args.slug)?;
                let evidence = evidence_from_args(&conn, policy, axiom.program_id, &args)?;
                let link = crate::db::add_axiom_evidence(&conn, axiom.id, &evidence)?;
                println!("added evidence {} to axiom {}", link.id, axiom.slug);
            }
        },
    }
    Ok(())
}

fn handle_review(db: &Path, args: ReviewArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        ReviewCommand::List => {
            let items =
                crate::db::list_review_items(&conn, &crate::schema::ReviewItemFilter::default())?;
            if items.is_empty() {
                println!("No review items.");
            } else {
                for item in items {
                    println!(
                        "{} [{}] {} {}: {}",
                        item.id, item.state, item.entity_type, item.entity_id, item.reason
                    );
                }
            }
        }
        ReviewCommand::Show(args) => {
            let item = crate::db::get_review_item_by_id(&conn, parse_id(&args.id, "review id")?)?;
            print_review_item(&item);
        }
        ReviewCommand::Mark(args) => {
            let id = parse_id(&args.review_id, "review id")?;
            let item = match args.state {
                ReviewState::Reviewed => crate::db::mark_review_item_reviewed(
                    &conn,
                    id,
                    Some(&args.by),
                    args.notes.as_deref(),
                )?,
                ReviewState::Dismissed => crate::db::dismiss_review_item(
                    &conn,
                    id,
                    Some(&args.by),
                    args.notes.as_deref(),
                )?,
                ReviewState::NeedsReview => crate::db::update_review_item_state(
                    &conn,
                    id,
                    crate::schema::ReviewItemState::NeedsReview,
                    Some(&args.by),
                    args.notes.as_deref(),
                )?,
            };
            println!("review {} [{}]", item.id, item.state);
        }
        ReviewCommand::Dismiss(args) => {
            let id = parse_id(&args.review_id, "review id")?;
            let item = crate::db::dismiss_review_item(
                &conn,
                id,
                args.by.as_deref(),
                args.notes.as_deref(),
            )?;
            println!("review {} [{}]", item.id, item.state);
        }
    }
    Ok(())
}

fn handle_override(db: &Path, args: OverrideArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        OverrideCommand::Request(args) => {
            let request = crate::db::request_override_approval(
                &conn,
                &crate::schema::NewOverrideApprovalRequest {
                    entity_type: "blocked_work",
                    entity_id: 0,
                    blocked_work: &args.blocked_work,
                    requested_action: &args.action,
                    justification: &args.justification,
                },
            )?;
            println!("requested override {}", request.id);
        }
        OverrideCommand::List => {
            let items = crate::db::list_override_approvals(
                &conn,
                &crate::schema::OverrideApprovalFilter::default(),
            )?;
            if items.is_empty() {
                println!("No overrides.");
            } else {
                for item in items {
                    println!("{} [{}] {}", item.id, item.status, item.requested_action);
                }
            }
        }
        OverrideCommand::Show(args) => {
            let item =
                crate::db::get_override_approval_by_id(&conn, parse_id(&args.id, "override id")?)?;
            print_override(&item);
        }
        OverrideCommand::Approve(args) => {
            let item = crate::db::approve_override_approval(
                &conn,
                parse_id(&args.override_id, "override id")?,
                &args.by,
            )?;
            println!("override {} [{}]", item.id, item.status);
        }
        OverrideCommand::Reject(args) => {
            let item = crate::db::reject_override_approval(
                &conn,
                parse_id(&args.override_id, "override id")?,
                &args.by,
            )?;
            println!("override {} [{}]", item.id, item.status);
        }
    }
    Ok(())
}

fn handle_bug(db: &Path, policy: &Path, args: BugArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        BugCommand::Report(args) => {
            let (program_id, branch_id, experiment_id) =
                resolve_bug_links(&conn, policy, &args.program, &args.branch, &args.experiment)?;
            let log_excerpt = match (args.log_excerpt.as_deref(), args.log_file.as_deref()) {
                (Some(excerpt), _) => Some(excerpt.to_owned()),
                (None, Some(path)) if path.exists() => Some(read_log_excerpt(path)?),
                _ => None,
            };
            let log_path = args
                .log_file
                .as_deref()
                .map(|path| path.to_string_lossy().to_string());
            let bug = crate::db::create_bug_report(
                &conn,
                &crate::schema::NewBugReport {
                    program_id,
                    branch_id,
                    experiment_id,
                    title: &args.title,
                    description: &args.description,
                    severity: args.severity.into_schema(),
                    command: args.command_text.as_deref(),
                    error: args.error.as_deref(),
                    reproduction: args.reproduction.as_deref(),
                    log_path: log_path.as_deref(),
                    log_excerpt: log_excerpt.as_deref(),
                    reported_by: args.by.as_deref(),
                },
            )?;
            println!("reported bug {} [{}]", bug.id, bug.severity);
        }
        BugCommand::List(args) => {
            let status = args.status.map(BugStatus::into_schema);
            let reports = crate::db::list_bug_reports(
                &conn,
                &crate::schema::BugReportFilter {
                    status,
                    ..Default::default()
                },
            )?;
            if reports.is_empty() {
                println!("No bug reports.");
            } else {
                for bug in reports {
                    println!("{} [{} {}] {}", bug.id, bug.status, bug.severity, bug.title);
                }
            }
        }
        BugCommand::Show(args) => {
            let bug = crate::db::get_bug_report_by_id(&conn, parse_id(&args.id, "bug id")?)?;
            print_bug_report(&bug);
        }
        BugCommand::Mark(args) => {
            let bug = crate::db::update_bug_report_status(
                &conn,
                parse_id(&args.bug_id, "bug id")?,
                args.status.into_schema(),
                args.by.as_deref(),
                args.notes.as_deref(),
            )?;
            println!("bug {} [{}]", bug.id, bug.status);
        }
    }
    Ok(())
}

fn handle_tool(tools: &Path, args: ToolArgs) -> anyhow::Result<()> {
    match args.command {
        ToolCommand::Init => {
            if crate::tools::write_starter_registry_if_missing(tools)? {
                println!("created tool registry {}", tools.display());
            } else {
                println!("tool registry already exists: {}", tools.display());
            }
        }
        ToolCommand::List => {
            let registry = crate::tools::load_registry(tools)?;
            if registry.tools.is_empty() {
                println!("No registered research tools.");
            } else {
                for tool in registry.tools {
                    println!(
                        "{} [{:?}/{:?}] {}",
                        tool.slug, tool.kind, tool.status, tool.path
                    );
                    println!("  purpose: {}", tool.purpose);
                }
            }
        }
        ToolCommand::Show(args) => {
            let registry = crate::tools::load_registry(tools)?;
            let tool = crate::tools::find_tool(&registry, &args.slug)
                .with_context(|| format!("tool {:?} not found", args.slug))?;
            println!("Tool: {}", tool.slug);
            println!("kind: {:?}", tool.kind);
            println!("status: {:?}", tool.status);
            println!("path: {}", tool.path);
            println!("mutability: {:?}", tool.mutability);
            println!("purpose: {}", tool.purpose);
            if !tool.inputs.is_empty() {
                println!("inputs: {}", tool.inputs.join(", "));
            }
            if !tool.outputs.is_empty() {
                println!("outputs: {}", tool.outputs.join(", "));
            }
            if let Some(notes) = tool.notes.as_deref() {
                println!("notes: {notes}");
            }
        }
        ToolCommand::Check => {
            let registry = crate::tools::load_registry(tools)?;
            let findings = crate::tools::validate_registry(&registry, Path::new("."));
            print!("{}", crate::tools::format_findings(&findings));
            if crate::tools::has_errors(&findings) {
                bail!("tool registry has validation errors");
            }
        }
    }
    Ok(())
}

fn print_review_item(item: &crate::schema::ReviewItem) {
    println!("Review: {}", item.id);
    println!("entity: {} {}", item.entity_type, item.entity_id);
    println!("state: {}", item.state);
    println!("reason: {}", item.reason);
    if let Some(reviewed_by) = item.reviewed_by.as_deref() {
        println!("reviewed_by: {reviewed_by}");
    }
    if let Some(notes) = item.notes.as_deref() {
        println!("notes: {notes}");
    }
}

fn print_override(item: &crate::schema::OverrideApproval) {
    println!("Override: {}", item.id);
    println!("entity: {} {}", item.entity_type, item.entity_id);
    println!("status: {}", item.status);
    println!("blocked_work: {}", item.blocked_work);
    println!("requested_action: {}", item.requested_action);
    println!("justification: {}", item.justification);
    if let Some(approved_by) = item.approved_by.as_deref() {
        println!("approved_by: {approved_by}");
    }
}

fn print_bug_report(item: &crate::schema::BugReport) {
    println!("Bug: {}", item.id);
    println!("status: {}", item.status);
    println!("severity: {}", item.severity);
    println!("title: {}", item.title);
    println!("description: {}", item.description);
    if let Some(program_id) = item.program_id {
        println!("program_id: {program_id}");
    }
    if let Some(branch_id) = item.branch_id {
        println!("branch_id: {branch_id}");
    }
    if let Some(experiment_id) = item.experiment_id {
        println!("experiment_id: {experiment_id}");
    }
    if let Some(command) = item.command.as_deref() {
        println!("command: {command}");
    }
    if let Some(error) = item.error.as_deref() {
        println!("error: {error}");
    }
    if let Some(reproduction) = item.reproduction.as_deref() {
        println!("reproduction: {reproduction}");
    }
    if let Some(log_path) = item.log_path.as_deref() {
        println!("log_path: {log_path}");
    }
    if let Some(log_excerpt) = item.log_excerpt.as_deref() {
        println!("log_excerpt:");
        println!("{log_excerpt}");
    }
    if let Some(reported_by) = item.reported_by.as_deref() {
        println!("reported_by: {reported_by}");
    }
    if let Some(resolution_notes) = item.resolution_notes.as_deref() {
        println!("resolution_notes: {resolution_notes}");
    }
    if let Some(resolved_by) = item.resolved_by.as_deref() {
        println!("resolved_by: {resolved_by}");
    }
}

fn print_fact(conn: &rusqlite::Connection, fact: &crate::schema::Fact) -> anyhow::Result<()> {
    println!("Fact: {}", fact.slug);
    println!("id: {}", fact.id);
    println!("statement: {}", fact.statement);
    println!("status: {}", fact.status);
    println!("review_state: {}", fact.review_state);
    print_evidence(conn, "fact", fact.id)
}

fn print_axiom(conn: &rusqlite::Connection, axiom: &crate::schema::Axiom) -> anyhow::Result<()> {
    println!("Axiom: {}", axiom.slug);
    println!("id: {}", axiom.id);
    println!("statement: {}", axiom.statement);
    println!("status: {}", axiom.status);
    println!("review_state: {}", axiom.review_state);
    if let Some(actor) = axiom.created_by_actor.as_deref() {
        println!("created_by: {actor}");
    }
    print_evidence(conn, "axiom", axiom.id)
}

fn print_evidence(
    conn: &rusqlite::Connection,
    subject_type: &str,
    subject_id: i64,
) -> anyhow::Result<()> {
    let links = crate::db::list_evidence_links(conn, subject_type, subject_id)?;
    println!("evidence:");
    if links.is_empty() {
        println!("none");
    } else {
        for link in links {
            let mut targets = Vec::new();
            if let Some(id) = link.experiment_id {
                targets.push(format!("experiment:{id}"));
            }
            if let Some(id) = link.run_id {
                targets.push(format!("run:{id}"));
            }
            if let Some(id) = link.metric_id {
                targets.push(format!("metric:{id}"));
            }
            if let Some(id) = link.artifact_id {
                targets.push(format!("artifact:{id}"));
            }
            if let Some(id) = link.decision_id {
                targets.push(format!("decision:{id}"));
            }
            if let Some(path) = link.report_path.as_deref() {
                let report = match link.report_anchor.as_deref() {
                    Some(anchor) => format!("report:{path}#{anchor}"),
                    None => format!("report:{path}"),
                };
                targets.push(report);
            }
            println!("- {} [{}] {}", link.id, link.relation, targets.join(", "));
            println!("  summary: {}", link.summary);
        }
    }
    Ok(())
}

fn handle_status(db: &Path) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    print!("{}", crate::reports::render_status(&conn)?);
    Ok(())
}

fn handle_tree(db: &Path, args: TreeArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    print!(
        "{}",
        crate::reports::render_tree(&conn, args.program.as_deref())?
    );
    Ok(())
}

fn handle_show(db: &Path, policy: &Path, args: ShowArgs) -> anyhow::Result<()> {
    match args.entity_type {
        ShowEntityType::Experiment => {
            let conn = crate::db::open_database(db)?;
            let (program, branch) = resolve_current_program_branch(&conn, policy, None)?;
            print!(
                "{}",
                crate::reports::render_experiment_markdown(
                    &conn,
                    &program.slug,
                    &branch.slug,
                    &args.slug
                )?
            );
            Ok(())
        }
        _ => bail!("show currently supports experiment"),
    }
}

fn handle_report(db: &Path, policy: &Path, args: ReportArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        ReportCommand::Experiment(target) => {
            let (program, branch) = resolve_current_program_branch(&conn, policy, None)?;
            let report = crate::reports::render_experiment_markdown(
                &conn,
                &program.slug,
                &branch.slug,
                &target.slug,
            )?;
            write_or_print(target.output.as_deref(), &report)
        }
        ReportCommand::Program(target) => {
            let report = crate::reports::render_program_markdown(&conn, &target.slug)?;
            write_or_print(target.output.as_deref(), &report)
        }
        ReportCommand::Branch(target) => {
            let policy_doc = crate::policy::load_policy(policy)?;
            let program_slug = policy_doc.current_program.as_deref().context(
                "no current program set; run `ldgr-research program set-current <slug>`",
            )?;
            let report = crate::reports::render_program_markdown(&conn, program_slug)?;
            if !report.contains(&target.slug) {
                bail!("branch `{}` not found in current program", target.slug);
            }
            write_or_print(target.output.as_deref(), &report)
        }
    }
}

fn handle_export(db: &Path, args: ExportArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        ExportCommand::Markdown(target) => {
            let program = target
                .program
                .context("export markdown requires --program <slug>")?;
            let output = target
                .output
                .context("export markdown requires -o <path>")?;
            let report = crate::reports::render_program_markdown(&conn, &program)?;
            write_or_print(Some(&output), &report)
        }
        ExportCommand::Json(target) => {
            let program = target
                .program
                .context("export json requires --program <slug>")?;
            let output = target.output.context("export json requires -o <path>")?;
            let json = crate::reports::export_program_json(&conn, &program)?;
            write_or_print(Some(&output), &serde_json::to_string_pretty(&json)?)
        }
    }
}

fn handle_dashboard(db: &Path, args: DashboardArgs) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    match args.command {
        DashboardCommand::Build(target) => {
            let html = crate::dashboard::render_dashboard_html(&conn, &target.program)?;
            write_or_print(Some(&target.output), &html)
        }
    }
}

fn handle_guard(db: &Path, policy: &Path, strict: bool) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db)?;
    let policy_doc = crate::policy::load_policy(policy)?;
    let findings =
        crate::guard::validate(&conn, &policy_doc, &crate::guard::ValidationPaths::new("."))?;
    print!("{}", crate::guard::format_findings(&findings));
    if strict && crate::guard::has_errors(&findings) {
        bail!("guard failed with validation errors");
    }
    Ok(())
}

fn handle_migrate(db: &Path) -> anyhow::Result<()> {
    let mut conn = crate::db::open_database(db)?;
    let report = crate::guard::migrate_report(&mut conn)?;
    print!("{}", crate::guard::format_migrate_report(&report));
    Ok(())
}

fn handle_doctor(db: &Path, policy: &Path) -> anyhow::Result<()> {
    let conn = crate::db::open_database(db).ok();
    let policy_doc = crate::policy::load_policy(policy).ok();
    let report = crate::guard::doctor_report(
        conn.as_ref(),
        policy_doc.as_ref(),
        db,
        policy,
        &crate::guard::ValidationPaths::new("."),
    )?;
    print!("{}", crate::guard::format_doctor_report(&report));
    Ok(())
}

fn write_experiment_outputs(
    conn: &rusqlite::Connection,
    policy: &Path,
    experiment: &crate::schema::Experiment,
    submit_artifact: Option<&SubmitArtifact>,
) -> anyhow::Result<()> {
    let report = crate::reports::render_experiment_markdown_by_id(conn, experiment.id)?;
    let report_path = PathBuf::from(".ldgr/research")
        .join("reports")
        .join("experiments")
        .join(format!("{}.md", experiment.slug));
    write_or_print(Some(&report_path), &report)?;
    write_review_package(
        conn,
        policy,
        experiment,
        &report_path,
        &report,
        submit_artifact,
    )
}

fn write_or_print(path: Option<&Path>, contents: &str) -> anyhow::Result<()> {
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create output directory {}", parent.display())
            })?;
        }
        fs::write(path, contents)
            .with_context(|| format!("failed to write output file {}", path.display()))?;
    } else {
        print!("{contents}");
    }
    Ok(())
}

fn write_review_package(
    conn: &rusqlite::Connection,
    policy: &Path,
    experiment: &crate::schema::Experiment,
    report_path: &Path,
    report: &str,
    submit_artifact: Option<&SubmitArtifact>,
) -> anyhow::Result<()> {
    let branch = crate::db::get_branch_by_id(conn, experiment.branch_id)?;
    let program = crate::db::get_program_by_id(conn, branch.program_id)?;
    let policy_doc = crate::policy::load_policy(policy)?;
    let program_export = crate::reports::export_program_json(conn, &program.slug)?;
    let package = serde_json::json!({
        "format": "ldgr-research.review_package.v1",
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "recommended_rubric_version": "researchreview.generic.v1",
        "program": {
            "id": program.id,
            "slug": program.slug,
            "title": program.title,
            "objective": program.objective,
            "status": program.status,
        },
        "branch": {
            "id": branch.id,
            "slug": branch.slug,
            "title": branch.title,
            "question": branch.question,
            "status": branch.status,
        },
        "experiment": {
            "id": experiment.id,
            "branch_id": experiment.branch_id,
            "option_id": experiment.option_id,
            "slug": experiment.slug,
            "title": experiment.title,
            "mode": experiment.mode,
            "status": experiment.status,
        },
        "report": {
            "path": report_path.to_string_lossy(),
            "content": report,
            "anchors": ["result", "interpretation", "limitations", "facts", "next-hypotheses"],
        },
        "submit_artifact": submit_artifact,
        "review_guidance": policy_doc.extra.get("review_guidance"),
        "policy_snapshot": policy_doc,
        "program_snapshot": program_export,
    });
    let package_path = PathBuf::from(".ldgr/research")
        .join("review-packages")
        .join(format!("{}.json", experiment.slug));
    write_or_print(
        Some(&package_path),
        &serde_json::to_string_pretty(&package)?,
    )
}

fn handle_experiment_submit(
    conn: &rusqlite::Connection,
    policy: &Path,
    args: ExperimentSubmit,
) -> anyhow::Result<()> {
    let (program, branch) = resolve_current_program_branch(conn, policy, None)?;
    let experiment =
        crate::db::get_experiment_by_slug(conn, branch.id, &args.slug)?.with_context(|| {
            format!(
                "experiment `{}` not found in current branch `{}`",
                args.slug, branch.slug
            )
        })?;
    let contents = fs::read_to_string(&args.file)
        .with_context(|| format!("failed to read submit file {}", args.file.display()))?;
    let submission: ExperimentSubmitFile = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse submit YAML {}", args.file.display()))?;
    let submit_artifact = SubmitArtifact {
        path: args.file.to_string_lossy().to_string(),
        content: contents.clone(),
    };

    let status = validate_submit_form(&experiment, &submission)?;
    let next_branch_id = match submission.next_branch.as_deref() {
        Some(slug) => Some(require_branch(conn, program.id, slug)?.id),
        None => None,
    };
    let next_experiment_id = match submission.next_experiment.as_deref() {
        Some(slug) => Some(
            crate::db::get_experiment_by_slug(conn, branch.id, slug)?
                .with_context(|| format!("next experiment `{slug}` not found"))?
                .id,
        ),
        None => None,
    };
    let decision = submission
        .decision
        .as_deref()
        .unwrap()
        .parse::<crate::schema::DecisionKind>()
        .context("invalid decision in submit YAML")?;
    let confidence = submission
        .confidence
        .as_deref()
        .unwrap()
        .parse::<crate::schema::Confidence>()
        .context("invalid confidence in submit YAML")?;
    let proposed_options = submission
        .proposed_options
        .iter()
        .map(|option| ProposedOption {
            slug: option.slug.trim().to_owned(),
            description: option.description.trim().to_owned(),
            classification: option.classification.clone(),
        })
        .collect::<Vec<_>>();
    let proposed_options_json = serde_json::to_string(&proposed_options)?;

    crate::db::create_decision(
        conn,
        &crate::schema::NewDecision {
            experiment_id: experiment.id,
            result_summary: submission.result_summary.as_deref().unwrap(),
            interpretation: submission.interpretation.as_deref().unwrap(),
            limitations: submission.limitations.as_deref().unwrap(),
            decision,
            confidence,
            next_branch_id,
            next_experiment_id,
            proposed_options_json: &proposed_options_json,
        },
    )?;

    let experiment = move_experiment_to_terminal_status(conn, &experiment, status)?;
    if let Some(option_id) = experiment.option_id {
        let option = crate::db::get_research_option_by_id(conn, option_id)?;
        if matches!(option.status.as_str(), "open" | "selected" | "in_progress") {
            let option = crate::db::update_research_option_status(
                conn,
                option.id,
                crate::schema::ResearchOptionStatus::Answered,
            )?;
            println!("answered option {}", option.slug);
        }
    }

    create_submit_candidate_facts(conn, &program, &branch, &experiment, &submission)?;
    write_experiment_outputs(conn, policy, &experiment, Some(&submit_artifact))?;
    println!(
        "submitted experiment {} [{}]",
        experiment.slug, experiment.status
    );
    Ok(())
}

fn validate_submit_form(
    experiment: &crate::schema::Experiment,
    submission: &ExperimentSubmitFile,
) -> anyhow::Result<crate::schema::ExperimentStatus> {
    let mut issues = Vec::new();
    require_submit_field(&mut issues, "status", submission.status.as_deref());
    require_submit_field(
        &mut issues,
        "result_summary",
        submission.result_summary.as_deref(),
    );
    require_submit_field(
        &mut issues,
        "interpretation",
        submission.interpretation.as_deref(),
    );
    require_submit_field(
        &mut issues,
        "limitations",
        submission.limitations.as_deref(),
    );
    require_submit_field(&mut issues, "decision", submission.decision.as_deref());
    require_submit_field(&mut issues, "confidence", submission.confidence.as_deref());

    let status = submission.status.as_deref().and_then(|value| match value
        .parse::<crate::schema::ExperimentStatus>(
    ) {
        Ok(status) => Some(status),
        Err(err) => {
            issues.push(format!("status is invalid: {err}"));
            None
        }
    });
    if let Some(status) = status {
        if !terminal_experiment_status(status) {
            issues.push(
                "status must be terminal: completed, failed, inconclusive, or superseded"
                    .to_string(),
            );
        }
        if !submit_status_transition_allowed(&experiment.status, status) {
            issues.push(format!(
                "status transition from {} to {} is invalid for submit",
                experiment.status, status
            ));
        }
    }

    if let Some(decision) = submission.decision.as_deref() {
        if let Err(err) = decision.parse::<crate::schema::DecisionKind>() {
            issues.push(format!("decision is invalid: {err}"));
        }
    }
    if let Some(confidence) = submission.confidence.as_deref() {
        if let Err(err) = confidence.parse::<crate::schema::Confidence>() {
            issues.push(format!("confidence is invalid: {err}"));
        }
    }
    for option in &submission.proposed_options {
        if option.slug.trim().is_empty() {
            issues.push("proposed_options entries require slug".to_string());
        }
        if option.description.trim().is_empty() {
            issues.push(format!(
                "proposed option `{}` requires description",
                option.slug
            ));
        }
        if let Some(classification) = option.classification.as_deref() {
            if let Err(err) = classification.parse::<crate::schema::ResearchOptionClassification>()
            {
                issues.push(format!(
                    "proposed option `{}` has invalid classification: {err}",
                    option.slug
                ));
            }
        }
    }
    for fact in &submission.candidate_facts {
        if fact.slug.trim().is_empty() {
            issues.push("candidate_facts entries require slug".to_string());
        }
        if fact.statement.trim().is_empty() {
            issues.push(format!("candidate fact `{}` requires statement", fact.slug));
        }
        if fact.evidence.report_anchor.is_none() && fact.evidence.artifacts.is_empty() {
            issues.push(format!(
                "candidate fact `{}` requires evidence.report_anchor or evidence.artifacts",
                fact.slug
            ));
        }
    }

    if !issues.is_empty() {
        let mut message = format!(
            "cannot submit experiment {} yet\n\nmissing or invalid:\n",
            experiment.slug
        );
        for issue in issues {
            message.push_str("- ");
            message.push_str(&issue);
            message.push('\n');
        }
        message.push_str("\nAsk the model to reassess the submit YAML and resubmit.");
        bail!("{message}");
    }

    Ok(status.expect("status validated above"))
}

fn require_submit_field(issues: &mut Vec<String>, name: &str, value: Option<&str>) {
    if value.map(str::trim).unwrap_or_default().is_empty() {
        issues.push(format!("{name} is required"));
    }
}

fn terminal_experiment_status(status: crate::schema::ExperimentStatus) -> bool {
    matches!(
        status,
        crate::schema::ExperimentStatus::Completed
            | crate::schema::ExperimentStatus::Failed
            | crate::schema::ExperimentStatus::Inconclusive
            | crate::schema::ExperimentStatus::Superseded
    )
}

fn submit_status_transition_allowed(
    current: &str,
    status: crate::schema::ExperimentStatus,
) -> bool {
    match current {
        "planned" => matches!(
            status,
            crate::schema::ExperimentStatus::Completed
                | crate::schema::ExperimentStatus::Failed
                | crate::schema::ExperimentStatus::Inconclusive
                | crate::schema::ExperimentStatus::Superseded
        ),
        "running" => terminal_experiment_status(status),
        current => current == status.as_str(),
    }
}

fn move_experiment_to_terminal_status(
    conn: &rusqlite::Connection,
    experiment: &crate::schema::Experiment,
    status: crate::schema::ExperimentStatus,
) -> anyhow::Result<crate::schema::Experiment> {
    if experiment.status == status.as_str() {
        return crate::db::get_experiment_by_id(conn, experiment.id);
    }
    if experiment.status == "planned" && status != crate::schema::ExperimentStatus::Superseded {
        crate::db::update_experiment_status(
            conn,
            experiment.id,
            crate::schema::ExperimentStatus::Running,
        )?;
    }
    crate::db::update_experiment_status(conn, experiment.id, status)
}

fn create_submit_candidate_facts(
    conn: &rusqlite::Connection,
    program: &crate::schema::Program,
    branch: &crate::schema::Branch,
    experiment: &crate::schema::Experiment,
    submission: &ExperimentSubmitFile,
) -> anyhow::Result<()> {
    let report_path = format!(".ldgr/research/reports/experiments/{}.md", experiment.slug);
    for fact in &submission.candidate_facts {
        let mut evidence = Vec::new();
        if let Some(anchor) = fact.evidence.report_anchor.as_deref() {
            evidence.push(crate::schema::NewEvidenceLink {
                relation: crate::schema::EvidenceRelation::Supports,
                experiment_id: Some(experiment.id),
                run_id: None,
                metric_id: None,
                artifact_id: None,
                decision_id: None,
                report_path: Some(report_path.as_str()),
                report_anchor: Some(anchor),
                summary: fact.statement.as_str(),
            });
        }
        for artifact_id in &fact.evidence.artifacts {
            evidence.push(crate::schema::NewEvidenceLink {
                relation: crate::schema::EvidenceRelation::Supports,
                experiment_id: Some(experiment.id),
                run_id: None,
                metric_id: None,
                artifact_id: Some(*artifact_id),
                decision_id: None,
                report_path: None,
                report_anchor: None,
                summary: fact.statement.as_str(),
            });
        }
        let created = crate::db::create_fact(
            conn,
            &crate::schema::NewFact {
                program_id: program.id,
                branch_id: Some(branch.id),
                slug: fact.slug.as_str(),
                statement: fact.statement.as_str(),
                status: crate::schema::FactStatus::Candidate,
                confidence: None,
                created_from_experiment_id: Some(experiment.id),
                created_from_decision_id: None,
            },
            &evidence,
        )
        .with_context(|| format!("failed to create candidate fact `{}`", fact.slug))?;
        println!("created candidate fact {}", created.slug);
    }
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct ProposedOption {
    slug: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    classification: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct SubmitArtifact {
    path: String,
    content: String,
}

#[derive(Debug, serde::Deserialize)]
struct ExperimentSubmitFile {
    status: Option<String>,
    result_summary: Option<String>,
    interpretation: Option<String>,
    limitations: Option<String>,
    decision: Option<String>,
    confidence: Option<String>,
    #[serde(default)]
    next_branch: Option<String>,
    #[serde(default)]
    next_experiment: Option<String>,
    #[serde(default)]
    proposed_options: Vec<SubmitProposedOption>,
    #[serde(default)]
    candidate_facts: Vec<SubmitCandidateFact>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct SubmitProposedOption {
    slug: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    classification: Option<String>,
    #[serde(default)]
    hypothesis: Option<String>,
    description: String,
}

#[derive(Debug, serde::Deserialize)]
struct SubmitCandidateFact {
    slug: String,
    statement: String,
    #[serde(default)]
    evidence: SubmitFactEvidence,
}

#[derive(Debug, Default, serde::Deserialize)]
struct SubmitFactEvidence {
    #[serde(default)]
    report_anchor: Option<String>,
    #[serde(default)]
    artifacts: Vec<i64>,
}

fn parse_proposed_options(values: &[String]) -> anyhow::Result<Vec<ProposedOption>> {
    values
        .iter()
        .map(|value| {
            let (slug_part, description) = value
                .split_once(':')
                .with_context(|| format!("proposed option `{value}` must be slug:description"))?;
            let (slug, classification) = match slug_part.split_once('@') {
                Some((slug, classification)) => {
                    let classification = classification.trim();
                    if classification.is_empty() {
                        bail!("proposed option `{value}` must include a non-empty classification");
                    }
                    parse_enum::<crate::schema::ResearchOptionClassification>(
                        classification,
                        "research option classification",
                    )?;
                    (slug, Some(classification.to_owned()))
                }
                None => (slug_part, None),
            };
            if slug.trim().is_empty() || description.trim().is_empty() {
                bail!("proposed option `{value}` must include non-empty slug and description");
            }
            Ok(ProposedOption {
                slug: slug.trim().to_owned(),
                description: description.trim().to_owned(),
                classification,
            })
        })
        .collect()
}

fn require_program(
    conn: &rusqlite::Connection,
    slug: &str,
) -> anyhow::Result<crate::schema::Program> {
    crate::db::get_program_by_slug(conn, slug)?
        .with_context(|| format!("program `{slug}` not found"))
}

fn require_current_program(
    conn: &rusqlite::Connection,
    policy: &Path,
) -> anyhow::Result<crate::schema::Program> {
    let policy_doc = crate::policy::load_policy(policy)?;
    let program_slug = policy_doc
        .current_program
        .as_deref()
        .context("no current program set; run `ldgr-research program set-current <slug>`")?;
    require_program(conn, program_slug)
}

fn require_matrix_in_current_program(
    conn: &rusqlite::Connection,
    policy: &Path,
    slug: &str,
) -> anyhow::Result<(crate::schema::Program, crate::schema::ResearchMatrix)> {
    let program = require_current_program(conn, policy)?;
    let matrix = crate::db::get_research_matrix_by_slug(conn, program.id, slug)?
        .with_context(|| format!("matrix `{slug}` not found in current program"))?;
    Ok((program, matrix))
}

fn require_branch(
    conn: &rusqlite::Connection,
    program_id: i64,
    slug: &str,
) -> anyhow::Result<crate::schema::Branch> {
    crate::db::get_branch_by_slug(conn, program_id, slug)?
        .with_context(|| format!("branch `{slug}` not found in current program"))
}

fn require_question(
    conn: &rusqlite::Connection,
    program_id: i64,
    slug: &str,
) -> anyhow::Result<crate::schema::OpenQuestion> {
    crate::db::get_open_question_by_slug(conn, program_id, slug)?
        .with_context(|| format!("question `{slug}` not found in program"))
}

fn require_question_in_current_program(
    conn: &rusqlite::Connection,
    policy: &Path,
    slug: &str,
) -> anyhow::Result<crate::schema::OpenQuestion> {
    let policy_doc = crate::policy::load_policy(policy)?;
    let program_slug = policy_doc
        .current_program
        .as_deref()
        .context("no current program set; run `ldgr-research program set-current <slug>`")?;
    let program = require_program(conn, program_slug)?;
    require_question(conn, program.id, slug)
}

fn require_option_in_current_program(
    conn: &rusqlite::Connection,
    policy: &Path,
    slug: &str,
) -> anyhow::Result<crate::schema::ResearchOption> {
    let policy_doc = crate::policy::load_policy(policy)?;
    let program_slug = policy_doc
        .current_program
        .as_deref()
        .context("no current program set; run `ldgr-research program set-current <slug>`")?;
    let program = require_program(conn, program_slug)?;
    crate::db::get_research_option_by_slug(conn, program.id, slug)?
        .with_context(|| format!("option `{slug}` not found in program"))
}

fn require_option_in_program(
    conn: &rusqlite::Connection,
    program_id: i64,
    slug: &str,
) -> anyhow::Result<crate::schema::ResearchOption> {
    crate::db::get_research_option_by_slug(conn, program_id, slug)?
        .with_context(|| format!("option `{slug}` not found in program"))
}

fn require_experiment_in_current_branch(
    conn: &rusqlite::Connection,
    policy: &Path,
    slug: &str,
) -> anyhow::Result<crate::schema::Experiment> {
    let (_program, branch) = resolve_current_program_branch(conn, policy, None)?;
    crate::db::get_experiment_by_slug(conn, branch.id, slug)?
        .with_context(|| format!("experiment `{slug}` not found in current branch"))
}

fn resolve_bug_links(
    conn: &rusqlite::Connection,
    policy: &Path,
    program_slug: &Option<String>,
    branch_slug: &Option<String>,
    experiment_slug: &Option<String>,
) -> anyhow::Result<(Option<i64>, Option<i64>, Option<i64>)> {
    let policy_doc = crate::policy::load_policy(policy)?;
    let program = match program_slug
        .as_deref()
        .or(policy_doc.current_program.as_deref())
    {
        Some(slug) => Some(require_program(conn, slug)?),
        None => None,
    };

    let branch = match (program.as_ref(), branch_slug.as_deref()) {
        (Some(program), Some(slug)) => Some(require_branch(conn, program.id, slug)?),
        (Some(program), None) => match (
            policy_doc.current_program.as_deref(),
            policy_doc.current_branch.as_deref(),
        ) {
            (Some(current_program), Some(current_branch)) if current_program == program.slug => {
                Some(require_branch(conn, program.id, current_branch)?)
            }
            _ => None,
        },
        _ => None,
    };

    let experiment = match (branch.as_ref(), experiment_slug.as_deref()) {
        (Some(branch), Some(slug)) => Some(
            crate::db::get_experiment_by_slug(conn, branch.id, slug)?
                .with_context(|| format!("experiment `{slug}` not found in linked branch"))?,
        ),
        (None, Some(_)) => bail!("bug report --experiment requires a resolvable branch"),
        _ => None,
    };

    Ok((
        program.as_ref().map(|program| program.id),
        branch.as_ref().map(|branch| branch.id),
        experiment.as_ref().map(|experiment| experiment.id),
    ))
}

fn read_log_excerpt(path: &Path) -> anyhow::Result<String> {
    let mut contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read log file {}", path.display()))?;
    const MAX_LOG_EXCERPT_BYTES: usize = 64 * 1024;
    if contents.len() > MAX_LOG_EXCERPT_BYTES {
        let start = contents
            .char_indices()
            .rev()
            .find(|(index, _)| contents.len() - *index <= MAX_LOG_EXCERPT_BYTES)
            .map(|(index, _)| index)
            .unwrap_or(0);
        contents = format!(
            "[truncated to last {MAX_LOG_EXCERPT_BYTES} bytes]\n{}",
            &contents[start..]
        );
    }
    Ok(contents)
}

type MatrixAxesWithLevels = Vec<(crate::schema::MatrixAxis, Vec<crate::schema::MatrixLevel>)>;

fn matrix_axes_with_levels(
    conn: &rusqlite::Connection,
    matrix_id: i64,
) -> anyhow::Result<MatrixAxesWithLevels> {
    crate::db::list_matrix_axes(conn, matrix_id)?
        .into_iter()
        .map(|axis| {
            let levels = crate::db::list_matrix_levels(conn, axis.id)?;
            Ok((axis, levels))
        })
        .collect()
}

fn instantiate_matrix_cells(conn: &rusqlite::Connection, matrix_id: i64) -> anyhow::Result<usize> {
    let axes = matrix_axes_with_levels(conn, matrix_id)?;
    if axes.is_empty() {
        bail!("matrix instantiate requires at least one axis");
    }
    if let Some((axis, _)) = axes.iter().find(|(_, levels)| levels.is_empty()) {
        bail!(
            "matrix instantiate requires axis `{}` to have at least one level",
            axis.slug
        );
    }

    let combinations = matrix_level_combinations(&axes);
    let mut created = 0;
    for combination in combinations {
        let mut coordinates = BTreeMap::new();
        let mut level_ids_by_axis = Vec::new();
        let mut slug_parts = Vec::new();
        let mut title_parts = Vec::new();
        for (axis, level) in combination {
            coordinates.insert(axis.slug.clone(), level.slug.clone());
            level_ids_by_axis.push((axis.id, level.id));
            slug_parts.push(format!("{}-{}", axis.slug, level.slug));
            title_parts.push(format!("{}={}", axis.title, level.title));
        }
        let coordinates_json = serde_json::to_string(&coordinates)?;
        if cell_exists_by_coordinates(conn, matrix_id, &coordinates_json)? {
            continue;
        }
        let slug = slug_parts.join("__");
        let title = title_parts.join(", ");
        crate::db::create_matrix_cell(
            conn,
            &crate::schema::NewMatrixCell {
                matrix_id,
                slug: &slug,
                title: &title,
                coordinates_json: &coordinates_json,
                level_ids_by_axis: &level_ids_by_axis,
            },
        )?;
        created += 1;
    }
    Ok(created)
}

fn matrix_level_combinations(
    axes: &[(crate::schema::MatrixAxis, Vec<crate::schema::MatrixLevel>)],
) -> Vec<Vec<(&crate::schema::MatrixAxis, &crate::schema::MatrixLevel)>> {
    let mut combinations: Vec<Vec<(&crate::schema::MatrixAxis, &crate::schema::MatrixLevel)>> =
        vec![Vec::new()];
    for (axis, levels) in axes {
        let mut next = Vec::new();
        for combination in &combinations {
            for level in levels {
                let mut expanded = combination.clone();
                expanded.push((axis, level));
                next.push(expanded);
            }
        }
        combinations = next;
    }
    combinations
}

fn cell_exists_by_coordinates(
    conn: &rusqlite::Connection,
    matrix_id: i64,
    coordinates_json: &str,
) -> anyhow::Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM matrix_cell WHERE matrix_id = ?1 AND coordinates_json = ?2",
        rusqlite::params![matrix_id, coordinates_json],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn print_matrix_show(
    matrix: &crate::schema::ResearchMatrix,
    axes: &MatrixAxesWithLevels,
    cells: &[crate::schema::MatrixCell],
) {
    println!("Matrix: {}", matrix.slug);
    println!("title: {}", matrix.title);
    println!("status: {}", matrix.status);
    println!("description: {}", matrix.description);
    println!("axes:");
    if axes.is_empty() {
        println!("none");
    } else {
        for (axis, levels) in axes {
            let level_slugs = levels
                .iter()
                .map(|level| level.slug.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            println!("- {} [{}]", axis.slug, level_slugs);
        }
    }
    println!("cells:");
    if cells.is_empty() {
        println!("none");
    } else {
        for cell in cells {
            println!("- {} [{}] {}", cell.slug, cell.status, cell.title);
        }
    }
}

fn matrix_json(
    program: &crate::schema::Program,
    matrix: &crate::schema::ResearchMatrix,
    axes: &MatrixAxesWithLevels,
    cells: &[crate::schema::MatrixCell],
) -> anyhow::Result<serde_json::Value> {
    let axes_json = axes
        .iter()
        .map(|(axis, levels)| {
            serde_json::json!({
                "slug": axis.slug,
                "title": axis.title,
                "position": axis.position,
                "levels": levels.iter().map(|level| serde_json::json!({
                    "slug": level.slug,
                    "title": level.title,
                    "position": level.position,
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let cells_json = cells
        .iter()
        .map(|cell| -> anyhow::Result<serde_json::Value> {
            Ok(serde_json::json!({
                "slug": cell.slug,
                "title": cell.title,
                "coordinates": serde_json::from_str::<serde_json::Value>(&cell.coordinates_json)?,
                "experiment_id": cell.experiment_id,
                "status": cell.status,
                "notes": cell.notes,
            }))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(serde_json::json!({
        "format": "ldgr-research.matrix.v1",
        "program": program.slug,
        "slug": matrix.slug,
        "title": matrix.title,
        "description": matrix.description,
        "status": matrix.status,
        "axes": axes_json,
        "cells": cells_json,
    }))
}

fn matrix_cell_status_from_experiment(status: &str) -> crate::schema::MatrixCellStatus {
    match status {
        "running" => crate::schema::MatrixCellStatus::Running,
        "completed" | "inconclusive" | "failed" | "superseded" => {
            crate::schema::MatrixCellStatus::Completed
        }
        _ => crate::schema::MatrixCellStatus::Planned,
    }
}

fn print_matrix_comparison(
    conn: &rusqlite::Connection,
    matrix: &crate::schema::ResearchMatrix,
    cells: &[crate::schema::MatrixCell],
    metric_name: Option<&str>,
) -> anyhow::Result<()> {
    println!("Matrix Comparison: {}", matrix.slug);
    match metric_name {
        Some(metric_name) => {
            println!("metric: {metric_name}");
            for cell in cells {
                let value = match cell.experiment_id {
                    Some(experiment_id) => {
                        let metrics = crate::db::list_metrics_by_experiment(conn, experiment_id)?
                            .into_iter()
                            .filter(|metric| metric.name == metric_name)
                            .collect::<Vec<_>>();
                        if metrics.is_empty() {
                            "no metric".to_owned()
                        } else {
                            let sum = metrics.iter().map(|metric| metric.value).sum::<f64>();
                            let mean = sum / metrics.len() as f64;
                            format!("{mean:.6} (n={})", metrics.len())
                        }
                    }
                    None => "no experiment".to_owned(),
                };
                println!("{} [{}] {}", cell.slug, cell.status, value);
            }
        }
        None => {
            for cell in cells {
                println!(
                    "{} [{}] experiment={}",
                    cell.slug,
                    cell.status,
                    cell.experiment_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "none".to_owned())
                );
            }
        }
    }
    Ok(())
}

fn require_fact_in_current_program(
    conn: &rusqlite::Connection,
    policy: &Path,
    slug: &str,
) -> anyhow::Result<crate::schema::Fact> {
    let policy_doc = crate::policy::load_policy(policy)?;
    let program_slug = policy_doc
        .current_program
        .as_deref()
        .context("no current program set; run `ldgr-research program set-current <slug>`")?;
    let program = require_program(conn, program_slug)?;
    crate::db::get_fact_by_slug(conn, program.id, slug)?
        .with_context(|| format!("fact `{slug}` not found in program"))
}

fn require_axiom_in_current_program(
    conn: &rusqlite::Connection,
    policy: &Path,
    slug: &str,
) -> anyhow::Result<crate::schema::Axiom> {
    let policy_doc = crate::policy::load_policy(policy)?;
    let program_slug = policy_doc
        .current_program
        .as_deref()
        .context("no current program set; run `ldgr-research program set-current <slug>`")?;
    let program = require_program(conn, program_slug)?;
    crate::db::get_axiom_by_slug(conn, program.id, slug)?
        .with_context(|| format!("axiom `{slug}` not found in program"))
}

fn resolve_branch_for_program(
    conn: &rusqlite::Connection,
    policy: &Path,
    program_id: i64,
) -> anyhow::Result<Option<crate::schema::Branch>> {
    let policy_doc = crate::policy::load_policy(policy)?;
    let Some(branch_slug) = policy_doc.current_branch.as_deref() else {
        return Ok(None);
    };
    let Some(program_slug) = policy_doc.current_program.as_deref() else {
        return Ok(None);
    };
    let current_program = require_program(conn, program_slug)?;
    if current_program.id != program_id {
        return Ok(None);
    }
    Ok(Some(require_branch(conn, program_id, branch_slug)?))
}

fn parse_id(value: &str, name: &str) -> anyhow::Result<i64> {
    value
        .parse::<i64>()
        .with_context(|| format!("invalid {name} `{value}`"))
}

fn evidence_from_creation_args<'a>(
    conn: &rusqlite::Connection,
    policy: &Path,
    program_id: i64,
    experiment: Option<&str>,
    artifact: Option<&str>,
    report: Option<&'a str>,
    summary: &'a str,
) -> anyhow::Result<crate::schema::NewEvidenceLink<'a>> {
    if experiment.is_none() && artifact.is_none() && report.is_none() {
        bail!("fact add requires --evidence-experiment, --evidence-artifact, or --evidence-report");
    }
    let experiment_id = match experiment {
        Some(value) => Some(resolve_experiment_reference(
            conn, policy, program_id, value,
        )?),
        None => None,
    };
    let artifact_id = match artifact {
        Some(value) => Some(parse_id(value, "artifact id")?),
        None => None,
    };
    let (report_path, report_anchor) = split_report_reference(report);
    Ok(crate::schema::NewEvidenceLink {
        relation: crate::schema::EvidenceRelation::Supports,
        experiment_id,
        run_id: None,
        metric_id: None,
        artifact_id,
        decision_id: None,
        report_path,
        report_anchor,
        summary,
    })
}

fn evidence_from_args<'a>(
    conn: &rusqlite::Connection,
    policy: &Path,
    program_id: i64,
    args: &'a EvidenceAdd,
) -> anyhow::Result<crate::schema::NewEvidenceLink<'a>> {
    let experiment_id = match args.experiment.as_deref() {
        Some(value) => Some(resolve_experiment_reference(
            conn, policy, program_id, value,
        )?),
        None => None,
    };
    let artifact_id = match args.artifact.as_deref() {
        Some(value) => Some(parse_id(value, "artifact id")?),
        None => None,
    };
    let (report_path, report_anchor) = split_report_reference(args.report.as_deref());
    let summary = args.summary.as_deref().unwrap_or("evidence link");
    Ok(crate::schema::NewEvidenceLink {
        relation: args.relation.clone().into_schema(),
        experiment_id,
        run_id: None,
        metric_id: None,
        artifact_id,
        decision_id: None,
        report_path,
        report_anchor,
        summary,
    })
}

fn resolve_experiment_reference(
    conn: &rusqlite::Connection,
    policy: &Path,
    program_id: i64,
    value: &str,
) -> anyhow::Result<i64> {
    if let Ok(id) = value.parse::<i64>() {
        let experiment = crate::db::get_experiment_by_id(conn, id)?;
        let branch = crate::db::get_branch_by_id(conn, experiment.branch_id)?;
        if branch.program_id != program_id {
            bail!("experiment id {id} is not in the same program");
        }
        return Ok(id);
    }
    let branch = resolve_branch_for_program(conn, policy, program_id)?
        .context("experiment slug evidence requires a current branch in the same program")?;
    let experiment = crate::db::get_experiment_by_slug(conn, branch.id, value)?
        .with_context(|| format!("experiment `{value}` not found in current branch"))?;
    Ok(experiment.id)
}

fn split_report_reference(value: Option<&str>) -> (Option<&str>, Option<&str>) {
    match value {
        Some(reference) => match reference.split_once('#') {
            Some((path, anchor)) => (Some(path), Some(anchor)),
            None => (Some(reference), None),
        },
        None => (None, None),
    }
}

fn question_filter_from_args(
    conn: &rusqlite::Connection,
    policy: &Path,
    program: Option<String>,
    branch: Option<String>,
    status: Option<crate::schema::OpenQuestionStatus>,
) -> anyhow::Result<crate::schema::OpenQuestionFilter> {
    let policy_doc = crate::policy::load_policy(policy)?;
    let program_slug = program.or(policy_doc.current_program);
    let program = match program_slug {
        Some(slug) => Some(require_program(conn, &slug)?),
        None => None,
    };
    let branch_id = match (&program, branch) {
        (Some(program), Some(branch_slug)) => {
            Some(require_branch(conn, program.id, &branch_slug)?.id)
        }
        (_, Some(_)) => bail!("--branch requires --program or a current program"),
        _ => None,
    };
    Ok(crate::schema::OpenQuestionFilter {
        program_id: program.map(|program| program.id),
        branch_id,
        status,
    })
}

fn print_question(question: &crate::schema::OpenQuestion) {
    println!("Question: {}", question.slug);
    println!("id: {}", question.id);
    println!("status: {}", question.status);
    println!("question: {}", question.question);
    println!("context: {}", question.context);
    println!("created_at: {}", question.created_at);
    println!("updated_at: {}", question.updated_at);
}

fn print_option(option: &crate::schema::ResearchOption) {
    println!("Option: {}", option.slug);
    println!("id: {}", option.id);
    println!("title: {}", option.title);
    println!("status: {}", option.status);
    println!("classification: {}", option.classification);
    if let Some(hypothesis) = option.hypothesis.as_deref() {
        println!("hypothesis: {hypothesis}");
    }
    println!("description: {}", option.description);
    if let Some(open_question_id) = option.open_question_id {
        println!("open_question_id: {open_question_id}");
    }
    println!("review_state: {}", option.review_state);
    println!("created_at: {}", option.created_at);
    println!("updated_at: {}", option.updated_at);
}

fn print_experiment(experiment: &crate::schema::Experiment) {
    println!("Experiment: {}", experiment.slug);
    println!("id: {}", experiment.id);
    println!("title: {}", experiment.title);
    println!("status: {}", experiment.status);
    println!("mode: {}", experiment.mode);
    if let Some(phase) = experiment.phase.as_deref() {
        println!("phase: {phase}");
    }
    if let Some(hypothesis) = experiment.hypothesis.as_deref() {
        println!("hypothesis: {hypothesis}");
    }
    if let Some(observation_goal) = experiment.observation_goal.as_deref() {
        println!("observation_goal: {observation_goal}");
    }
    if let Some(rationale) = experiment.rationale.as_deref() {
        println!("rationale: {rationale}");
    }
    if let Some(setup) = experiment.setup.as_deref() {
        println!("setup: {setup}");
    }
    println!("primary_metrics_json: {}", experiment.primary_metrics_json);
    println!(
        "secondary_metrics_json: {}",
        experiment.secondary_metrics_json
    );
    if let Some(pass_criteria) = experiment.pass_criteria.as_deref() {
        println!("pass_criteria: {pass_criteria}");
    }
    if let Some(fail_criteria) = experiment.fail_criteria.as_deref() {
        println!("fail_criteria: {fail_criteria}");
    }
    if let Some(allowed_next_steps) = experiment.allowed_next_steps.as_deref() {
        println!("allowed_next_steps: {allowed_next_steps}");
    }
    if let Some(blocked_next_steps) = experiment.blocked_next_steps.as_deref() {
        println!("blocked_next_steps: {blocked_next_steps}");
    }
    println!("created_at: {}", experiment.created_at);
    println!("updated_at: {}", experiment.updated_at);
}

fn join_json_array(values: Vec<String>) -> anyhow::Result<Option<String>> {
    if values.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::to_string(&values)?))
    }
}

fn environment_json(entries: Vec<String>) -> anyhow::Result<String> {
    let mut map = serde_json::Map::new();
    for entry in entries {
        let (key, value) = entry
            .split_once('=')
            .with_context(|| format!("environment entry `{entry}` must be key=value"))?;
        if key.is_empty() {
            bail!("environment key cannot be empty");
        }
        map.insert(key.to_owned(), serde_json::Value::String(value.to_owned()));
    }
    Ok(serde_json::Value::Object(map).to_string())
}

fn validate_artifact_path(path: &Path, allowed_roots: &[String]) -> anyhow::Result<()> {
    let path_text = path.to_string_lossy();
    if allowed_roots.is_empty() || allowed_roots.iter().any(|root| path_text.starts_with(root)) {
        return Ok(());
    }
    bail!(
        "artifact path `{}` is outside allowed roots: {}",
        path.display(),
        allowed_roots.join(", ")
    )
}

fn compute_sha256(path: &Path) -> anyhow::Result<Option<String>> {
    if !path.exists() {
        eprintln!(
            "warning: artifact {} does not exist; checksum skipped",
            path.display()
        );
        return Ok(None);
    }
    let mut file = fs::File::open(path)
        .with_context(|| format!("failed to open artifact {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read artifact {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(Some(format!("{:x}", hasher.finalize())))
}

fn validate_experiment_mode_inputs(
    mode: crate::schema::ExperimentMode,
    hypothesis: Option<&str>,
    fail_criteria: &[String],
    observation_goal: Option<&str>,
    _rationale: Option<&str>,
) -> anyhow::Result<()> {
    match mode {
        crate::schema::ExperimentMode::Falsification => {
            if hypothesis.is_none_or(str::is_empty) {
                bail!("falsification experiments require --hypothesis");
            }
            if fail_criteria.is_empty() {
                bail!("falsification experiments require at least one --fail criterion");
            }
        }
        crate::schema::ExperimentMode::Exploration => {
            if observation_goal.is_none_or(str::is_empty) {
                bail!("exploration experiments require --observation-goal");
            }
        }
    }
    Ok(())
}

fn experiment_update_empty(args: &ExperimentUpdate) -> bool {
    args.title.is_none()
        && args.phase.is_none()
        && args.mode.is_none()
        && args.hypothesis.is_none()
        && args.setup.is_none()
        && args.primary_metrics.is_empty()
        && args.secondary_metrics.is_empty()
        && args.pass_criteria.is_empty()
        && args.fail_criteria.is_empty()
        && args.allowed_next.is_empty()
        && args.blocked_next.is_empty()
        && args.observation_goal.is_none()
        && args.rationale.is_none()
        && args.status.is_none()
}

fn validate_experiment_status_transition(
    current: &str,
    next: crate::schema::ExperimentStatus,
) -> anyhow::Result<()> {
    let allowed = matches!(
        (current, next),
        ("planned", crate::schema::ExperimentStatus::Running)
            | ("planned", crate::schema::ExperimentStatus::Superseded)
            | ("running", crate::schema::ExperimentStatus::Completed)
            | ("running", crate::schema::ExperimentStatus::Failed)
            | ("running", crate::schema::ExperimentStatus::Inconclusive)
            | ("running", crate::schema::ExperimentStatus::Superseded)
    );
    if allowed {
        Ok(())
    } else if current == "planned" && next == crate::schema::ExperimentStatus::Completed {
        bail!(
            "invalid experiment status transition from planned to completed; run `ldgr-research experiment update <slug> --status running` before completing"
        )
    } else {
        bail!("invalid experiment status transition from {current} to {next}")
    }
}

fn validate_experiment_completion(
    conn: &rusqlite::Connection,
    policy: &crate::policy::Policy,
    experiment: &crate::schema::Experiment,
) -> anyhow::Result<()> {
    for field in &policy.required_experiment_fields {
        if experiment_required_field_applies(experiment, field)
            && experiment_required_field_missing(experiment, field)
        {
            bail!(
                "experiment {} cannot complete; required field `{}` is missing",
                experiment.slug,
                field
            );
        }
    }

    if let Some(allowed_next_steps) = experiment.allowed_next_steps.as_deref() {
        for blocked in &policy.blocked_work {
            if !blocked.is_empty() && allowed_next_steps.contains(blocked) {
                bail!(
                    "experiment {} cannot complete; allowed-next matches blocked work `{}`",
                    experiment.slug,
                    blocked
                );
            }
        }
    }

    if policy.required_decision_after_experiment
        && !crate::db::experiment_has_decision(conn, experiment.id)?
    {
        bail!(
            "experiment {} cannot complete; a decision is required before completion",
            experiment.slug
        );
    }

    validate_experiment_status_transition(
        &experiment.status,
        crate::schema::ExperimentStatus::Completed,
    )
}

fn experiment_required_field_applies(experiment: &crate::schema::Experiment, field: &str) -> bool {
    if experiment.mode == "exploration" {
        matches!(field, "mode" | "observation_goal")
    } else {
        true
    }
}

fn experiment_required_field_missing(experiment: &crate::schema::Experiment, field: &str) -> bool {
    match field {
        "mode" => experiment.mode.is_empty(),
        "hypothesis" => option_str_empty(experiment.hypothesis.as_deref()),
        "setup" => option_str_empty(experiment.setup.as_deref()),
        "primary_metrics" => json_array_empty(&experiment.primary_metrics_json),
        "result" | "interpretation" | "limitations" | "decision" => false,
        "allowed_next_steps" => option_str_empty(experiment.allowed_next_steps.as_deref()),
        "blocked_next_steps" => option_str_empty(experiment.blocked_next_steps.as_deref()),
        "observation_goal" => option_str_empty(experiment.observation_goal.as_deref()),
        "rationale" => option_str_empty(experiment.rationale.as_deref()),
        _ => false,
    }
}

fn option_str_empty(value: Option<&str>) -> bool {
    value.is_none_or(str::is_empty)
}

fn json_array_empty(value: &str) -> bool {
    serde_json::from_str::<Vec<serde_json::Value>>(value)
        .map(|values| values.is_empty())
        .unwrap_or(true)
}

fn resolve_current_program_branch(
    conn: &rusqlite::Connection,
    policy: &Path,
    branch_slug: Option<&str>,
) -> anyhow::Result<(crate::schema::Program, crate::schema::Branch)> {
    let policy_doc = crate::policy::load_policy(policy)?;
    let program_slug = policy_doc
        .current_program
        .as_deref()
        .context("no current program set; run `ldgr-research program set-current <slug>`")?;
    let program = require_program(conn, program_slug)?;
    let branch_slug = branch_slug
        .or(policy_doc.current_branch.as_deref())
        .context("no current branch set; run `ldgr-research branch set-current <slug>`")?;
    let branch = require_branch(conn, program.id, branch_slug)?;
    Ok((program, branch))
}

fn parse_enum<T>(value: &str, name: &str) -> anyhow::Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    value
        .parse()
        .map_err(|err| anyhow::anyhow!("invalid {name} `{value}`: {err}"))
}

fn print_list_or_none(items: &[String]) {
    if items.is_empty() {
        println!("none");
    } else {
        for item in items {
            println!("- {item}");
        }
    }
}

#[derive(Debug, Args)]
pub struct ProgramArgs {
    #[command(subcommand)]
    pub command: ProgramCommand,
}

#[derive(Debug, Subcommand)]
pub enum ProgramCommand {
    Create(ProgramCreate),
    List,
    Show(SlugArg),
    SetCurrent(SlugArg),
    Update(ProgramUpdate),
}

#[derive(Debug, Args)]
pub struct ProgramCreate {
    pub slug: String,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub objective: String,
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProgramUpdate {
    pub slug: String,
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct BranchArgs {
    #[command(subcommand)]
    pub command: BranchCommand,
}

#[derive(Debug, Subcommand)]
pub enum BranchCommand {
    Create(BranchCreate),
    List,
    Show(SlugArg),
    SetCurrent(SlugArg),
    Update(BranchUpdate),
}

#[derive(Debug, Args)]
pub struct BranchCreate {
    pub slug: String,
    #[arg(long)]
    pub program: String,
    #[arg(long)]
    pub parent: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub question: String,
    #[arg(long)]
    pub rationale: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct BranchUpdate {
    pub slug: String,
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct ExperimentArgs {
    #[command(subcommand)]
    pub command: ExperimentCommand,
}

#[derive(Debug, Subcommand)]
pub enum ExperimentCommand {
    Create(ExperimentCreate),
    List(ExperimentList),
    Show(SlugArg),
    Submit(ExperimentSubmit),
    Update(ExperimentUpdate),
    Complete(SlugArg),
}

#[derive(Debug, Args)]
pub struct ExperimentCreate {
    pub slug: String,
    #[arg(long)]
    pub branch: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub phase: Option<String>,
    #[arg(long, value_enum)]
    pub mode: ExperimentMode,
    #[arg(long)]
    pub hypothesis: Option<String>,
    #[arg(long)]
    pub setup: Option<String>,
    #[arg(long = "primary-metric")]
    pub primary_metrics: Vec<String>,
    #[arg(long = "secondary-metric")]
    pub secondary_metrics: Vec<String>,
    #[arg(long = "pass")]
    pub pass_criteria: Vec<String>,
    #[arg(long = "fail")]
    pub fail_criteria: Vec<String>,
    #[arg(long = "allowed-next")]
    pub allowed_next: Vec<String>,
    #[arg(long = "blocked-next")]
    pub blocked_next: Vec<String>,
    #[arg(long = "observation-goal")]
    pub observation_goal: Option<String>,
    #[arg(long)]
    pub rationale: Option<String>,
    #[arg(long)]
    pub option: Option<String>,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ExperimentMode {
    Falsification,
    Exploration,
}

impl ExperimentMode {
    fn into_schema(self) -> crate::schema::ExperimentMode {
        match self {
            Self::Falsification => crate::schema::ExperimentMode::Falsification,
            Self::Exploration => crate::schema::ExperimentMode::Exploration,
        }
    }
}

#[derive(Debug, Args)]
pub struct ExperimentList {
    #[arg(long)]
    pub branch: Option<String>,
}

#[derive(Debug, Args)]
pub struct ExperimentSubmit {
    pub slug: String,
    #[arg(long)]
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct ExperimentUpdate {
    pub slug: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub phase: Option<String>,
    #[arg(long, value_enum)]
    pub mode: Option<ExperimentMode>,
    #[arg(long)]
    pub hypothesis: Option<String>,
    #[arg(long)]
    pub setup: Option<String>,
    #[arg(long = "primary-metric")]
    pub primary_metrics: Vec<String>,
    #[arg(long = "secondary-metric")]
    pub secondary_metrics: Vec<String>,
    #[arg(long = "pass")]
    pub pass_criteria: Vec<String>,
    #[arg(long = "fail")]
    pub fail_criteria: Vec<String>,
    #[arg(long = "allowed-next")]
    pub allowed_next: Vec<String>,
    #[arg(long = "blocked-next")]
    pub blocked_next: Vec<String>,
    #[arg(long = "observation-goal")]
    pub observation_goal: Option<String>,
    #[arg(long)]
    pub rationale: Option<String>,
    #[arg(long, value_enum)]
    pub status: Option<ExperimentStatus>,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ExperimentStatus {
    Planned,
    Running,
    Completed,
    Inconclusive,
    Failed,
    Superseded,
}

impl ExperimentStatus {
    fn into_schema(self) -> crate::schema::ExperimentStatus {
        match self {
            Self::Planned => crate::schema::ExperimentStatus::Planned,
            Self::Running => crate::schema::ExperimentStatus::Running,
            Self::Completed => crate::schema::ExperimentStatus::Completed,
            Self::Inconclusive => crate::schema::ExperimentStatus::Inconclusive,
            Self::Failed => crate::schema::ExperimentStatus::Failed,
            Self::Superseded => crate::schema::ExperimentStatus::Superseded,
        }
    }
}

#[derive(Debug, Args)]
pub struct RunArgs {
    #[command(subcommand)]
    pub command: RunCommand,
}

#[derive(Debug, Subcommand)]
pub enum RunCommand {
    Start(RunStart),
    Finish(RunFinish),
    Fail(RunFail),
    List(RunList),
}

#[derive(Debug, Args)]
pub struct RunStart {
    pub experiment: String,
    #[arg(long)]
    pub command: Option<String>,
    #[arg(long)]
    pub dataset: Option<String>,
    #[arg(long = "code-ref")]
    pub code_ref: Option<String>,
    #[arg(
        long = "env",
        value_name = "KEY=VALUE",
        help = "Environment entry to store with the run; repeatable"
    )]
    pub env: Vec<String>,
    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(Debug, Args)]
pub struct RunFinish {
    pub run_id: String,
    #[arg(long, value_enum)]
    pub status: RunStatus,
    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum RunStatus {
    Running,
    Success,
    Failed,
    Partial,
}

impl RunStatus {
    fn into_schema(self) -> crate::schema::RunStatus {
        match self {
            Self::Running => crate::schema::RunStatus::Running,
            Self::Success => crate::schema::RunStatus::Success,
            Self::Failed => crate::schema::RunStatus::Failed,
            Self::Partial => crate::schema::RunStatus::Partial,
        }
    }
}

#[derive(Debug, Args)]
pub struct RunFail {
    pub run_id: String,
    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(Debug, Args)]
pub struct RunList {
    #[arg(long)]
    pub experiment: Option<String>,
}

#[derive(Debug, Args)]
pub struct MetricArgs {
    #[command(subcommand)]
    pub command: MetricCommand,
}

#[derive(Debug, Subcommand)]
pub enum MetricCommand {
    Add(MetricAdd),
    Trend(MetricTrend),
    List(MetricList),
}

#[derive(Debug, Args)]
pub struct MetricAdd {
    pub run_id: String,
    pub name: String,
    pub value: f64,
    #[arg(long)]
    pub unit: Option<String>,
    #[arg(long)]
    pub split: Option<String>,
    #[arg(long = "higher-is-better")]
    pub higher_is_better: Option<bool>,
}

#[derive(Debug, Args)]
pub struct MetricTrend {
    pub name: String,
    #[arg(long = "by")]
    pub group_by: Option<String>,
}

#[derive(Debug, Args)]
pub struct MetricList {
    #[arg(long)]
    pub experiment: Option<String>,
}

#[derive(Debug, Args)]
pub struct ArtifactArgs {
    #[command(subcommand)]
    pub command: ArtifactCommand,
}

#[derive(Debug, Subcommand)]
pub enum ArtifactCommand {
    Add(ArtifactAdd),
    List(ArtifactList),
}

#[derive(Debug, Args)]
pub struct ArtifactAdd {
    pub run_id: String,
    /// Artifact path. Starter policy allows paths under output/, docs/, or experiments/.
    pub path: PathBuf,
    #[arg(long, value_enum)]
    pub kind: ArtifactKind,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub checksum: bool,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ArtifactKind {
    Json,
    Csv,
    Audio,
    Image,
    Report,
    Model,
    Npz,
    Midi,
    Other,
}

impl ArtifactKind {
    fn into_schema(self) -> crate::schema::ArtifactKind {
        match self {
            Self::Json => crate::schema::ArtifactKind::Json,
            Self::Csv => crate::schema::ArtifactKind::Csv,
            Self::Audio => crate::schema::ArtifactKind::Audio,
            Self::Image => crate::schema::ArtifactKind::Image,
            Self::Report => crate::schema::ArtifactKind::Report,
            Self::Model => crate::schema::ArtifactKind::Model,
            Self::Npz => crate::schema::ArtifactKind::Npz,
            Self::Midi => crate::schema::ArtifactKind::Midi,
            Self::Other => crate::schema::ArtifactKind::Other,
        }
    }
}

#[derive(Debug, Args)]
pub struct ArtifactList {
    #[arg(long)]
    pub experiment: Option<String>,
}

#[derive(Debug, Args)]
pub struct DecisionArgs {
    #[command(subcommand)]
    pub command: DecisionCommand,
}

#[derive(Debug, Subcommand)]
pub enum DecisionCommand {
    Add(DecisionAdd),
}

#[derive(Debug, Args)]
pub struct DecisionAdd {
    pub experiment: String,
    #[arg(long, value_enum)]
    pub decision: DecisionKind,
    #[arg(long, value_enum)]
    pub confidence: Confidence,
    #[arg(long)]
    pub result: String,
    #[arg(long)]
    pub interpretation: String,
    #[arg(long)]
    pub limitations: String,
    #[arg(long = "next-branch")]
    pub next_branch: Option<String>,
    #[arg(long = "next-experiment")]
    pub next_experiment: Option<String>,
    #[arg(
        long = "propose-option",
        value_name = "SLUG[@CLASSIFICATION]:DESCRIPTION",
        help = "Create a follow-up option from this decision; examples: next-try:Do the next try, metric-check@validation:Validate the metric"
    )]
    pub propose_options: Vec<String>,
    #[arg(long = "next")]
    pub next_steps: Vec<String>,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum DecisionKind {
    Continue,
    Branch,
    Revise,
    Stop,
    Inconclusive,
}

impl DecisionKind {
    fn into_schema(self) -> crate::schema::DecisionKind {
        match self {
            Self::Continue => crate::schema::DecisionKind::Continue,
            Self::Branch => crate::schema::DecisionKind::Branch,
            Self::Revise => crate::schema::DecisionKind::Revise,
            Self::Stop => crate::schema::DecisionKind::Stop,
            Self::Inconclusive => crate::schema::DecisionKind::Inconclusive,
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    fn into_schema(self) -> crate::schema::Confidence {
        match self {
            Self::Low => crate::schema::Confidence::Low,
            Self::Medium => crate::schema::Confidence::Medium,
            Self::High => crate::schema::Confidence::High,
        }
    }
}

#[derive(Debug, Args)]
pub struct QuestionArgs {
    #[command(subcommand)]
    pub command: QuestionCommand,
}

#[derive(Debug, Subcommand)]
pub enum QuestionCommand {
    Add(QuestionAdd),
    List(QuestionList),
    Show(SlugArg),
    Update(QuestionUpdate),
    Answer(QuestionAnswer),
    Reject(SlugArg),
    Supersede(SlugArg),
}

#[derive(Debug, Args)]
pub struct QuestionAdd {
    pub slug: String,
    #[arg(long)]
    pub program: String,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long)]
    pub question: String,
    #[arg(long)]
    pub context: Option<String>,
}

#[derive(Debug, Args)]
pub struct QuestionList {
    #[arg(long)]
    pub program: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
}

#[derive(Debug, Args)]
pub struct QuestionUpdate {
    pub slug: String,
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct QuestionAnswer {
    pub slug: String,
    #[arg(long)]
    pub summary: String,
}

#[derive(Debug, Args)]
pub struct OptionArgs {
    #[command(subcommand)]
    pub command: OptionCommand,
}

#[derive(Debug, Subcommand)]
pub enum OptionCommand {
    Add(OptionAdd),
    List(OptionList),
    Show(SlugArg),
    Select(OptionSelect),
    Update(OptionUpdate),
    Reject(SlugArg),
    Supersede(SlugArg),
}

#[derive(Debug, Args)]
pub struct OptionAdd {
    pub slug: String,
    #[arg(long)]
    pub program: String,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long = "question")]
    pub open_question: Option<String>,
    #[arg(long, value_enum)]
    pub classification: OptionClassification,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub hypothesis: Option<String>,
    #[arg(long)]
    pub description: String,
}

#[derive(Debug, Args)]
pub struct OptionList {
    #[arg(long, value_enum)]
    pub classification: Option<OptionClassification>,
    #[arg(long, value_enum)]
    pub status: Option<OptionStatus>,
}

#[derive(Debug, Args)]
pub struct OptionSelect {
    pub slug: String,
    #[arg(long)]
    pub by: Option<String>,
    #[arg(long)]
    pub rationale: String,
}

#[derive(Debug, Args)]
pub struct OptionUpdate {
    pub slug: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub hypothesis: Option<Option<String>>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long, value_enum)]
    pub classification: Option<OptionClassification>,
    #[arg(long, value_enum)]
    pub status: Option<OptionStatus>,
}

#[derive(Clone, Debug, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum OptionClassification {
    MainPath,
    Validation,
    Exploratory,
    LongRunning,
    Blocked,
    Maintenance,
}

impl OptionClassification {
    fn into_schema(self) -> crate::schema::ResearchOptionClassification {
        match self {
            Self::MainPath => crate::schema::ResearchOptionClassification::MainPath,
            Self::Validation => crate::schema::ResearchOptionClassification::Validation,
            Self::Exploratory => crate::schema::ResearchOptionClassification::Exploratory,
            Self::LongRunning => crate::schema::ResearchOptionClassification::LongRunning,
            Self::Blocked => crate::schema::ResearchOptionClassification::Blocked,
            Self::Maintenance => crate::schema::ResearchOptionClassification::Maintenance,
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum OptionStatus {
    Open,
    Selected,
    InProgress,
    Answered,
    Rejected,
    Superseded,
}

impl OptionStatus {
    fn into_schema(self) -> crate::schema::ResearchOptionStatus {
        match self {
            Self::Open => crate::schema::ResearchOptionStatus::Open,
            Self::Selected => crate::schema::ResearchOptionStatus::Selected,
            Self::InProgress => crate::schema::ResearchOptionStatus::InProgress,
            Self::Answered => crate::schema::ResearchOptionStatus::Answered,
            Self::Rejected => crate::schema::ResearchOptionStatus::Rejected,
            Self::Superseded => crate::schema::ResearchOptionStatus::Superseded,
        }
    }
}

#[derive(Debug, Args)]
pub struct MatrixArgs {
    #[command(subcommand)]
    pub command: MatrixCommand,
}

#[derive(Debug, Subcommand)]
pub enum MatrixCommand {
    /// Create a durable evaluation matrix.
    Create(MatrixCreate),
    /// List matrices for the current program.
    List,
    /// Show a matrix with axes, levels, and cells.
    Show(MatrixShow),
    /// Update matrix metadata.
    Update(MatrixUpdate),
    /// Manage matrix axes.
    Axis(MatrixAxisArgs),
    /// Manage matrix axis levels.
    Level(MatrixLevelArgs),
    /// Instantiate missing cells from the axis-level Cartesian product.
    Instantiate(SlugArg),
    /// Manage matrix cells.
    Cell(MatrixCellArgs),
    /// Compare cell status or a metric across linked experiments.
    Compare(MatrixCompare),
}

#[derive(Debug, Args)]
pub struct MatrixCreate {
    pub slug: String,
    #[arg(long)]
    pub program: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct MatrixShow {
    pub slug: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct MatrixUpdate {
    pub slug: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long, value_enum)]
    pub status: Option<MatrixStatusArg>,
}

#[derive(Debug, Args)]
pub struct MatrixAxisArgs {
    #[command(subcommand)]
    pub command: MatrixAxisCommand,
}

#[derive(Debug, Subcommand)]
pub enum MatrixAxisCommand {
    Add(MatrixAxisAdd),
    List(MatrixScopedArg),
}

#[derive(Debug, Args)]
pub struct MatrixAxisAdd {
    pub matrix: String,
    pub slug: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub position: Option<i64>,
}

#[derive(Debug, Args)]
pub struct MatrixLevelArgs {
    #[command(subcommand)]
    pub command: MatrixLevelCommand,
}

#[derive(Debug, Subcommand)]
pub enum MatrixLevelCommand {
    Add(MatrixLevelAdd),
    List(MatrixAxisScopedArg),
}

#[derive(Debug, Args)]
pub struct MatrixLevelAdd {
    pub matrix: String,
    pub axis: String,
    pub slug: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub position: Option<i64>,
}

#[derive(Debug, Args)]
pub struct MatrixCellArgs {
    #[command(subcommand)]
    pub command: MatrixCellCommand,
}

#[derive(Debug, Subcommand)]
pub enum MatrixCellCommand {
    List(MatrixScopedArg),
    Link(MatrixCellLink),
    Mark(MatrixCellMark),
}

#[derive(Debug, Args)]
pub struct MatrixScopedArg {
    pub matrix: String,
}

#[derive(Debug, Args)]
pub struct MatrixAxisScopedArg {
    pub matrix: String,
    pub axis: String,
}

#[derive(Debug, Args)]
pub struct MatrixCellLink {
    pub matrix: String,
    pub cell: String,
    #[arg(long)]
    pub experiment: String,
}

#[derive(Debug, Args)]
pub struct MatrixCellMark {
    pub matrix: String,
    pub cell: String,
    #[arg(long, value_enum)]
    pub status: MatrixCellStatusArg,
    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(Debug, Args)]
pub struct MatrixCompare {
    pub slug: String,
    #[arg(long)]
    pub metric: Option<String>,
}

#[derive(Clone, Debug, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum MatrixCellStatusArg {
    Planned,
    Running,
    Completed,
    Blocked,
    Skipped,
}

#[derive(Clone, Debug, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum MatrixStatusArg {
    Active,
    Complete,
    Archived,
}

impl MatrixStatusArg {
    fn into_schema(self) -> crate::schema::MatrixStatus {
        match self {
            Self::Active => crate::schema::MatrixStatus::Active,
            Self::Complete => crate::schema::MatrixStatus::Complete,
            Self::Archived => crate::schema::MatrixStatus::Archived,
        }
    }
}

impl MatrixCellStatusArg {
    fn into_schema(self) -> crate::schema::MatrixCellStatus {
        match self {
            Self::Planned => crate::schema::MatrixCellStatus::Planned,
            Self::Running => crate::schema::MatrixCellStatus::Running,
            Self::Completed => crate::schema::MatrixCellStatus::Completed,
            Self::Blocked => crate::schema::MatrixCellStatus::Blocked,
            Self::Skipped => crate::schema::MatrixCellStatus::Skipped,
        }
    }
}

#[derive(Debug, Args)]
pub struct FactArgs {
    #[command(subcommand)]
    pub command: FactCommand,
}

#[derive(Debug, Subcommand)]
pub enum FactCommand {
    Add(FactAdd),
    List,
    Show(SlugArg),
    Update(FactUpdate),
    Evidence(FactEvidenceArgs),
}

#[derive(Debug, Args)]
pub struct FactAdd {
    pub slug: String,
    #[arg(long)]
    pub program: String,
    #[arg(long)]
    pub statement: String,
    #[arg(long, value_enum)]
    pub status: FactStatus,
    #[arg(long = "evidence-experiment")]
    pub evidence_experiment: Option<String>,
    #[arg(long = "evidence-artifact")]
    pub evidence_artifact: Option<String>,
    #[arg(long = "evidence-report")]
    pub evidence_report: Option<String>,
}

#[derive(Debug, Args)]
pub struct FactUpdate {
    pub slug: String,
    #[arg(long, value_enum)]
    pub status: Option<FactStatus>,
    #[arg(long = "reviewed-by")]
    pub reviewed_by: Option<String>,
}

#[derive(Debug, Args)]
pub struct FactEvidenceArgs {
    #[command(subcommand)]
    pub command: FactEvidenceCommand,
}

#[derive(Debug, Subcommand)]
pub enum FactEvidenceCommand {
    Add(EvidenceAdd),
}

#[derive(Clone, Debug, ValueEnum)]
pub enum FactStatus {
    Candidate,
    Accepted,
    Contested,
    Rejected,
    Superseded,
}

impl FactStatus {
    fn into_schema(self) -> crate::schema::FactStatus {
        match self {
            Self::Candidate => crate::schema::FactStatus::Candidate,
            Self::Accepted => crate::schema::FactStatus::Accepted,
            Self::Contested => crate::schema::FactStatus::Contested,
            Self::Rejected => crate::schema::FactStatus::Rejected,
            Self::Superseded => crate::schema::FactStatus::Superseded,
        }
    }
}

#[derive(Debug, Args)]
pub struct AxiomArgs {
    #[command(subcommand)]
    pub command: AxiomCommand,
}

#[derive(Debug, Subcommand)]
pub enum AxiomCommand {
    Add(AxiomAdd),
    List,
    Show(SlugArg),
    Update(AxiomUpdate),
    Evidence(AxiomEvidenceArgs),
}

#[derive(Debug, Args)]
pub struct AxiomAdd {
    pub slug: String,
    #[arg(long)]
    pub program: String,
    #[arg(long)]
    pub statement: String,
    #[arg(long)]
    pub by: Option<String>,
}

#[derive(Debug, Args)]
pub struct AxiomUpdate {
    pub slug: String,
    #[arg(long, value_enum)]
    pub status: Option<AxiomStatus>,
    #[arg(long = "approved-by")]
    pub approved_by: Option<String>,
}

#[derive(Debug, Args)]
pub struct AxiomEvidenceArgs {
    #[command(subcommand)]
    pub command: AxiomEvidenceCommand,
}

#[derive(Debug, Subcommand)]
pub enum AxiomEvidenceCommand {
    Add(EvidenceAdd),
}

#[derive(Clone, Debug, ValueEnum)]
pub enum AxiomStatus {
    Active,
    Validated,
    Contested,
    Retired,
}

impl AxiomStatus {
    fn into_schema(self) -> crate::schema::AxiomStatus {
        match self {
            Self::Active => crate::schema::AxiomStatus::Active,
            Self::Validated => crate::schema::AxiomStatus::Validated,
            Self::Contested => crate::schema::AxiomStatus::Contested,
            Self::Retired => crate::schema::AxiomStatus::Retired,
        }
    }
}

#[derive(Debug, Args)]
pub struct EvidenceAdd {
    pub slug: String,
    #[arg(long, value_enum)]
    pub relation: EvidenceRelation,
    #[arg(long)]
    pub experiment: Option<String>,
    #[arg(long)]
    pub artifact: Option<String>,
    #[arg(long)]
    pub report: Option<String>,
    #[arg(long)]
    pub summary: Option<String>,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum EvidenceRelation {
    Supports,
    Contradicts,
    Refines,
    Supersedes,
}

impl EvidenceRelation {
    fn into_schema(self) -> crate::schema::EvidenceRelation {
        match self {
            Self::Supports => crate::schema::EvidenceRelation::Supports,
            Self::Contradicts => crate::schema::EvidenceRelation::Contradicts,
            Self::Refines => crate::schema::EvidenceRelation::Refines,
            Self::Supersedes => crate::schema::EvidenceRelation::Supersedes,
        }
    }
}

#[derive(Debug, Args)]
pub struct ReviewArgs {
    #[command(subcommand)]
    pub command: ReviewCommand,
}

#[derive(Debug, Subcommand)]
pub enum ReviewCommand {
    List,
    Show(IdArg),
    Mark(ReviewMark),
    Dismiss(ReviewDismiss),
}

#[derive(Debug, Args)]
pub struct ReviewMark {
    pub review_id: String,
    #[arg(long, value_enum)]
    pub state: ReviewState,
    #[arg(long)]
    pub by: String,
    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(Debug, Args)]
pub struct ReviewDismiss {
    pub review_id: String,
    #[arg(long)]
    pub by: Option<String>,
    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(Clone, Debug, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum ReviewState {
    NeedsReview,
    Reviewed,
    Dismissed,
}

#[derive(Debug, Args)]
pub struct OverrideArgs {
    #[command(subcommand)]
    pub command: OverrideCommand,
}

#[derive(Debug, Subcommand)]
pub enum OverrideCommand {
    Request(OverrideRequest),
    List,
    Show(IdArg),
    Approve(OverrideApproval),
    Reject(OverrideApproval),
}

#[derive(Debug, Args)]
pub struct OverrideRequest {
    #[arg(long = "blocked-work")]
    pub blocked_work: String,
    #[arg(long)]
    pub action: String,
    #[arg(long)]
    pub justification: String,
    #[arg(long)]
    pub by: Option<String>,
}

#[derive(Debug, Args)]
pub struct OverrideApproval {
    pub override_id: String,
    #[arg(long)]
    pub by: String,
    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(Debug, Args)]
pub struct BugArgs {
    #[command(subcommand)]
    pub command: BugCommand,
}

#[derive(Debug, Subcommand)]
pub enum BugCommand {
    /// Report a bug or harness issue for later debugging.
    Report(BugReportArgs),
    /// List bug reports.
    List(BugListArgs),
    /// Show one bug report with captured context.
    Show(IdArg),
    /// Mark a bug report triaged, resolved, or dismissed.
    Mark(BugMarkArgs),
}

#[derive(Debug, Args)]
pub struct BugReportArgs {
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub description: String,
    #[arg(long, value_enum, default_value = "medium")]
    pub severity: BugSeverity,
    #[arg(long)]
    pub program: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long)]
    pub experiment: Option<String>,
    #[arg(long = "command")]
    pub command_text: Option<String>,
    #[arg(long)]
    pub error: Option<String>,
    #[arg(long)]
    pub reproduction: Option<String>,
    #[arg(long = "log-file")]
    pub log_file: Option<PathBuf>,
    #[arg(long = "log-excerpt")]
    pub log_excerpt: Option<String>,
    #[arg(long)]
    pub by: Option<String>,
}

#[derive(Debug, Args)]
pub struct BugListArgs {
    #[arg(long, value_enum)]
    pub status: Option<BugStatus>,
}

#[derive(Debug, Args)]
pub struct BugMarkArgs {
    pub bug_id: String,
    #[arg(long, value_enum)]
    pub status: BugStatus,
    #[arg(long)]
    pub by: Option<String>,
    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(Debug, Args)]
pub struct ToolArgs {
    #[command(subcommand)]
    pub command: ToolCommand,
}

#[derive(Debug, Subcommand)]
pub enum ToolCommand {
    /// Create an empty .ldgr/research/tools.yaml registry if missing.
    Init,
    /// List registered reusable research tools.
    List,
    /// Show one registered tool.
    Show(SlugArg),
    /// Validate the tool registry file.
    Check,
}

#[derive(Debug, Args)]
pub struct GraphArgs {
    #[command(subcommand)]
    pub command: GraphCommand,
}

#[derive(Debug, Subcommand)]
pub enum GraphCommand {
    /// Build the derived graph projection from the current ledger.
    Build(GraphOutputArgs),
    /// Show a compact graph membrane view.
    Show(GraphOutputArgs),
    /// Validate graph consistency and obligation satisfaction.
    Validate(GraphOutputArgs),
    /// Recommend the next valid research option from graph and policy state.
    Next(GraphNextArgs),
    /// Propose typed ledger mutations for graph gaps.
    Propose(GraphOutputArgs),
    /// Apply one supported proposal through typed ledger operations.
    Apply(GraphApplyArgs),
}

#[derive(Debug, Args)]
pub struct DashboardArgs {
    #[command(subcommand)]
    pub command: DashboardCommand,
}

#[derive(Debug, Subcommand)]
pub enum DashboardCommand {
    /// Build a static HTML dashboard from the canonical ledger snapshot.
    Build(DashboardBuild),
}

#[derive(Debug, Args)]
pub struct DashboardBuild {
    #[arg(long)]
    pub program: String,
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Debug, Args)]
pub struct GraphOutputArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct GraphNextArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long = "include-long-running")]
    pub include_long_running: bool,
}

#[derive(Debug, Args)]
pub struct GraphApplyArgs {
    pub proposal_id: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct HypothesisArgs {
    #[command(subcommand)]
    pub command: HypothesisCommand,
}

#[derive(Debug, Subcommand)]
pub enum HypothesisCommand {
    /// Generate candidate hypotheses from the current ldgr-research context.
    Generate(HypothesisGenerate),
    /// Critique candidate hypotheses using deterministic reflection criteria.
    Critique(HypothesisFileTransform),
    /// Rank candidate hypotheses using critique and ldgr-research fit.
    Rank(HypothesisFileTransform),
    /// Create a refined candidate from an existing candidate without replacing it.
    Evolve(HypothesisEvolve),
    /// Explicitly accept one candidate into the ldgr-research ledger.
    Accept(HypothesisAccept),
}

#[derive(Debug, Args)]
pub struct HypothesisGenerate {
    #[arg(long)]
    pub goal: String,
    #[arg(long, default_value_t = 3)]
    pub count: usize,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Debug, Args)]
pub struct HypothesisFileTransform {
    #[arg(long)]
    pub file: PathBuf,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Debug, Args)]
pub struct HypothesisEvolve {
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub candidate: String,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Debug, Args)]
pub struct HypothesisAccept {
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub candidate: String,
    #[arg(long)]
    pub program: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long, value_enum, default_value = "validation")]
    pub classification: OptionClassification,
    #[arg(long = "create-experiment")]
    pub create_experiment: bool,
    #[arg(long = "experiment-slug")]
    pub experiment_slug: Option<String>,
}

#[derive(Clone, Debug, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum BugSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl BugSeverity {
    fn into_schema(self) -> crate::schema::BugReportSeverity {
        match self {
            Self::Low => crate::schema::BugReportSeverity::Low,
            Self::Medium => crate::schema::BugReportSeverity::Medium,
            Self::High => crate::schema::BugReportSeverity::High,
            Self::Critical => crate::schema::BugReportSeverity::Critical,
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum BugStatus {
    Open,
    Triaged,
    Resolved,
    Dismissed,
}

impl BugStatus {
    fn into_schema(self) -> crate::schema::BugReportStatus {
        match self {
            Self::Open => crate::schema::BugReportStatus::Open,
            Self::Triaged => crate::schema::BugReportStatus::Triaged,
            Self::Resolved => crate::schema::BugReportStatus::Resolved,
            Self::Dismissed => crate::schema::BugReportStatus::Dismissed,
        }
    }
}

#[derive(Debug, Args)]
pub struct TreeArgs {
    #[arg(long)]
    pub program: Option<String>,
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    #[arg(value_enum)]
    pub entity_type: ShowEntityType,
    pub slug: String,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ShowEntityType {
    Program,
    Branch,
    Experiment,
    Run,
    Option,
    Question,
    Fact,
    Axiom,
    Review,
    Override,
}

#[derive(Debug, Args)]
pub struct ReportArgs {
    #[command(subcommand)]
    pub command: ReportCommand,
}

#[derive(Debug, Subcommand)]
pub enum ReportCommand {
    Branch(ReportTarget),
    Program(ReportTarget),
    Experiment(ReportTarget),
}

#[derive(Debug, Args)]
pub struct ReportTarget {
    pub slug: String,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    #[command(subcommand)]
    pub command: ExportCommand,
}

#[derive(Debug, Subcommand)]
pub enum ExportCommand {
    Markdown(ExportTarget),
    Json(ExportTarget),
}

#[derive(Debug, Args)]
pub struct ExportTarget {
    #[arg(long)]
    pub program: Option<String>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    #[command(subcommand)]
    pub command: ImportCommand,
}

#[derive(Debug, Subcommand)]
pub enum ImportCommand {
    Json(ImportJson),
}

#[derive(Debug, Args)]
pub struct ImportJson {
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct SlugArg {
    pub slug: String,
}

#[derive(Debug, Args)]
pub struct IdArg {
    pub id: String,
}
