use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

const COMMERCIAL_ENV_KEYS: &[&str] = &[
    "LDGR_LICENSE",
    "LDGR_LICENSE_FILE",
    "LDGR_LICENSE_PATH",
    "LDGR_ENTITLEMENT",
    "LDGR_ENTITLEMENT_FILE",
    "LDGR_ENTITLEMENT_PATH",
    "LDGR_CUSTOMER_ID",
    "LDGR_PRODUCT",
    "LDGR_PRODUCT_FAMILY",
    "LDGR_SUBSCRIPTION",
];

fn research_command() -> anyhow::Result<Command> {
    let mut command = Command::cargo_bin("ldgr-research")?;
    for key in COMMERCIAL_ENV_KEYS {
        command.env_remove(key);
    }
    command.env_remove("LDGR_TELEMETRY");
    Ok(command)
}

fn run_research(cwd: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = research_command()?.current_dir(cwd).args(args).output()?;
    anyhow::ensure!(
        output.status.success(),
        "ldgr-research {} failed\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

fn run_research_with_home(cwd: &Path, home: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = research_command()?
        .current_dir(cwd)
        .env("HOME", home)
        .args(args)
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "ldgr-research {} failed\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

fn enable_sequence_collection(home: &Path) -> anyhow::Result<()> {
    ldgr::telemetry::save_telemetry_consent(
        &home.join(".ldgr"),
        &ldgr::telemetry::TelemetryConsent::current(
            ldgr::telemetry::TelemetryConsentDecision::Enabled,
        ),
    )?;
    Ok(())
}

fn research_sequence_payload_bytes(home: &Path) -> anyhow::Result<Vec<Vec<u8>>> {
    let route = home.join(".ldgr/telemetry-pending/research-workflow/v1");
    if !route.exists() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(route)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    paths.iter().map(|path| Ok(fs::read(path)?)).collect()
}

fn research_sequence_payloads(home: &Path) -> anyhow::Result<Vec<Vec<u16>>> {
    research_sequence_payload_bytes(home)?
        .iter()
        .map(|payload| Ok(serde_json::from_slice::<Vec<u16>>(payload)?))
        .collect()
}

#[test]
fn help_documents_agent_first_adapter_surface() -> anyhow::Result<()> {
    let mut command = research_command()?;
    command.arg("--help");
    command.assert().success().stdout(
        predicate::str::contains("Agent quickstart")
            .and(predicate::str::contains("ldgr research agent-guide"))
            .and(predicate::str::contains("mode"))
            .and(predicate::str::contains("core <command>"))
            .and(predicate::str::contains("Canonical LDGR surface"))
            .and(predicate::str::contains("No profile step is required"))
            .and(predicate::str::contains("profile discover/apply").not()),
    );
    Ok(())
}

#[test]
fn adapter_install_materializes_research_bundle() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let install_root = temp.path().join("research-adapter");

    let mut command = research_command()?;
    command.args([
        "adapter",
        "install",
        "--install-root",
        install_root.to_str().expect("utf-8 temp path"),
    ]);
    command.assert().success().stdout(
        predicate::str::contains("installed LDGR adapter `research`")
            .and(predicate::str::contains("ldgr research --help")),
    );

    assert!(install_root.join("adapter.toml").is_file());
    assert!(install_root.join("loop-prompt.md").is_file());
    assert!(install_root.join("prompts/research-loop.md").is_file());
    assert!(install_root.join("adapter-resources.json").is_file());
    // Adapters no longer ship skills, extensions, or harness commands. The
    // single `ldgr` skill installed by core is the only harness surface.
    assert!(!install_root.join("skills").exists());
    assert!(!install_root.join("extensions").exists());
    assert!(!install_root.join("commands").exists());
    assert!(install_root.join("scripts/campaign_launch.sh").is_file());
    Ok(())
}

#[test]
fn install_alias_installs_harness_resources() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let install_root = temp.path().join("research-adapter");
    let home = temp.path().join("home");

    let mut command = research_command()?;
    command.env("HOME", &home).args([
        "install",
        "--install-root",
        install_root.to_str().expect("utf-8 temp path"),
    ]);
    command
        .assert()
        .success()
        .stdout(predicate::str::contains("installed research prompts"));

    assert!(install_root.join("adapter.toml").is_file());
    assert!(home.join(".ldgr/prompts/research-loop.md").is_file());
    assert!(!home.join(".pi/agent/skills").exists());
    assert!(install_root.join("harness-setup.md").is_file());
    Ok(())
}

#[test]
fn install_alias_uses_codex_paths_when_codex_harness_is_configured() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let install_root = temp.path().join("research-adapter");
    let home = temp.path().join("home");
    fs::create_dir_all(home.join(".ldgr"))?;
    fs::write(
        home.join(".ldgr/config.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "installed": [{
                "harness": "codex",
                "prompt_paths": [home.join(".codex/prompts")],
                "skill_paths": [home.join(".codex/skills")]
            }]
        }))?,
    )?;

    research_command()?
        .env("HOME", &home)
        .args([
            "install",
            "--install-root",
            install_root.to_str().expect("utf-8 temp path"),
        ])
        .assert()
        .success();

    assert!(home.join(".codex/prompts/research-loop.md").is_file());
    assert!(!home.join(".codex/skills").exists());
    assert!(!home.join(".pi/agent/skills").exists());
    Ok(())
}

