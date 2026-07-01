use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

use ldgr::manifest_integrity::canonical_manifest_digest;

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
fn help_documents_conduct_style_adapter_surface() -> anyhow::Result<()> {
    let mut command = research_command()?;
    command.arg("--help");
    command.assert().success().stdout(
        predicate::str::contains("ldgr-research install")
            .and(predicate::str::contains("ldgr research <command>"))
            .and(predicate::str::contains("Canonical LDGR surface"))
            .and(predicate::str::contains("profile discover/apply remain")),
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
    assert!(home.join(".ldgr/research/harness-setup.md").is_file());
    assert!(home
        .join(".pi/agent/skills/research-project-setup/SKILL.md")
        .is_file());
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
fn profile_apply_can_materialize_without_invoking_ldgr() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let install_root = temp.path().join("research-profile");

    let mut command = research_command()?;
    command.args([
        "profile",
        "apply",
        "--install-root",
        install_root.to_str().expect("utf-8 temp path"),
        "--materialize-only",
    ]);
    command.assert().success().stdout(predicate::str::contains(
        "materialized research adapter profile",
    ));

    let adapter = fs::read_to_string(install_root.join("adapter.toml"))?;
    assert!(adapter.contains("research"), "{adapter}");
    Ok(())
}

#[test]
fn open_research_adapter_install_and_apply_do_not_require_commercial_context() -> anyhow::Result<()>
{
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

    research_command()?
        .args([
            "profile",
            "apply",
            "--install-root",
            install_root.to_str().expect("utf-8 temp path"),
            "--materialize-only",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "materialized research adapter profile",
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
fn profile_discover_reads_adapter_path_ldgr_home_and_default_roots() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let adapter_path_root = temp.path().join("adapter-path-root");
    let ldgr_home = temp.path().join("ldgr-home");
    let home = temp.path().join("home");
    write_adapter(
        &adapter_path_root,
        "from-path",
        "Adapter path profile",
        &["path-alias"],
    )?;
    write_adapter(
        &ldgr_home.join("adapters"),
        "from-ldgr-home",
        "LDGR home profile",
        &[],
    )?;
    write_adapter(
        &home.join(".ldgr/adapters"),
        "from-default-home",
        "Default home profile",
        &[],
    )?;

    let adapter_path = std::env::join_paths([adapter_path_root.as_path()])?;
    let output = discover_command()?
        .env("LDGR_ADAPTER_PATH", adapter_path)
        .env("LDGR_HOME", &ldgr_home)
        .env("HOME", &home)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("adapter=from-path"), "{stdout}");
    assert!(stdout.contains("aliases=path-alias"), "{stdout}");
    assert!(stdout.contains("adapter=from-ldgr-home"), "{stdout}");
    assert!(stdout.contains("adapter=from-default-home"), "{stdout}");
    Ok(())
}

#[test]
fn profile_discover_skips_malformed_manifests_without_hiding_valid_adapters() -> anyhow::Result<()>
{
    let temp = TempDir::new()?;
    let root = temp.path().join("adapters");
    write_adapter(&root, "valid", "Valid profile", &[])?;
    let malformed_root = root.join("malformed");
    fs::create_dir_all(&malformed_root)?;
    fs::write(malformed_root.join("adapter.toml"), "[adapter\nslug =")?;

    let adapter_path = std::env::join_paths([root.as_path()])?;
    let output = discover_command()?
        .env("LDGR_ADAPTER_PATH", adapter_path)
        .env_remove("LDGR_HOME")
        .env("HOME", temp.path().join("empty-home"))
        .output()?;

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("adapter=valid"), "{stdout}");
    assert!(!stdout.contains("adapter=malformed"), "{stdout}");
    assert!(
        stderr.contains("warning: skipped adapter manifest"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn profile_discover_skips_manifest_with_mismatched_digest() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let root = temp.path().join("adapters");
    write_adapter(&root, "valid", "Valid profile", &[])?;
    write_signed_adapter(&root, "tampered", "Original profile", "Tampered profile")?;

    let adapter_path = std::env::join_paths([root.as_path()])?;
    let output = discover_command()?
        .env("LDGR_ADAPTER_PATH", adapter_path)
        .env_remove("LDGR_HOME")
        .env("HOME", temp.path().join("empty-home"))
        .output()?;

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("adapter=valid"), "{stdout}");
    assert!(!stdout.contains("adapter=tampered"), "{stdout}");
    assert!(
        stderr.contains("failed to verify") && stderr.contains("adapter manifest digest mismatch"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn profile_discover_prints_aliases_and_deduplicates_adapter_slugs() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let first_root = temp.path().join("first-root");
    let second_root = temp.path().join("second-root");
    write_adapter(
        &first_root,
        "duplicate",
        "First duplicate profile",
        &["dupe", "friendly"],
    )?;
    write_adapter(
        &second_root,
        "duplicate",
        "Second duplicate profile",
        &["second"],
    )?;

    let adapter_path = std::env::join_paths([first_root.as_path(), second_root.as_path()])?;
    let output = discover_command()?
        .env("LDGR_ADAPTER_PATH", adapter_path)
        .env_remove("LDGR_HOME")
        .env("HOME", temp.path().join("empty-home"))
        .output()?;

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.matches("adapter=duplicate").count(), 1, "{stdout}");
    assert!(stdout.contains("aliases=dupe,friendly"), "{stdout}");
    assert!(!stdout.contains("aliases=second"), "{stdout}");
    Ok(())
}

#[test]
fn unknown_commands_pass_through_to_ldgr() -> anyhow::Result<()> {
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
        .args(["status", "--json"]);
    command
        .assert()
        .success()
        .stdout(predicate::str::contains("passed-through"));

    let args = fs::read_to_string(args_log)?;
    assert_eq!(args.trim(), "status --json");
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

fn discover_command() -> anyhow::Result<Command> {
    let mut command = research_command()?;
    command.args(["profile", "discover"]);
    Ok(command)
}

fn write_adapter(root: &Path, slug: &str, title: &str, aliases: &[&str]) -> anyhow::Result<()> {
    let adapter_dir = root.join(slug);
    fs::create_dir_all(&adapter_dir)?;
    let aliases = aliases
        .iter()
        .map(|alias| format!(r#""{alias}""#))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        adapter_dir.join("adapter.toml"),
        format!(
            r#"[adapter]
slug = "{slug}"
title = "{title}"
core_version = "0.1"
aliases = [{aliases}]
"#
        ),
    )?;
    Ok(())
}

fn write_signed_adapter(
    root: &Path,
    slug: &str,
    signed_title: &str,
    tampered_title: &str,
) -> anyhow::Result<()> {
    let adapter_dir = root.join(slug);
    fs::create_dir_all(&adapter_dir)?;
    let manifest = format!(
        r#"[adapter]
slug = "{slug}"
title = "{signed_title}"
core_version = "0.1"
"#
    );
    let digest = canonical_manifest_digest(&manifest)?;
    let tampered = manifest.replace(signed_title, tampered_title);
    fs::write(
        adapter_dir.join("adapter.toml"),
        format!(
            r#"{tampered}[integrity]
manifest_digest = "{digest}"
"#
        ),
    )?;
    Ok(())
}
