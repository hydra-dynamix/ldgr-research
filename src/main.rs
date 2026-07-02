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

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ldgr::store::{
    create_prompt, get_prompt, init_store, open_store, set_prompt_status, update_prompt,
};

const RESEARCH_LOOP_PROMPT_SLUG: &str = "research-loop";
const ADAPTER_INSTALL_DIR: &str = "research";
const RESEARCH_LOOP_PROMPT_ROLE: &str = "research-loop";

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
const RESEARCH_HARNESS_GUIDE: &str = r#"# LDGR Research harness guide

Use `ldgr research <command>` for research adapter workflows after installing the research adapter. Start with `ldgr research --help`, then initialize project research state with `ldgr research init` or `ldgr-research init`.

Research adapter resources installed by `ldgr-research install`:

- adapter bundle: `~/.ldgr/research` by default
- skill: `research-project-setup`
- active core loop prompt: `research-loop` after `ldgr-research init`

Core LDGR commands remain available through `ldgr`. The research adapter owns research-specific programs, branches, options, experiments, facts, metrics, and reports.
"#;

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
    let research_command = first_research_command(&args).map(str::to_owned);
    match args.first().and_then(|arg| arg.to_str()) {
        None | Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some("install") => install_alias(&args[1..]),
        Some("adapter") => adapter_install(&args[1..]),
        Some("profile") => Err(
            "profile commands are not part of ldgr-research. Use `ldgr-research install`, `ldgr-research init`, then `ldgr research <command>`.".to_string(),
        ),
        _ if research_command.as_deref() == Some("init") && !has_help_flag(&args) => {
            research_init_with_adapter_resources()
        }
        _ if research_command.is_some() => cli::run().map_err(|error| format!("{error:#}")),
        Some(_) => pass_through_ldgr(&args),
    }
}

fn install_alias(args: &[OsString]) -> Result<(), String> {
    if has_help_flag(args) {
        print_install_help();
        return Ok(());
    }
    install_adapter_bundle_from_options(args)
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

    install_adapter_bundle_from_options(&args[1..])
}

