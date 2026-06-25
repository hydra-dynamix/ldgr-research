mod cli;
mod dashboard;
mod db;
mod graph;
mod guard;
mod hypothesis;
mod migrations;
mod policy;
mod reports;
mod schema;
mod tools;

use serde::Deserialize;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ldgr::manifest_integrity::verify_manifest_digest;
use ldgr::store::{
    create_prompt, get_prompt, init_store, open_store, set_prompt_status, update_prompt,
};

const PROFILE_PROMPT_SLUG: &str = "research-loop";
const PROFILE_PROMPT_ROLE: &str = "research-loop";

const ADAPTER_TOML: &str = include_str!("../adapter.toml");
const LOOP_PROMPT: &str = include_str!("../loop-prompt.md");
const RESEARCH_CAMPAIGN_PROCESS: &str = include_str!("../docs/research-campaign-process.md");
const RESEARCH_LDGR_HANDOFF: &str = include_str!("../docs/research_ldgr_handoff.md");
const CAMPAIGN_CLEANUP_SCRIPT: &str = include_str!("../scripts/campaign_cleanup.sh");
const CAMPAIGN_COLLECT_SCRIPT: &str = include_str!("../scripts/campaign_collect.sh");
const CAMPAIGN_CREATE_SCRIPT: &str = include_str!("../scripts/campaign_create.sh");
const CAMPAIGN_LAUNCH_SCRIPT: &str = include_str!("../scripts/campaign_launch.sh");
const CAMPAIGN_LIB_SCRIPT: &str = include_str!("../scripts/campaign_lib.sh");
const CAMPAIGN_STATUS_SCRIPT: &str = include_str!("../scripts/campaign_status.sh");
const CLAIM_REVIEW: &str = include_str!("../templates/claim-review.md");
const CAMPAIGN_BRANCH: &str = include_str!("../templates/campaign-branch.md");
const CAMPAIGN_COMPARISON: &str = include_str!("../templates/campaign-comparison.md");
const EXPERIMENT_PLAN: &str = include_str!("../templates/experiment-plan.md");
const MILESTONES: &str = include_str!("../templates/milestones.md");
const NEGATIVE_RESULT: &str = include_str!("../templates/negative-result.md");
const RESEARCH_SPEC: &str = include_str!("../templates/research-spec.md");
const SETUP_SKILL: &str = include_str!("../skills/research-project-setup/SKILL.md");
const SETUP_SKILL_TOML: &str = include_str!("../skills/research-project-setup/skill.toml");
const SETUP_PROMPT_ARTIFACT: &str =
    include_str!("../skills/research-project-setup/fragments/prompt-artifact.md");
const SETUP_ADAPTER: &str =
    include_str!("../skills/research-project-setup/fragments/adapter-setup.md");
const SETUP_FIRST_WORK: &str =
    include_str!("../skills/research-project-setup/fragments/first-work.md");
const SETUP_EXAMPLE: &str =
    include_str!("../skills/research-project-setup/examples/setup-summary.md");
const SETUP_VALIDATOR: &str =
    include_str!("../skills/research-project-setup/validators/setup_completeness.md");

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    match args.first().and_then(|arg| arg.to_str()) {
        None | Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some("adapter") => adapter_install(&args[1..]),
        Some("profile") => profile(&args[1..]),
        _ if first_research_command(&args).is_some() => {
            cli::run().map_err(|error| format!("{error:#}"))
        }
        Some(_) => pass_through_ldgr(&args),
    }
}

fn adapter_install(args: &[OsString]) -> Result<(), String> {
    let subcommand = args.first().and_then(|arg| arg.to_str()).ok_or_else(|| {
        "adapter requires a subcommand: `ldgr-research adapter install`".to_string()
    })?;
    if subcommand != "install" {
        return Err(format!(
            "unknown adapter subcommand `{subcommand}`. Try `ldgr-research adapter install`."
        ));
    }

    let mut install_root = default_adapter_root().join("research");
    let mut print_path = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].to_str() {
            Some("--adapter-root") => {
                install_root = next_path(args, index, "--adapter-root")?.join("research");
                index += 2;
            }
            Some("--install-root") => {
                install_root = next_path(args, index, "--install-root")?;
                index += 2;
            }
            Some("--print-path") => {
                print_path = true;
                index += 1;
            }
            Some("--help") | Some("-h") => {
                print_adapter_install_help();
                return Ok(());
            }
            Some(flag) => return Err(format!("unknown adapter install option `{flag}`")),
            None => return Err("adapter install arguments must be valid UTF-8".to_string()),
        }
    }

    let manifest_path = install_bundle(&install_root)?;
    if print_path {
        println!("{}", manifest_path.display());
    } else {
        println!(
            "installed LDGR adapter `research`: {}",
            manifest_path.display()
        );
        println!(
            "next: `ldgr-research profile discover` then `ldgr-research profile apply research`"
        );
    }
    Ok(())
}