#[test]
fn install_removes_legacy_research_extensions_without_touching_other_files() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let install_root = temp.path().join("research-adapter");
    let home = temp.path().join("home");
    let pi_extensions = home.join(".pi/agent/extensions");
    let codex_extensions = home.join(".codex/extensions");
    fs::create_dir_all(&pi_extensions)?;
    fs::create_dir_all(&codex_extensions)?;
    fs::write(pi_extensions.join("ldgr-research.ts"), "legacy")?;
    fs::write(codex_extensions.join("ldgr-research.ts"), "legacy")?;
    fs::write(pi_extensions.join("third-party.ts"), "keep")?;

    research_command()?
        .env("HOME", &home)
        .args([
            "install",
            "--install-root",
            install_root.to_str().expect("utf-8 temp path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "removed legacy research extension",
        ));

    assert!(!pi_extensions.join("ldgr-research.ts").exists());
    assert!(!codex_extensions.join("ldgr-research.ts").exists());
    assert!(pi_extensions.join("third-party.ts").is_file());
    Ok(())
}

#[test]
fn init_installs_research_loop_prompt_and_adapter_resources() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    research_command()?
        .current_dir(temp.path())
        .env("HOME", &home)
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "activated LDGR research loop prompt",
        ));

    assert!(temp.path().join(".ldgr/research/research.db").is_file());
    assert!(home.join(".ldgr/prompts/research-loop.md").is_file());
    assert!(home
        .join(".ldgr/adapters/research/harness-setup.md")
        .is_file());
    assert!(!home.join(".pi/agent/skills").exists());
    let connection = rusqlite::Connection::open(temp.path().join(".ldgr/ldgr.db"))?;
    let status: String = connection.query_row(
        "SELECT status FROM prompt WHERE slug = 'research-loop'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(status, "active");
    Ok(())
}

#[test]
fn open_research_adapter_install_does_not_require_commercial_context() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let install_root = temp.path().join("research-open-adapter");

    research_command()?
        .args([
            "adapter",
            "install",
            "--install-root",
            install_root.to_str().expect("utf-8 temp path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "installed LDGR adapter `research`",
        ));

    let manifest = fs::read_to_string(install_root.join("adapter.toml"))?;
    let manifest_lower = manifest.to_ascii_lowercase();
    for forbidden in [
        "commercial_public_key",
        "entitlement_claim",
        "entitlement_schema",
        "product_version_family",
        "version_family_enforcement",
    ] {
        assert!(
            !manifest_lower.contains(forbidden),
            "research manifest contains commercial enforcement marker {forbidden}"
        );
    }
    Ok(())
}
#[test]
fn init_prefers_existing_adapter_path_for_bundle_refresh() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    let adapter_path_root = temp.path().join("adapter-path-root");
    let adapter_root = adapter_path_root.join("research");
    fs::create_dir_all(&adapter_root)?;
    fs::write(
        adapter_root.join("adapter.toml"),
        "[adapter]\nslug = \"research\"\ntitle = \"Research\"\ncore_version = \"0.1\"\n",
    )?;

    research_command()?
        .current_dir(temp.path())
        .env("HOME", &home)
        .env("LDGR_ADAPTER_PATH", &adapter_path_root)
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "installed LDGR adapter `research`: {}",
            adapter_root.join("adapter.toml").display()
        )));

    assert!(adapter_root.join("loop-prompt.md").is_file());
    assert!(adapter_root.join("prompts/research-loop.md").is_file());
    assert!(!home.join(".ldgr/adapters/research/adapter.toml").exists());
    Ok(())
}

#[test]
fn init_preserves_an_existing_current_core_store() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    let db = temp.path().join(".ldgr/ldgr.db");
    let artifact_root = temp.path().join(".ldgr/artifacts");

    ldgr::store::init_store(&db, &artifact_root)?;
    let connection = ldgr::store::open_store(&db)?;
    let schema_version: i64 = connection.query_row(
        "SELECT version FROM schema_version WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(
        schema_version,
        ldgr::store::CURRENT_SCHEMA_VERSION,
        "fixture must use the active Core schema"
    );
    drop(connection);

    let output = research_command()?
        .current_dir(temp.path())
        .env("HOME", &home)
        .arg("init")
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "research init failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("failed to initialize LDGR store"),
        "research initializer rejected the active Core schema: {stderr}"
    );

    let connection = ldgr::store::open_store(&db)?;
    let schema_version: i64 = connection.query_row(
        "SELECT version FROM schema_version WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(
        schema_version,
        ldgr::store::CURRENT_SCHEMA_VERSION,
        "research init must preserve the active Core schema"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_continues_when_adapter_bundle_refresh_is_unwritable() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    let adapter_root = home.join(".ldgr/adapters/research");
    fs::create_dir_all(&adapter_root)?;
    fs::set_permissions(&adapter_root, fs::Permissions::from_mode(0o500))?;

    let output = research_command()?
        .current_dir(temp.path())
        .env("HOME", &home)
        .arg("init")
        .output()?;

    fs::set_permissions(&adapter_root, fs::Permissions::from_mode(0o700))?;
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("activated LDGR research loop prompt"),
        "{stdout}"
    );
    assert!(
        stderr.contains("could not refresh research adapter bundle"),
        "{stderr}"
    );
    assert!(temp.path().join(".ldgr/research/research.db").is_file());
    Ok(())
}

