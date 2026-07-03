use std::fs;
use std::path::Path;

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
    assert!(install_root
        .join("skills/research-project-setup/SKILL.md")
        .is_file());
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
        .stdout(predicate::str::contains("installed research skills"));

    assert!(install_root.join("adapter.toml").is_file());
    assert!(home.join(".ldgr/prompts/research-loop.md").is_file());
    assert!(home
        .join(".pi/agent/skills/research-project-setup/SKILL.md")
        .is_file());
    assert!(home.join(".ldgr/research/harness-setup.md").is_file());
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
    assert!(home.join(".ldgr/research/harness-setup.md").is_file());
    assert!(home
        .join(".pi/agent/skills/research-project-setup/SKILL.md")
        .is_file());
    let connection = rusqlite::Connection::open(temp.path().join(".ldgr/ldgr.db"))?;
    let (status, source_path): (String, String) = connection.query_row(
        "SELECT status, source_path FROM prompt WHERE slug = 'research-loop'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(status, "active");
    assert_eq!(
        source_path,
        home.join(".ldgr/research/loop-prompt.md")
            .to_string_lossy()
            .as_ref()
    );
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
    assert!(!home.join(".ldgr/research/adapter.toml").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_continues_when_adapter_bundle_refresh_is_unwritable() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    let adapter_root = home.join(".ldgr/research");
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

    let mut discover_command = research_command()?;
    discover_command.args(["profile", "discover"]);
    discover_command.assert().failure().stderr(
        predicate::str::contains("profile commands are not part of ldgr-research")
            .and(predicate::str::contains("ldgr-research profile apply").not())
            .and(predicate::str::contains("ldgr-research profile apply community-sample").not()),
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
fn non_conflicting_core_commands_pass_through_to_ldgr() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let fake_ldgr = temp.path().join("fake-ldgr.sh");
    let args_log = temp.path().join("args.txt");
    fs::write(
        &fake_ldgr,
        format!(
            "#!/usr/bin/env bash\nprintf '%s\\n' \"$*\" > {}\necho passed-through\n",
            args_log.display()
        ),
    )?;
    make_executable(&fake_ldgr)?;

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
    let fake_ldgr = temp.path().join("fake-ldgr.sh");
    let args_log = temp.path().join("args.txt");
    fs::write(
        &fake_ldgr,
        format!(
            "#!/usr/bin/env bash\nprintf '%s\n' \"$*\" > {}\necho core-pass-through\n",
            args_log.display()
        ),
    )?;
    make_executable(&fake_ldgr)?;

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
    let fake_ldgr = temp.path().join("fake-ldgr.sh");
    let args_log = temp.path().join("args.txt");
    fs::write(
        &fake_ldgr,
        format!(
            "#!/usr/bin/env bash\nprintf '%s\n' \"$*\" > {}\necho loop-pass-through\n",
            args_log.display()
        ),
    )?;
    make_executable(&fake_ldgr)?;

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
    let fake_ldgr = temp.path().join("fake-ldgr.sh");
    fs::write(
        &fake_ldgr,
        "#!/usr/bin/env bash\necho core-status-from-fake\n",
    )?;
    make_executable(&fake_ldgr)?;

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
    let fake_ldgr = temp.path().join("fake-ldgr.sh");
    let args_log = temp.path().join("args.txt");
    fs::write(
        &fake_ldgr,
        format!(
            "#!/usr/bin/env bash\nprintf '%s\\n' \"$*\" > {}\necho loop-pass-through\n",
            args_log.display()
        ),
    )?;
    make_executable(&fake_ldgr)?;

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
fn loop_run_pass_through_preserves_explicit_prompt_source() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let fake_ldgr = temp.path().join("fake-ldgr.sh");
    let args_log = temp.path().join("args.txt");
    fs::write(
        &fake_ldgr,
        format!(
            "#!/usr/bin/env bash\nprintf '%s\\n' \"$*\" > {}\necho explicit-prompt\n",
            args_log.display()
        ),
    )?;
    make_executable(&fake_ldgr)?;

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

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}