fn profile(args: &[OsString]) -> Result<(), String> {
    let subcommand = args.first().and_then(|arg| arg.to_str()).ok_or_else(|| {
        "profile requires a subcommand: `ldgr-research profile discover` or `ldgr-research profile apply`".to_string()
    })?;
    match subcommand {
        "discover" => profile_discover(&args[1..]),
        "apply" => profile_apply(&args[1..]),
        _ => Err(format!(
            "unknown profile subcommand `{subcommand}`. Try `ldgr-research profile discover` or `ldgr-research profile apply`."
        )),
    }
}

fn profile_discover(args: &[OsString]) -> Result<(), String> {
    if args
        .iter()
        .any(|arg| matches!(arg.to_str(), Some("--help") | Some("-h")))
    {
        print_profile_discover_help();
        return Ok(());
    }
    if let Some(flag) = args.first().and_then(|arg| arg.to_str()) {
        return Err(format!("unknown profile discover option `{flag}`"));
    }

    let manifests = discover_adapter_manifests()?;
    if manifests.is_empty() {
        println!("No adapter manifests discovered.");
        return Ok(());
    }
    for manifest in manifests {
        let aliases = if manifest.aliases.is_empty() {
            String::new()
        } else {
            format!(" aliases={}", manifest.aliases.join(","))
        };
        println!(
            "adapter={} title={} core_version={}{} manifest={} apply=\"ldgr-research profile apply {}\"",
            manifest.slug,
            manifest.title,
            manifest.core_version,
            aliases,
            manifest.manifest_path.display(),
            manifest.slug
        );
    }
    Ok(())
}

fn profile_apply(args: &[OsString]) -> Result<(), String> {
    let mut requested_slug: Option<String> = None;
    let mut install_root = default_adapter_root().join("research");
    let mut materialize_only = false;
    let mut ldgr_db = env::var_os("LDGR_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".ldgr/ldgr.db"));
    let mut ldgr_artifact_root = env::var_os("LDGR_ARTIFACT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".ldgr/artifacts"));

    let mut index = 0;
    while index < args.len() {
        match args[index].to_str() {
            Some("--install-root") => {
                install_root = next_path(args, index, "--install-root")?;
                index += 2;
            }
            Some("--ldgr-db") => {
                ldgr_db = next_path(args, index, "--ldgr-db")?;
                index += 2;
            }
            Some("--ldgr-artifact-root") => {
                ldgr_artifact_root = next_path(args, index, "--ldgr-artifact-root")?;
                index += 2;
            }
            Some("--materialize-only") => {
                materialize_only = true;
                index += 1;
            }
            Some("--help") | Some("-h") => {
                print_profile_apply_help();
                return Ok(());
            }
            Some(value) if !value.starts_with('-') && requested_slug.is_none() => {
                requested_slug = Some(value.to_owned());
                index += 1;
            }
            Some(flag) => return Err(format!("unknown profile apply option `{flag}`")),
            None => return Err("profile apply arguments must be valid UTF-8".to_string()),
        }
    }

    if let Some(slug) = requested_slug.as_deref() {
        if slug != "research" {
            return Err(format!(
                "unknown research profile `{slug}`; expected `research`"
            ));
        }
    }

    let manifest_path = install_bundle(&install_root)?;
    println!(
        "installed LDGR adapter `research`: {}",
        manifest_path.display()
    );
    if materialize_only {
        println!("materialized research adapter profile; skipped ledger prompt activation");
        return Ok(());
    }

    apply_research_prompt(&ldgr_db, &ldgr_artifact_root, &install_root)?;
    println!("applied LDGR research profile prompt={PROFILE_PROMPT_SLUG}");
    Ok(())
}

fn next_path(args: &[OsString], index: usize, flag: &str) -> Result<PathBuf, String> {
    args.get(index + 1)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} requires a path"))
}

fn default_adapter_root() -> PathBuf {
    env::var_os("LDGR_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ldgr")
        })
        .join("adapters")
}