#[test]
fn agent_guide_documents_copy_pasteable_canonical_flow() -> anyhow::Result<()> {
    let mut command = research_command()?;
    command.arg("agent-guide");
    command.assert().success().stdout(
        predicate::str::contains("LDGR Research agent guide")
            .and(predicate::str::contains("ldgr research init"))
            .and(predicate::str::contains("ldgr research doctor"))
            .and(predicate::str::contains("ldgr research program create"))
            .and(predicate::str::contains("ldgr research question add"))
            .and(predicate::str::contains("ldgr research option add"))
            .and(predicate::str::contains("ldgr research experiment create"))
            .and(predicate::str::contains("ldgr research core run close"))
            .and(predicate::str::contains("ldgr research mode disable"))
            .and(predicate::str::contains("profile/discover/apply").not()),
    );
    Ok(())
}

#[test]
fn profile_command_points_agents_to_install_init_dispatch() -> anyhow::Result<()> {
    let mut command = research_command()?;
    command.arg("profile");
    command.assert().failure().stderr(
        predicate::str::contains("profile commands are not part of ldgr-research")
            .and(predicate::str::contains("ldgr-research install"))
            .and(predicate::str::contains("ldgr research <command>")),
    );
    Ok(())
}

#[test]
fn agent_control_surface_supports_fresh_project_flow() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    run_research(temp.path(), &["init"])?;
    let guide = run_research(temp.path(), &["agent-guide"])?;
    assert!(guide.contains("ldgr research init"), "{guide}");
    run_research(temp.path(), &["doctor"])?;
    run_research(temp.path(), &["status"])?;
    run_research(temp.path(), &["context"])?;
    run_research(
        temp.path(),
        &[
            "program",
            "create",
            "agent-smoke",
            "--title",
            "Agent Smoke",
            "--objective",
            "Verify agent-facing commands",
        ],
    )?;
    run_research(temp.path(), &["program", "set-current", "agent-smoke"])?;
    run_research(
        temp.path(),
        &[
            "branch",
            "create",
            "main",
            "--program",
            "agent-smoke",
            "--title",
            "Main",
            "--question",
            "Can an agent use the control surface?",
            "--rationale",
            "Agent usability smoke",
        ],
    )?;
    run_research(temp.path(), &["branch", "set-current", "main"])?;
    run_research(
        temp.path(),
        &[
            "question",
            "add",
            "agent-usability",
            "--program",
            "agent-smoke",
            "--branch",
            "main",
            "--question",
            "Which command should the agent run next?",
        ],
    )?;
    run_research(
        temp.path(),
        &[
            "option",
            "add",
            "next-doctor",
            "--program",
            "agent-smoke",
            "--branch",
            "main",
            "--question",
            "agent-usability",
            "--classification",
            "validation",
            "--description",
            "Run doctor/status/context before changing research state",
        ],
    )?;
    run_research(
        temp.path(),
        &[
            "experiment",
            "create",
            "first-check",
            "--branch",
            "main",
            "--option",
            "next-doctor",
            "--mode",
            "exploration",
            "--title",
            "First check",
            "--hypothesis",
            "Agents should run doctor/status/context before changes",
            "--setup",
            "Run the agent control-surface smoke",
            "--observation-goal",
            "Observe whether the command flow is usable",
        ],
    )?;
    run_research(temp.path(), &["guard"])?;
    run_research(temp.path(), &["lint"])?;
    Ok(())
}

