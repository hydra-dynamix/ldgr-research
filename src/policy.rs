#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

pub const STARTER_POLICY: &str = r#"current_program:
current_branch:
research_mode_enabled: true
required_context_before_work: true
required_decision_after_experiment: true
require_human_approval_for_blocked_overrides: true
blocked_work: []
allowed_work: []
required_experiment_fields:
  - mode
  - hypothesis
  - setup
  - primary_metrics
  - result
  - interpretation
  - limitations
  - decision
  - allowed_next_steps
  - blocked_next_steps
allowed_artifact_roots:
  - output/
  - docs/
  - experiments/
review_thresholds:
  contested_fact_counterevidence_count: 1
recommendation:
  prefer_classifications:
    - main_path
    - validation
    - exploratory
"#;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    #[serde(default)]
    pub current_program: Option<String>,
    #[serde(default)]
    pub current_branch: Option<String>,
    #[serde(default = "default_true")]
    pub research_mode_enabled: bool,
    #[serde(default)]
    pub required_context_before_work: bool,
    #[serde(default)]
    pub required_decision_after_experiment: bool,
    #[serde(default)]
    pub require_human_approval_for_blocked_overrides: bool,
    #[serde(default)]
    pub blocked_work: Vec<String>,
    #[serde(default)]
    pub allowed_work: Vec<String>,
    #[serde(default)]
    pub required_experiment_fields: Vec<String>,
    #[serde(default)]
    pub allowed_artifact_roots: Vec<String>,
    #[serde(default)]
    pub review_thresholds: ReviewThresholds,
    #[serde(default)]
    pub recommendation: RecommendationPolicy,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewThresholds {
    #[serde(default)]
    pub contested_fact_counterevidence_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecommendationPolicy {
    #[serde(default)]
    pub prefer_classifications: Vec<String>,
}

fn default_true() -> bool {
    true
}

pub fn write_starter_policy_if_missing(path: &Path) -> anyhow::Result<bool> {
    if path.exists() {
        return Ok(false);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create policy directory {}", parent.display()))?;
    }

    fs::write(path, STARTER_POLICY)
        .with_context(|| format!("failed to write starter policy {}", path.display()))?;
    Ok(true)
}

pub fn load_policy(path: &Path) -> anyhow::Result<Policy> {
    if !path.exists() {
        bail!(
            "policy file {} not found; run `ldgr-research init` to create a starter policy or pass --policy with the correct path",
            path.display()
        );
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read policy file {}", path.display()))?;
    serde_yaml::from_str(&contents).with_context(|| {
        format!(
            "failed to parse policy file {}; check that known fields have the expected types and list fields are YAML lists",
            path.display()
        )
    })
}

pub fn save_policy(path: &Path, policy: &Policy) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create policy directory {}", parent.display()))?;
    }

    let contents = serde_yaml::to_string(policy)
        .with_context(|| format!("failed to serialize policy {}", path.display()))?;
    fs::write(path, contents)
        .with_context(|| format!("failed to write policy file {}", path.display()))
}

pub fn set_current_program(path: &Path, slug: &str) -> anyhow::Result<Policy> {
    let mut policy = load_policy(path)?;
    policy.current_program = Some(slug.to_owned());
    save_policy(path, &policy)?;
    Ok(policy)
}

pub fn set_current_branch(path: &Path, slug: &str) -> anyhow::Result<Policy> {
    let mut policy = load_policy(path)?;
    policy.current_branch = Some(slug.to_owned());
    save_policy(path, &policy)?;
    Ok(policy)
}

pub fn set_research_mode(path: &Path, enabled: bool) -> anyhow::Result<Policy> {
    let mut policy = load_policy(path)?;
    policy.research_mode_enabled = enabled;
    save_policy(path, &policy)?;
    Ok(policy)
}