#[derive(Debug, Deserialize)]
struct AdapterManifest {
    adapter: ManifestAdapter,
}

#[derive(Debug, Deserialize)]
struct ManifestAdapter {
    slug: String,
    title: String,
    core_version: String,
    #[serde(default)]
    aliases: Vec<String>,
}

#[derive(Debug)]
struct DiscoveredAdapterManifest {
    slug: String,
    title: String,
    core_version: String,
    aliases: Vec<String>,
    manifest_path: PathBuf,
}

fn discover_adapter_manifests() -> Result<Vec<DiscoveredAdapterManifest>, String> {
    let mut discovered = Vec::new();
    for root in adapter_search_roots() {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries {
            let entry =
                entry.map_err(|error| format!("failed to read {}: {error}", root.display()))?;
            let manifest_path = entry.path().join("adapter.toml");
            if !manifest_path.is_file() {
                continue;
            }
            let manifest_text = match fs::read_to_string(&manifest_path) {
                Ok(text) => text,
                Err(error) => {
                    eprintln!(
                        "warning: skipped adapter manifest {}: failed to read: {error}",
                        manifest_path.display()
                    );
                    continue;
                }
            };
            let manifest: AdapterManifest = match toml::from_str(&manifest_text) {
                Ok(manifest) => manifest,
                Err(error) => {
                    eprintln!(
                        "warning: skipped adapter manifest {}: failed to parse: {error}",
                        manifest_path.display()
                    );
                    continue;
                }
            };
            if let Err(error) = verify_manifest_digest(&manifest_text) {
                eprintln!(
                    "warning: skipped adapter manifest {}: failed to verify: {error}",
                    manifest_path.display()
                );
                continue;
            }
            discovered.push(DiscoveredAdapterManifest {
                slug: manifest.adapter.slug,
                title: manifest.adapter.title,
                core_version: manifest.adapter.core_version,
                aliases: manifest.adapter.aliases,
                manifest_path: manifest_path.canonicalize().unwrap_or(manifest_path),
            });
        }
    }
    discovered.sort_by(|left, right| left.slug.cmp(&right.slug));
    discovered.dedup_by(|left, right| left.slug == right.slug);
    Ok(discovered)
}

fn adapter_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(paths) = env::var_os("LDGR_ADAPTER_PATH") {
        roots.extend(env::split_paths(&paths));
    }
    if let Some(home) = env::var_os("LDGR_HOME") {
        roots.push(PathBuf::from(home).join("adapters"));
    }
    if let Some(home) = env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".ldgr/adapters"));
    }
    roots
}