#[test]
fn research_primitives_complete_happy_path_end_to_end() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir_all(temp.path().join("output"))?;
    run_research(temp.path(), &["init"])?;
    run_research(
        temp.path(),
        &[
            "program",
            "create",
            "demo",
            "--title",
            "Demo",
            "--objective",
            "Validate primitives",
        ],
    )?;
    run_research(temp.path(), &["program", "set-current", "demo"])?;
    run_research(
        temp.path(),
        &[
            "branch",
            "create",
            "main",
            "--program",
            "demo",
            "--title",
            "Main",
            "--question",
            "Does it work?",
            "--rationale",
            "E2E",
        ],
    )?;
    run_research(temp.path(), &["branch", "set-current", "main"])?;
    run_research(
        temp.path(),
        &[
            "option",
            "add",
            "hyp-1",
            "--program",
            "demo",
            "--branch",
            "main",
            "--title",
            "Hypothesis one",
            "--description",
            "Exercise the path",
            "--classification",
            "validation",
            "--hypothesis",
            "the primitive path works",
        ],
    )?;
    run_research(
        temp.path(),
        &[
            "option",
            "select",
            "hyp-1",
            "--by",
            "test",
            "--rationale",
            "best next check",
        ],
    )?;
    run_research(
        temp.path(),
        &[
            "experiment",
            "create",
            "exp-1",
            "--branch",
            "main",
            "--option",
            "hyp-1",
            "--mode",
            "falsification",
            "--title",
            "Experiment one",
            "--hypothesis",
            "the CLI can finish the experiment",
            "--setup",
            "temp repo",
            "--primary-metric",
            "exit_code",
            "--pass",
            "exit zero",
            "--fail",
            "nonzero exit",
            "--allowed-next",
            "bounded follow-up only",
            "--blocked-next",
            "broad placeholder work",
        ],
    )?;
    run_research(
        temp.path(),
        &["experiment", "update", "exp-1", "--status", "running"],
    )?;
    let run_output = run_research(temp.path(), &["run", "start", "exp-1", "--command", "true"])?;
    let run_id = run_output
        .split_whitespace()
        .last()
        .expect("run id in output")
        .to_owned();
    fs::write(temp.path().join("output/result.json"), "{\"ok\":true}\n")?;
    run_research(
        temp.path(),
        &[
            "metric",
            "add",
            &run_id,
            "exit_code",
            "0",
            "--unit",
            "code",
            "--split",
            "e2e",
        ],
    )?;
    run_research(
        temp.path(),
        &[
            "artifact",
            "add",
            &run_id,
            "output/result.json",
            "--kind",
            "json",
            "--description",
            "result",
            "--checksum",
        ],
    )?;
    run_research(
        temp.path(),
        &["run", "finish", &run_id, "--status", "success"],
    )?;
    run_research(
        temp.path(),
        &[
            "decision",
            "add",
            "exp-1",
            "--decision",
            "continue",
            "--confidence",
            "high",
            "--result",
            "commands passed",
            "--interpretation",
            "primitive path works",
            "--limitations",
            "smoke only",
        ],
    )?;
    run_research(
        temp.path(),
        &[
            "fact",
            "add",
            "fact-1",
            "--program",
            "demo",
            "--statement",
            "primitive path works",
            "--status",
            "accepted",
            "--evidence-experiment",
            "exp-1",
        ],
    )?;
    run_research(
        temp.path(),
        &[
            "matrix",
            "create",
            "m1",
            "--program",
            "demo",
            "--title",
            "Matrix",
            "--description",
            "E2E",
        ],
    )?;
    run_research(
        temp.path(),
        &["matrix", "axis", "add", "m1", "axis-a", "--title", "Axis A"],
    )?;
    run_research(
        temp.path(),
        &[
            "matrix", "level", "add", "m1", "axis-a", "level-a", "--title", "Level A",
        ],
    )?;
    run_research(temp.path(), &["matrix", "instantiate", "m1"])?;
    run_research(
        temp.path(),
        &[
            "matrix",
            "cell",
            "link",
            "m1",
            "axis-a-level-a",
            "--experiment",
            "exp-1",
        ],
    )?;
    run_research(temp.path(), &["experiment", "complete", "exp-1"])?;
    run_research(
        temp.path(),
        &["branch", "update", "main", "--status", "complete"],
    )?;
    let context = run_research(temp.path(), &["context"])?;
    assert!(context.contains("Hard Facts"), "{context}");
    run_research(temp.path(), &["guard"])?;
    run_research(temp.path(), &["lint"])?;
    run_research(temp.path(), &["doctor"])?;
    run_research(
        temp.path(),
        &["--enable-graph-reasoning", "graph", "validate"],
    )?;
    Ok(())
}

#[test]
fn failed_experiment_queues_completed_negative_research_sequence() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    enable_sequence_collection(&home)?;

    run_research_with_home(temp.path(), &home, &["init"])?;
    run_research_with_home(
        temp.path(),
        &home,
        &[
            "program",
            "create",
            "telemetry-demo",
            "--title",
            "Telemetry Demo",
            "--objective",
            "Validate research telemetry",
        ],
    )?;
    run_research_with_home(
        temp.path(),
        &home,
        &["program", "set-current", "telemetry-demo"],
    )?;
    run_research_with_home(
        temp.path(),
        &home,
        &[
            "branch",
            "create",
            "main",
            "--program",
            "telemetry-demo",
            "--title",
            "Main",
            "--question",
            "Does negative evidence stay useful?",
            "--rationale",
            "Telemetry regression",
        ],
    )?;
    run_research_with_home(temp.path(), &home, &["branch", "set-current", "main"])?;
    run_research_with_home(
        temp.path(),
        &home,
        &[
            "experiment",
            "create",
            "negative-check",
            "--branch",
            "main",
            "--mode",
            "exploration",
            "--title",
            "Negative check",
            "--setup",
            "local telemetry fixture",
            "--observation-goal",
            "Observe failed experiment terminal mapping",
        ],
    )?;
    run_research_with_home(
        temp.path(),
        &home,
        &[
            "experiment",
            "update",
            "negative-check",
            "--status",
            "running",
        ],
    )?;
    assert!(research_sequence_payloads(&home)?.is_empty());

    run_research_with_home(
        temp.path(),
        &home,
        &[
            "experiment",
            "update",
            "negative-check",
            "--status",
            "failed",
        ],
    )?;

    assert_eq!(research_sequence_payloads(&home)?, vec![vec![0, 1, 4]]);
    let raw_payloads = research_sequence_payload_bytes(&home)?;
    assert_eq!(raw_payloads, vec![b"[0,1,4]".to_vec()]);
    let payload_text = std::str::from_utf8(&raw_payloads[0])?;
    for prohibited in [
        "telemetry-demo",
        "Telemetry Demo",
        "Validate research telemetry",
        "Does negative evidence stay useful?",
        "Telemetry regression",
        "negative-check",
        "Negative check",
        "local telemetry fixture",
        "Observe failed experiment terminal mapping",
        "research",
        "experiment",
        "branch",
        "citation",
        "artifact",
        "source",
    ] {
        assert!(
            !payload_text.contains(prohibited),
            "payload leaked `{prohibited}`"
        );
    }
    Ok(())
}