fn install_adapter_bundle_from_options(args: &[OsString]) -> Result<(), String> {
    let mut install_root = default_adapter_root().join(ADAPTER_INSTALL_DIR);
    let mut print_path = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].to_str() {
            Some("--adapter-root") => {
                install_root = next_path(args, index, "--adapter-root")?.join(ADAPTER_INSTALL_DIR);
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
    install_adapter_harness_resources(&install_root)?;
    if print_path {
        println!("{}", manifest_path.display());
    } else {
        println!(
            "installed LDGR adapter `research`: {}",
            manifest_path.display()
        );
        println!("next: `ldgr research --help` or `ldgr-research init`");
    }
    Ok(())
}

fn research_init_with_adapter_resources() -> Result<(), String> {
    cli::run().map_err(|error| format!("{error:#}"))?;
    let (install_root, from_adapter_path) = init_adapter_install_root();
    let manifest_path = match install_bundle(&install_root) {
        Ok(manifest_path) => Some(manifest_path),
        Err(error) => {
            eprintln!(
                "warning: could not refresh research adapter bundle at {}: {error}",
                install_root.display()
            );
            None
        }
    };
    if manifest_path.is_some() && !from_adapter_path {
        if let Err(error) = install_adapter_harness_resources(&install_root) {
            eprintln!(
                "warning: could not install research harness resources from {}: {error}",
                install_root.display()
            );
        }
    }
    let ldgr_db = env::var_os("LDGR_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".ldgr/ldgr.db"));
    let ldgr_artifact_root = env::var_os("LDGR_ARTIFACT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".ldgr/artifacts"));
    apply_research_prompt(&ldgr_db, &ldgr_artifact_root, &install_root)?;
    if let Some(manifest_path) = manifest_path {
        println!(
            "installed LDGR adapter `research`: {}",
            manifest_path.display()
        );
    }
    println!("activated LDGR research loop prompt {RESEARCH_LOOP_PROMPT_SLUG}");
    println!("next: `ldgr research --help` or `ldgr-research status`");
    Ok(())
}

fn has_help_flag(args: &[OsString]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.to_str(), Some("--help") | Some("-h")))
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
}

fn init_adapter_install_root() -> (PathBuf, bool) {
    if let Some(root) = discover_research_adapter_root_from_env() {
        (root, true)
    } else {
        (default_adapter_root().join(ADAPTER_INSTALL_DIR), false)
    }
}

fn discover_research_adapter_root_from_env() -> Option<PathBuf> {
    let paths = env::var_os("LDGR_ADAPTER_PATH")?;
    for root in env::split_paths(&paths) {
        if is_research_adapter_root(&root) {
            return Some(root);
        }
        let child = root.join(ADAPTER_INSTALL_DIR);
        if is_research_adapter_root(&child) {
            return Some(child);
        }
    }
    None
}

fn is_research_adapter_root(root: &Path) -> bool {
    let manifest = root.join("adapter.toml");
    fs::read_to_string(manifest)
        .map(|text| text.contains("slug = \"research\""))
        .unwrap_or(false)
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

fn install_adapter_harness_resources(install_root: &Path) -> Result<(), String> {
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| {
            "could not determine HOME/USERPROFILE for harness asset install".to_string()
        })?;
    let config = read_ldgr_harness_config(&home);
    let skill_dirs = configured_skill_dirs(&home, &config);
    let skills = install_root.join("skills");
    if skills.is_dir() {
        for dir in &skill_dirs {
            copy_directory_children(&skills, dir)?;
        }
        if !skill_dirs.is_empty() {
            println!(
                "installed research skills to {}",
                skill_dirs
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    write_parented(
        &home.join(".ldgr/research/harness-setup.md"),
        RESEARCH_HARNESS_GUIDE,
    )?;
    Ok(())
}

fn read_ldgr_harness_config(home: &Path) -> Option<serde_json::Value> {
    let text = fs::read_to_string(home.join(".ldgr/config.json")).ok()?;
    serde_json::from_str(&text).ok()
}

fn configured_skill_dirs(home: &Path, config: &Option<serde_json::Value>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(config) = config {
        if let Some(installed) = config.get("installed").and_then(|value| value.as_array()) {
            for harness in installed {
                if let Some(paths) = harness
                    .get("skill_paths")
                    .and_then(|value| value.as_array())
                {
                    dirs.extend(
                        paths
                            .iter()
                            .filter_map(json_path)
                            .map(|path| expand_home_path(home, path)),
                    );
                }
            }
        }
    }
    if dirs.is_empty() {
        dirs.push(home.join(".pi/agent/skills"));
    }
    dedup_paths(dirs)
}

fn json_path(value: &serde_json::Value) -> Option<&str> {
    value.as_str()
}

fn expand_home_path(home: &Path, value: &str) -> PathBuf {
    value
        .strip_prefix("~/")
        .map(|suffix| home.join(suffix))
        .unwrap_or_else(|| PathBuf::from(value))
}

fn dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    for path in paths {
        if !deduped.contains(&path) {
            deduped.push(path);
        }
    }
    deduped
}

fn copy_directory_children(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to)
        .map_err(|error| format!("failed to create {}: {error}", to.display()))?;
    for entry in
        fs::read_dir(from).map_err(|error| format!("failed to read {}: {error}", from.display()))?
    {
        let entry =
            entry.map_err(|error| format!("failed to read {} entry: {error}", from.display()))?;
        let source = entry.path();
        let dest = to.join(entry.file_name());
        if source.is_dir() {
            copy_dir_recursive(&source, &dest)?;
        } else if source.is_file() {
            write_parented(
                &dest,
                &fs::read_to_string(&source)
                    .map_err(|error| format!("failed to read {}: {error}", source.display()))?,
            )?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to)
        .map_err(|error| format!("failed to create {}: {error}", to.display()))?;
    for entry in
        fs::read_dir(from).map_err(|error| format!("failed to read {}: {error}", from.display()))?
    {
        let entry =
            entry.map_err(|error| format!("failed to read {} entry: {error}", from.display()))?;
        let source = entry.path();
        let dest = to.join(entry.file_name());
        if source.is_dir() {
            copy_dir_recursive(&source, &dest)?;
        } else if source.is_file() {
            fs::copy(&source, &dest).map_err(|error| {
                format!(
                    "failed to copy {} to {}: {error}",
                    source.display(),
                    dest.display()
                )
            })?;
        }
    }
    Ok(())
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
            | "agent-guide"
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
        forwarded.insert(2, OsString::from(RESEARCH_LOOP_PROMPT_SLUG));
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
    if get_prompt(&connection, RESEARCH_LOOP_PROMPT_SLUG)
        .map_err(|error| format!("failed to inspect existing prompt: {error:#}"))?
        .is_some()
    {
        update_prompt(
            &connection,
            RESEARCH_LOOP_PROMPT_SLUG,
            LOOP_PROMPT,
            Some(source_path.as_ref()),
            Some("Loop prompt installed by ldgr-research."),
        )
        .map_err(|error| format!("failed to update research prompt: {error:#}"))?;
    } else {
        create_prompt(
            &connection,
            RESEARCH_LOOP_PROMPT_SLUG,
            RESEARCH_LOOP_PROMPT_ROLE,
            LOOP_PROMPT,
            Some(source_path.as_ref()),
            Some("Loop prompt installed by ldgr-research."),
        )
        .map_err(|error| format!("failed to create research prompt: {error:#}"))?;
    }
    set_prompt_status(&connection, RESEARCH_LOOP_PROMPT_SLUG, "active")
        .map_err(|error| format!("failed to activate research prompt: {error:#}"))?;
    Ok(())
}

fn print_help() {
    println!(
        "ldgr-research\n\nUsage:\n  ldgr-research install [OPTIONS]\n  ldgr-research adapter install [OPTIONS]\n  ldgr-research init\n  ldgr research <command> [options]\n  ldgr-research <ldgr-command> [ARGS...]\n\nAgent quickstart:\n  ldgr-research install\n  ldgr research init\n  ldgr research agent-guide\n  ldgr research doctor\n  ldgr research status\n  ldgr research context\n\nCommands:\n  install            Install the research adapter bundle and harness resources.\n  adapter install    Installer entrypoint used by LDGR core adapter install.\n  init               Initialize project research state and activate research-loop.\n  agent-guide        Print a copy-pasteable guide for autonomous agents.\n  <research-command> Run research programs, branches, options, experiments, facts, and reports.\n\nCanonical LDGR surface:\n  After install, prefer `ldgr research <command>` so core owns discovery and dispatch.\n  `ldgr-research <command>` remains available for direct use and core pass-through.\n  `ldgr-research loop run` defaults to --prompt-slug research-loop when no prompt source is supplied.\n\nNo profile step is required. The research adapter is installed with `install` and initialized with `init`."
    );
}

fn print_install_help() {
    println!(
        "ldgr-research install\n\nOptions:\n      --adapter-root <PATH>  Adapter root; installs a research/ child [default: LDGR_HOME or ~/.ldgr]\n      --install-root <PATH>  Exact install directory for the research adapter bundle\n      --print-path           Print the installed adapter.toml path\n  -h, --help                 Print help\n\nInstalls the adapter bundle plus harness resources. After install, use `ldgr research --help`."
    );
}

fn print_adapter_install_help() {
    println!(
        "ldgr-research adapter install\n\nInstaller entrypoint used by `ldgr adapter install`; humans can also use `ldgr-research install`.\n\nOptions:\n      --adapter-root <PATH>  Adapter root; installs a research/ child [default: LDGR_HOME or ~/.ldgr]\n      --install-root <PATH>  Exact install directory for the research adapter bundle\n      --print-path           Print the installed adapter.toml path\n  -h, --help                 Print help"
    );
}
