use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

pub const STARTER_TOOLS: &str = r#"# Durable reusable research tools.
# Agents may add entries here when a script or fixture becomes a repeated instrument.
# Keep experiments as the claim-bearing records; tools are reusable meters, runners, or inputs.
version: 1
tools: []
"#;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolRegistry {
    #[serde(default = "default_version")]
    pub version: u64,
    #[serde(default)]
    pub tools: Vec<ResearchTool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchTool {
    pub slug: String,
    pub kind: ToolKind,
    pub path: String,
    pub purpose: String,
    #[serde(default)]
    pub mutability: ToolMutability,
    #[serde(default)]
    pub status: ToolStatus,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Diagnostic,
    Probe,
    Audit,
    Validation,
    Harness,
    Perturbation,
    Fixture,
    ReportTemplate,
    #[default]
    Other,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolMutability {
    ReadOnly,
    WritesArtifacts,
    MutatesSubstrate,
    #[default]
    Unknown,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    #[default]
    Active,
    Draft,
    Deprecated,
    Broken,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolFindingSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolFinding {
    pub severity: ToolFindingSeverity,
    pub slug: Option<String>,
    pub message: String,
}

fn default_version() -> u64 {
    1
}

pub fn write_starter_registry_if_missing(path: &Path) -> anyhow::Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create tools directory {}", parent.display()))?;
    }
    fs::write(path, STARTER_TOOLS)
        .with_context(|| format!("failed to write starter tools registry {}", path.display()))?;
    Ok(true)
}

pub fn load_registry(path: &Path) -> anyhow::Result<ToolRegistry> {
    if !path.exists() {
        bail!(
            "tool registry {} not found; run `ldgr-research tool init` to create it",
            path.display()
        );
    }
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read tool registry {}", path.display()))?;
    serde_yaml::from_str(&contents).with_context(|| {
        format!(
            "failed to parse tool registry {}; check that it is valid YAML",
            path.display()
        )
    })
}

pub fn find_tool<'a>(registry: &'a ToolRegistry, slug: &str) -> Option<&'a ResearchTool> {
    registry.tools.iter().find(|tool| tool.slug == slug)
}

pub fn validate_registry(registry: &ToolRegistry, project_root: &Path) -> Vec<ToolFinding> {
    let mut findings = Vec::new();
    let mut seen = BTreeSet::new();

    if registry.version != 1 {
        findings.push(ToolFinding {
            severity: ToolFindingSeverity::Warning,
            slug: None,
            message: format!(
                "tool registry version {} is not recognized",
                registry.version
            ),
        });
    }

    for tool in &registry.tools {
        let slug = if tool.slug.trim().is_empty() {
            None
        } else {
            Some(tool.slug.clone())
        };
        if let Some(slug) = slug.as_deref() {
            if !seen.insert(slug.to_owned()) {
                findings.push(ToolFinding {
                    severity: ToolFindingSeverity::Error,
                    slug: Some(slug.to_owned()),
                    message: "duplicate tool slug".to_owned(),
                });
            }
        } else {
            findings.push(ToolFinding {
                severity: ToolFindingSeverity::Error,
                slug: None,
                message: "tool slug must not be empty".to_owned(),
            });
        }

        if tool.path.trim().is_empty() {
            findings.push(ToolFinding {
                severity: ToolFindingSeverity::Error,
                slug,
                message: "tool path must not be empty".to_owned(),
            });
        } else {
            let path = project_root.join(&tool.path);
            if !path.exists() {
                findings.push(ToolFinding {
                    severity: ToolFindingSeverity::Warning,
                    slug,
                    message: format!("tool path {} does not exist", tool.path),
                });
            }
        }

        if tool.purpose.trim().is_empty() {
            findings.push(ToolFinding {
                severity: ToolFindingSeverity::Warning,
                slug: if tool.slug.trim().is_empty() {
                    None
                } else {
                    Some(tool.slug.clone())
                },
                message: "tool purpose is empty".to_owned(),
            });
        }
    }

    findings
}

pub fn format_findings(findings: &[ToolFinding]) -> String {
    if findings.is_empty() {
        return "ok: tool registry has no validation findings\n".to_owned();
    }

    let mut output = String::new();
    for finding in findings {
        let severity = match finding.severity {
            ToolFindingSeverity::Error => "error",
            ToolFindingSeverity::Warning => "warning",
        };
        match finding.slug.as_deref() {
            Some(slug) => output.push_str(&format!("{severity}: {slug}: {}\n", finding.message)),
            None => output.push_str(&format!("{severity}: {}\n", finding.message)),
        }
    }
    output
}

pub fn has_errors(findings: &[ToolFinding]) -> bool {
    findings
        .iter()
        .any(|finding| finding.severity == ToolFindingSeverity::Error)
}