#[test]
fn non_conflicting_core_commands_pass_through_to_ldgr() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let args_log = temp.path().join("args.txt");
    let fake_ldgr = fake_ldgr(temp.path(), Some(&args_log), "passed-through")?;

    let mut command = research_command()?;
    command
        .env("LDGR_BIN", &fake_ldgr)
        .args(["observation", "add", "7", "--body", "evidence"]);
    command
        .assert()
        .success()
        .stdout(predicate::str::contains("passed-through"));

    let args = fs::read_to_string(args_log)?;
    assert_eq!(args.trim(), "observation add 7 --body evidence");
    Ok(())
}

#[test]
fn core_escape_hatch_passes_conflicting_commands_to_ldgr() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let args_log = temp.path().join("args.txt");
    let fake_ldgr = fake_ldgr(temp.path(), Some(&args_log), "core-pass-through")?;

    research_command()?
        .env("LDGR_BIN", &fake_ldgr)
        .args(["core", "run", "close", "7", "--status", "success"])
        .assert()
        .success()
        .stdout(predicate::str::contains("core-pass-through"));

    let args = fs::read_to_string(args_log)?;
    assert_eq!(args.trim(), "run close 7 --status success");
    Ok(())
}

#[test]
fn mode_disable_stops_research_loop_prompt_injection() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let args_log = temp.path().join("args.txt");
    let fake_ldgr = fake_ldgr(temp.path(), Some(&args_log), "loop-pass-through")?;

    run_research(temp.path(), &["init"])?;
    run_research(temp.path(), &["mode", "disable"])?;

    research_command()?
        .current_dir(temp.path())
        .env("LDGR_BIN", &fake_ldgr)
        .args(["loop", "run", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("loop-pass-through"));

    let args = fs::read_to_string(args_log)?;
    assert_eq!(args.trim(), "loop run --dry-run");
    Ok(())
}

#[test]
fn status_includes_core_and_research_sections() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let fake_ldgr = fake_ldgr(temp.path(), None, "core-status-from-fake")?;

    run_research(temp.path(), &["init"])?;
    research_command()?
        .current_dir(temp.path())
        .env("LDGR_BIN", &fake_ldgr)
        .arg("status")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("LDGR Research status")
                .and(predicate::str::contains("Core LDGR status"))
                .and(predicate::str::contains("core-status-from-fake"))
                .and(predicate::str::contains("Research status")),
        );
    Ok(())
}

#[test]
fn loop_run_pass_through_defaults_to_research_prompt_slug() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let args_log = temp.path().join("args.txt");
    let fake_ldgr = fake_ldgr(temp.path(), Some(&args_log), "loop-pass-through")?;

    let mut command = research_command()?;
    command
        .env("LDGR_BIN", &fake_ldgr)
        .args(["loop", "run", "--dry-run"]);
    command
        .assert()
        .success()
        .stdout(predicate::str::contains("loop-pass-through"));

    let args = fs::read_to_string(args_log)?;
    assert_eq!(
        args.trim(),
        "loop run --prompt-slug research-loop --dry-run"
    );
    Ok(())
}