fn install_bundle(install_root: &Path) -> Result<PathBuf, String> {
    write_parented(&install_root.join("adapter.toml"), ADAPTER_TOML)?;
    write_parented(&install_root.join("loop-prompt.md"), LOOP_PROMPT)?;
    write_parented(
        &install_root.join("docs/research-campaign-process.md"),
        RESEARCH_CAMPAIGN_PROCESS,
    )?;
    write_parented(
        &install_root.join("docs/research_ldgr_handoff.md"),
        RESEARCH_LDGR_HANDOFF,
    )?;
    write_executable(
        &install_root.join("scripts/campaign_cleanup.sh"),
        CAMPAIGN_CLEANUP_SCRIPT,
    )?;
    write_executable(
        &install_root.join("scripts/campaign_collect.sh"),
        CAMPAIGN_COLLECT_SCRIPT,
    )?;
    write_executable(
        &install_root.join("scripts/campaign_create.sh"),
        CAMPAIGN_CREATE_SCRIPT,
    )?;
    write_executable(
        &install_root.join("scripts/campaign_launch.sh"),
        CAMPAIGN_LAUNCH_SCRIPT,
    )?;
    write_executable(
        &install_root.join("scripts/campaign_lib.sh"),
        CAMPAIGN_LIB_SCRIPT,
    )?;
    write_executable(
        &install_root.join("scripts/campaign_status.sh"),
        CAMPAIGN_STATUS_SCRIPT,
    )?;
    write_parented(
        &install_root.join("templates/claim-review.md"),
        CLAIM_REVIEW,
    )?;
    write_parented(
        &install_root.join("templates/campaign-branch.md"),
        CAMPAIGN_BRANCH,
    )?;
    write_parented(
        &install_root.join("templates/campaign-comparison.md"),
        CAMPAIGN_COMPARISON,
    )?;
    write_parented(
        &install_root.join("templates/experiment-plan.md"),
        EXPERIMENT_PLAN,
    )?;
    write_parented(&install_root.join("templates/milestones.md"), MILESTONES)?;
    write_parented(
        &install_root.join("templates/negative-result.md"),
        NEGATIVE_RESULT,
    )?;
    write_parented(
        &install_root.join("templates/research-spec.md"),
        RESEARCH_SPEC,
    )?;
    write_parented(
        &install_root.join("skills/research-project-setup/SKILL.md"),
        SETUP_SKILL,
    )?;
    write_parented(
        &install_root.join("skills/research-project-setup/skill.toml"),
        SETUP_SKILL_TOML,
    )?;
    write_parented(
        &install_root.join("skills/research-project-setup/fragments/prompt-artifact.md"),
        SETUP_PROMPT_ARTIFACT,
    )?;
    write_parented(
        &install_root.join("skills/research-project-setup/fragments/adapter-setup.md"),
        SETUP_ADAPTER,
    )?;
    write_parented(
        &install_root.join("skills/research-project-setup/fragments/first-work.md"),
        SETUP_FIRST_WORK,
    )?;
    write_parented(
        &install_root.join("skills/research-project-setup/examples/setup-summary.md"),
        SETUP_EXAMPLE,
    )?;
    write_parented(
        &install_root.join("skills/research-project-setup/validators/setup_completeness.md"),
        SETUP_VALIDATOR,
    )?;
    Ok(install_root.join("adapter.toml"))
}