#[test]
fn rerunning_loop_requeues_same_work_after_failed_agent_attempt() -> anyhow::Result<()> {
    use ldgr::store::{
        create_work_item, finish_run, get_work_item_by_slug, init_store, open_store, start_run,
        RunStatus, WorkItemStatus,
    };

    let temp = TempDir::new()?;
    let db = temp.path().join(".ldgr/ldgr.db");
    let artifacts = temp.path().join(".ldgr/artifacts");
    init_store(&db, &artifacts)?;
    let connection = open_store(&db)?;
    create_work_item(&connection, None, "retry-me", "Retry me", "Bounded work")?;
    let failed = start_run(&connection, "retry-me", Some("agentctl"))?;
    finish_run(
        &connection,
        failed.id,
        RunStatus::Failed,
        Some("agent failed before completing work"),
    )?;
    drop(connection);

    let fake_ldgr = fake_ldgr(temp.path(), None, "loop-retried")?;

    research_command()?
        .current_dir(temp.path())
        .env("LDGR_BIN", &fake_ldgr)
        .args(["loop", "run", "--dry-run"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("research loop: retrying retry-me")
                .and(predicate::str::contains(
                    "prior failure evidence remains recorded",
                ))
                .and(predicate::str::contains("loop-retried")),
        );

    let connection = open_store(&db)?;
    let work = get_work_item_by_slug(&connection, "retry-me")?;
    assert_eq!(work.status, WorkItemStatus::Pending);
    Ok(())
}

#[test]
fn loop_run_pass_through_preserves_explicit_prompt_source() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let args_log = temp.path().join("args.txt");
    let fake_ldgr = fake_ldgr(temp.path(), Some(&args_log), "explicit-prompt")?;

    let mut command = research_command()?;
    command
        .env("LDGR_BIN", &fake_ldgr)
        .args(["loop", "run", "--prompt", "custom.md", "--dry-run"]);
    command
        .assert()
        .success()
        .stdout(predicate::str::contains("explicit-prompt"));

    let args = fs::read_to_string(args_log)?;
    assert_eq!(args.trim(), "loop run --prompt custom.md --dry-run");
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

/// Write a stand-in `ldgr` executable for `LDGR_BIN`.
///
/// Windows cannot execute a shell script, so emit a `.cmd` batch file there and
/// a shell script everywhere else. Both append the received arguments to
/// `args_log` when set, then print `stdout_line`.
#[cfg(windows)]
fn fake_ldgr(dir: &Path, args_log: Option<&Path>, stdout_line: &str) -> anyhow::Result<PathBuf> {
    let path = dir.join("fake-ldgr.cmd");
    let mut body = String::from("@echo off\r\n");
    if let Some(log) = args_log {
        // Parenthesized so a trailing digit in %* is not read as a redirect handle.
        body.push_str(&format!("(echo %*)>\"{}\"\r\n", log.display()));
    }
    body.push_str(&format!("echo {stdout_line}\r\n"));
    fs::write(&path, body)?;
    Ok(path)
}

#[cfg(not(windows))]
fn fake_ldgr(dir: &Path, args_log: Option<&Path>, stdout_line: &str) -> anyhow::Result<PathBuf> {
    let path = dir.join("fake-ldgr.sh");
    let mut body = String::from("#!/usr/bin/env bash\n");
    if let Some(log) = args_log {
        body.push_str(&format!("printf '%s\\n' \"$*\" > {}\n", log.display()));
    }
    body.push_str(&format!("echo {stdout_line}\n"));
    fs::write(&path, body)?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

#[test]
fn preregistration_verdicts_review_gate_and_run_notes() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir_all(temp.path().join("output"))?;
    run_research(temp.path(), &["init"])?;
    run_research(
        temp.path(),
        &[
            "program",
            "create",
            "demo",
            "--title",
            "Demo",
            "--objective",
            "Verdict flow",
        ],
    )?;
    run_research(temp.path(), &["program", "set-current", "demo"])?;
    run_research(
        temp.path(),
        &[
            "branch",
            "create",
            "main",
            "--program",
            "demo",
            "--title",
            "Main",
            "--question",
            "Q?",
            "--rationale",
            "R",
        ],
    )?;
    run_research(temp.path(), &["branch", "set-current", "main"])?;

    // a claim to attack
    run_research(
        temp.path(),
        &[
            "fact",
            "add",
            "shaky-claim",
            "--program",
            "demo",
            "--statement",
            "the shaky claim holds",
            "--status",
            "candidate",
            "--evidence-report",
            "docs/origin.md",
        ],
    )?;

    // adversarial experiment with pre-registration hash
    let created = run_research(
        temp.path(),
        &[
            "experiment",
            "create",
            "attack-1",
            "--branch",
            "main",
            "--mode",
            "falsification",
            "--hypothesis",
            "shaky-claim is refutable",
            "--setup",
            "attack it",
            "--primary-metric",
            "refuted",
            "--pass",
            "the claim breaks",
            "--fail",
            "the claim survives",
            "--allowed-next",
            "record refutation",
            "--blocked-next",
            "building on the claim",
            "--attacks-fact",
            "shaky-claim",
        ],
    )?;
    assert!(created.contains("registration_hash:"));
    assert!(created.contains("attacks_fact: shaky-claim"));

    // run notes + negative metric + failed finish via `finish --status failed`
    run_research(
        temp.path(),
        &["experiment", "update", "attack-1", "--status", "running"],
    )?;
    let started = run_research(temp.path(), &["run", "start", "attack-1", "--command", "t"])?;
    let run_id = started.trim().rsplit(' ').next().unwrap().to_string();
    run_research(
        temp.path(),
        &["run", "note", &run_id, "--body", "first incremental note"],
    )?;
    let notes = run_research(temp.path(), &["run", "notes", &run_id])?;
    assert!(notes.contains("first incremental note"));
    run_research(
        temp.path(),
        &["metric", "add", &run_id, "delta", "-0.5", "--unit", "ratio"],
    )?;
    fs::write(temp.path().join("output/att.log"), "log")?;
    run_research(
        temp.path(),
        &[
            "artifact",
            "add",
            &run_id,
            "output/att.log",
            "--kind",
            "log",
        ],
    )?;
    let finished = run_research(
        temp.path(),
        &[
            "run",
            "finish",
            &run_id,
            "--status",
            "failed",
            "--notes",
            "attack run crashed",
        ],
    )?;
    assert!(finished.contains("[failed]"));

    // verdict pass -> attacked fact contested
    let second = run_research(temp.path(), &["run", "start", "attack-1"])?;
    let run2 = second.trim().rsplit(' ').next().unwrap().to_string();
    run_research(
        temp.path(),
        &["run", "finish", &run2, "--status", "success"],
    )?;
    let verdict = run_research(
        temp.path(),
        &[
            "experiment",
            "verdict",
            "attack-1",
            "--outcome",
            "pass",
            "--statement",
            "registered pass criteria met",
        ],
    )?;
    assert!(verdict.contains("recorded verdict `pass`"));
    assert!(verdict.contains("marked CONTESTED"));
    let shown = run_research(temp.path(), &["fact", "show", "shaky-claim"])?;
    assert!(shown.contains("contested"));

    // impact lists the attacker
    let impact = run_research(temp.path(), &["fact", "impact", "shaky-claim"])?;
    assert!(impact.contains("attack-1"));

    // drift detection: mutate the registered surface, verdict must refuse
    run_research(
        temp.path(),
        &[
            "experiment",
            "update",
            "attack-1",
            "--hypothesis",
            "changed after the fact",
        ],
    )?;
    let drift = research_command()?
        .current_dir(temp.path())
        .args(["experiment", "verdict", "attack-1", "--outcome", "pass"])
        .output()?;
    assert!(!drift.status.success());
    assert!(String::from_utf8_lossy(&drift.stderr).contains("drift"));

    // review gate: accepted facts need --reviewed-by when policy demands it
    let policy_path = temp.path().join(".ldgr/research/policy.yaml");
    let policy_text = fs::read_to_string(&policy_path)?;
    fs::write(
        &policy_path,
        policy_text.replace(
            "require_review_for_fact_acceptance: false",
            "require_review_for_fact_acceptance: true",
        ),
    )?;
    let gated = research_command()?
        .current_dir(temp.path())
        .args([
            "fact",
            "add",
            "gated-claim",
            "--program",
            "demo",
            "--statement",
            "needs review",
            "--status",
            "accepted",
            "--evidence-report",
            "docs/origin.md",
        ])
        .output()?;
    assert!(!gated.status.success());
    assert!(String::from_utf8_lossy(&gated.stderr).contains("requires review"));
    let ok = run_research(
        temp.path(),
        &[
            "fact",
            "add",
            "gated-claim",
            "--program",
            "demo",
            "--statement",
            "needs review",
            "--status",
            "accepted",
            "--evidence-report",
            "docs/origin.md",
            "--reviewed-by",
            "adversarial-agent",
        ],
    )?;
    assert!(ok.contains("created fact gated-claim [accepted]"));
    Ok(())
}

#[test]
fn secondary_source_facts_cannot_be_accepted() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir_all(temp.path().join("output"))?;
    run_research(temp.path(), &["init"])?;
    run_research(
        temp.path(),
        &[
            "program",
            "create",
            "demo",
            "--title",
            "D",
            "--objective",
            "Source classes",
        ],
    )?;
    run_research(temp.path(), &["program", "set-current", "demo"])?;

    // secondary evidence (a tool summary / someone else's report) cannot be accepted
    let blocked = research_command()?
        .current_dir(temp.path())
        .args([
            "fact",
            "add",
            "hearsay",
            "--program",
            "demo",
            "--statement",
            "their README says X",
            "--status",
            "accepted",
            "--evidence-report",
            "docs/sweep.md",
            "--source",
            "secondary",
        ])
        .output()?;
    assert!(!blocked.status.success());
    let err = String::from_utf8_lossy(&blocked.stderr);
    assert!(
        err.contains("secondary evidence"),
        "unexpected error: {err}"
    );

    // recording it as a candidate is fine
    let cand = run_research(
        temp.path(),
        &[
            "fact",
            "add",
            "hearsay",
            "--program",
            "demo",
            "--statement",
            "their README says X",
            "--status",
            "candidate",
            "--evidence-report",
            "docs/sweep.md",
            "--source",
            "secondary",
        ],
    )?;
    assert!(cand.contains("source_class: secondary"));

    // promoting it while still secondary is refused
    let promote = research_command()?
        .current_dir(temp.path())
        .args(["fact", "update", "hearsay", "--status", "accepted"])
        .output()?;
    assert!(!promote.status.success());
    assert!(String::from_utf8_lossy(&promote.stderr).contains("secondary evidence"));

    // after reading the primary source, reclassify and promote
    let ok = run_research(
        temp.path(),
        &[
            "fact", "update", "hearsay", "--status", "accepted", "--source", "primary",
        ],
    )?;
    assert!(ok.contains("[accepted]"));
    let shown = run_research(temp.path(), &["fact", "show", "hearsay"])?;
    assert!(shown.contains("source_class: primary"));
    Ok(())
}

#[test]
fn context_lists_all_branches_and_terminal_update_answers_option() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir_all(temp.path().join("output"))?;
    run_research(temp.path(), &["init"])?;
    run_research(
        temp.path(),
        &[
            "program",
            "create",
            "demo",
            "--title",
            "D",
            "--objective",
            "Branch visibility",
        ],
    )?;
    run_research(temp.path(), &["program", "set-current", "demo"])?;
    for (slug, q) in [("main", "main question?"), ("side", "side question?")] {
        run_research(
            temp.path(),
            &[
                "branch",
                "create",
                slug,
                "--program",
                "demo",
                "--title",
                slug,
                "--question",
                q,
                "--rationale",
                "r",
            ],
        )?;
    }
    run_research(temp.path(), &["branch", "set-current", "main"])?;

    // an experiment on the OTHER branch, with a linked option
    run_research(
        temp.path(),
        &[
            "option",
            "add",
            "side-opt",
            "--program",
            "demo",
            "--branch",
            "side",
            "--title",
            "Side option",
            "--description",
            "d",
            "--classification",
            "exploratory",
            "--hypothesis",
            "h",
        ],
    )?;
    run_research(
        temp.path(),
        &[
            "option",
            "select",
            "side-opt",
            "--by",
            "test",
            "--rationale",
            "r",
        ],
    )?;
    run_research(
        temp.path(),
        &[
            "experiment",
            "create",
            "side-exp",
            "--branch",
            "side",
            "--option",
            "side-opt",
            "--mode",
            "exploration",
            "--observation-goal",
            "see something",
            "--setup",
            "s",
            "--primary-metric",
            "m",
        ],
    )?;

    // context on `main` must still reveal that `side` exists, with its question
    let ctx = run_research(temp.path(), &["context"])?;
    assert!(
        ctx.contains("Program Branches"),
        "no branch overview:\n{ctx}"
    );
    assert!(ctx.contains("side"), "other branch not listed:\n{ctx}");
    assert!(
        ctx.contains("side question?"),
        "other branch's question hidden:\n{ctx}"
    );
    assert!(
        ctx.contains("branch set-current side"),
        "no switch hint:\n{ctx}"
    );

    // a terminal status reached via `update` answers the linked option, as `complete` would
    run_research(
        temp.path(),
        &[
            "experiment",
            "update",
            "side-exp",
            "--branch",
            "side",
            "--status",
            "running",
        ],
    )?;
    let out = run_research(
        temp.path(),
        &[
            "experiment",
            "update",
            "side-exp",
            "--branch",
            "side",
            "--status",
            "failed",
        ],
    )?;
    assert!(
        out.contains("answered option side-opt"),
        "option left stale:\n{out}"
    );
    Ok(())
}