fn write_parented(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(path, content).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn write_executable(path: &Path, content: &str) -> Result<(), String> {
    write_parented(path, content)?;
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path)
            .map_err(|error| format!("failed to stat {}: {error}", path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .map_err(|error| format!("failed to chmod {}: {error}", path.display()))?;
    }
    Ok(())
}

fn first_research_command(args: &[OsString]) -> Option<&str> {
    let mut index = 0;
    while index < args.len() {
        let value = args[index].to_str()?;
        match value {
            "--db" | "--policy" | "--tools" => index += 2,
            "--enable-graph-reasoning" | "--enable-hypothesis-engine" => index += 1,
            value
                if value.starts_with("--db=")
                    || value.starts_with("--policy=")
                    || value.starts_with("--tools=") =>
            {
                index += 1;
            }
            value if is_research_command(value) => return Some(value),
            value if value.starts_with('-') => return None,
            _ => return None,
        }
    }
    None
}

fn is_research_command(command: &str) -> bool {
    matches!(
        command,
        "init"
            | "context"
            | "program"
            | "branch"
            | "experiment"
            | "run"
            | "metric"
            | "artifact"
            | "decision"
            | "question"
            | "option"
            | "matrix"
            | "fact"
            | "axiom"
            | "review"
            | "override"
            | "bug"
            | "tool"
            | "graph"
            | "dashboard"
            | "hypothesis"
            | "tree"
            | "show"
            | "report"
            | "export"
            | "import"
            | "guard"
            | "lint"
            | "migrate"
            | "doctor"
    )
}

fn pass_through_ldgr(args: &[OsString]) -> Result<(), String> {
    let ldgr_bin = env::var_os("LDGR_BIN").unwrap_or_else(|| OsString::from("ldgr"));
    let forwarded = research_adjusted_ldgr_args(args);
    let status = Command::new(&ldgr_bin)
        .args(&forwarded)
        .status()
        .map_err(|error| {
            format!(
                "failed to run {}: {error}",
                PathBuf::from(&ldgr_bin).display()
            )
        })?;
    std::process::exit(status.code().unwrap_or(1));
}

fn research_adjusted_ldgr_args(args: &[OsString]) -> Vec<OsString> {
    let mut forwarded = args.to_vec();
    if is_loop_run(args) && !has_loop_prompt_source(args) {
        forwarded.insert(2, OsString::from(PROFILE_PROMPT_SLUG));
        forwarded.insert(2, OsString::from("--prompt-slug"));
    }
    forwarded
}

fn is_loop_run(args: &[OsString]) -> bool {
    matches!(args.first().and_then(|arg| arg.to_str()), Some("loop"))
        && matches!(args.get(1).and_then(|arg| arg.to_str()), Some("run"))
}

fn has_loop_prompt_source(args: &[OsString]) -> bool {
    args.iter().any(|arg| match arg.to_str() {
        Some("--prompt") | Some("--prompt-slug") | Some("--bundle") => true,
        Some(value) => {
            value.starts_with("--prompt=")
                || value.starts_with("--prompt-slug=")
                || value.starts_with("--bundle=")
        }
        None => false,
    })
}

fn apply_research_prompt(
    db: &Path,
    artifact_root: &Path,
    install_root: &Path,
) -> Result<(), String> {
    init_store(db, artifact_root)
        .map_err(|error| format!("failed to initialize LDGR store: {error:#}"))?;
    let connection =
        open_store(db).map_err(|error| format!("failed to open LDGR store: {error:#}"))?;
    let prompt_path = install_root.join("loop-prompt.md");
    let source_path = prompt_path.to_string_lossy();
    if get_prompt(&connection, PROFILE_PROMPT_SLUG)
        .map_err(|error| format!("failed to inspect existing prompt: {error:#}"))?
        .is_some()
    {
        update_prompt(
            &connection,
            PROFILE_PROMPT_SLUG,
            LOOP_PROMPT,
            Some(source_path.as_ref()),
            Some("Loop prompt installed by ldgr-research."),
        )
        .map_err(|error| format!("failed to update research prompt: {error:#}"))?;
    } else {
        create_prompt(
            &connection,
            PROFILE_PROMPT_SLUG,
            PROFILE_PROMPT_ROLE,
            LOOP_PROMPT,
            Some(source_path.as_ref()),
            Some("Loop prompt installed by ldgr-research."),
        )
        .map_err(|error| format!("failed to create research prompt: {error:#}"))?;
    }
    set_prompt_status(&connection, PROFILE_PROMPT_SLUG, "active")
        .map_err(|error| format!("failed to activate research prompt: {error:#}"))?;
    Ok(())
}

fn print_help() {
    println!(
        "ldgr-research\n\nUsage:\n  ldgr-research adapter install [OPTIONS]\n  ldgr-research profile discover [OPTIONS]\n  ldgr-research profile apply [research] [OPTIONS]\n  ldgr-research <ldgr-command> [ARGS...]\n\nCommands:\n  adapter install    Install the bundled LDGR research adapter files.\n  profile discover   List installed LDGR adapter manifests.\n  profile apply      Install files and activate the research-loop prompt in an LDGR ledger.\n\nCore LDGR pass-through:\n  Any other command is forwarded to `ldgr`. `ldgr-research loop run` defaults to --prompt-slug research-loop when no prompt source is supplied."
    );
}

fn print_adapter_install_help() {
    println!(
        "ldgr-research adapter install\n\nOptions:\n      --adapter-root <PATH>  Adapter root; installs a research/ child [default: LDGR_HOME/adapters or ~/.ldgr/adapters]\n      --install-root <PATH>  Exact install directory for the research adapter bundle\n      --print-path           Print the installed adapter.toml path\n  -h, --help                 Print help"
    );
}

fn print_profile_apply_help() {
    println!(
        "ldgr-research profile apply\n\nOptions:\n      --install-root <PATH>       Where to copy the bundled adapter files [default: LDGR_HOME/adapters/research or ~/.ldgr/adapters/research]\n      --materialize-only          Copy files without activating the ledger prompt\n      --ldgr-db <PATH>            LDGR database path [default: LDGR_DB or .ldgr/ldgr.db]\n      --ldgr-artifact-root <PATH> LDGR artifact root [default: LDGR_ARTIFACT_ROOT or .ldgr/artifacts]\n  -h, --help                      Print help"
    );
}

fn print_profile_discover_help() {
    println!(
        "ldgr-research profile discover\n\nSearches LDGR_ADAPTER_PATH, LDGR_HOME/adapters, and ~/.ldgr/adapters for <slug>/adapter.toml manifests.\n\nOptions:\n  -h, --help  Print help"
    );
}