/// stderr of a command expected to FAIL, for asserting on guidance messages.
fn run_research_expect_failure(cwd: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = research_command()?.current_dir(cwd).args(args).output()?;
    anyhow::ensure!(
        !output.status.success(),
        "ldgr-research {} unexpectedly succeeded\nstdout:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(String::from_utf8_lossy(&output.stderr).into_owned())
}

#[test]
fn artifact_add_refuses_empty_files_but_allows_an_explicit_override() -> anyhow::Result<()> {
    // An artifact backs a fact, so a zero-byte file is exactly the failure the evidence gate
    // exists to prevent: it registers cleanly and its checksum is the digest of the empty
    // string, so nothing downstream notices that the evidence is vacuous.
    let temp = TempDir::new()?;
    fs::create_dir_all(temp.path().join("output"))?;
    run_research(temp.path(), &["init"])?;
    fs::write(temp.path().join("output").join("empty.txt"), b"")?;

    let stderr = run_research_expect_failure(
        temp.path(),
        &["artifact", "add", "1", "output/empty.txt", "--kind", "report", "--checksum"],
    )?;
    assert!(
        stderr.contains("is empty (0 bytes)"),
        "error should name the emptiness, got: {stderr}"
    );
    assert!(
        stderr.contains("--allow-empty"),
        "error should name the escape hatch, got: {stderr}"
    );
    Ok(())
}

#[test]
fn evidence_artifact_rejects_a_path_with_actionable_guidance() -> anyhow::Result<()> {
    // Passing a path to --evidence-artifact is a natural mistake. The error should say what is
    // wanted and how to obtain it, rather than surfacing a raw integer-parse failure.
    let temp = TempDir::new()?;
    fs::create_dir_all(temp.path().join("output"))?;
    run_research(temp.path(), &["init"])?;
    run_research(
        temp.path(),
        &["program", "create", "demo", "--title", "Demo", "--objective", "Exercise the guard"],
    )?;

    let stderr = run_research_expect_failure(
        temp.path(),
        &[
            "fact", "add", "some-fact", "--program", "demo", "--status", "candidate",
            "--statement", "s", "--evidence-artifact", "output/report.txt",
        ],
    )?;
    assert!(
        stderr.contains("expects an artifact id"),
        "error should say an id is wanted, got: {stderr}"
    );
    assert!(
        stderr.contains("artifact add"),
        "error should show how to register it, got: {stderr}"
    );
    Ok(())
}
