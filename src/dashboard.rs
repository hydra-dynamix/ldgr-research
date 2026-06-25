use anyhow::Context;
use rusqlite::Connection;
use serde_json::{json, Value};

pub fn render_dashboard_html(conn: &Connection, program_slug: &str) -> anyhow::Result<String> {
    let snapshot = crate::reports::export_program_json(conn, program_slug)?;
    let program = snapshot
        .get("program")
        .context("program export missing program object")?;
    let title = value_str(program, "title").unwrap_or(program_slug);
    let objective = value_str(program, "objective").unwrap_or("");
    let dashboard_json = dashboard_summary_json(&snapshot);
    let snapshot_json = serde_json::to_string_pretty(&snapshot)?;
    let summary_json = serde_json::to_string_pretty(&dashboard_json)?;

    Ok(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Research Dashboard - {title}</title>
<style>
:root {{
  color-scheme: light;
  --bg: #f6f7f9;
  --panel: #ffffff;
  --ink: #1d2733;
  --muted: #667085;
  --line: #d8dee7;
  --accent: #0f766e;
  --accent-weak: #d7f0ec;
  --warn: #b45309;
  --warn-weak: #fff4df;
  --bad: #b42318;
  --bad-weak: #ffe8e5;
}}
* {{ box-sizing: border-box; }}
body {{
  margin: 0;
  font: 14px/1.45 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  color: var(--ink);
  background: var(--bg);
}}
header {{
  padding: 28px clamp(18px, 4vw, 48px) 18px;
  border-bottom: 1px solid var(--line);
  background: var(--panel);
}}
main {{
  padding: 20px clamp(18px, 4vw, 48px) 40px;
}}
h1, h2, h3, p {{ margin-top: 0; }}
h1 {{ margin-bottom: 8px; font-size: clamp(1.7rem, 3vw, 2.5rem); }}
h2 {{ font-size: 1.05rem; margin-bottom: 12px; }}
h3 {{ font-size: 0.95rem; margin-bottom: 8px; }}
.muted {{ color: var(--muted); }}
.grid {{ display: grid; gap: 14px; }}
.summary {{ grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); margin-bottom: 18px; }}
.layout {{ grid-template-columns: minmax(0, 1.4fr) minmax(320px, 0.8fr); align-items: start; }}
.panel {{
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 16px;
}}
.stat .value {{ font-size: 1.9rem; font-weight: 700; }}
.stat .label {{ color: var(--muted); }}
.pill {{
  display: inline-flex;
  align-items: center;
  min-height: 24px;
  padding: 2px 8px;
  border-radius: 999px;
  background: #eef2f6;
  color: #344054;
  font-size: 0.8rem;
  white-space: nowrap;
}}
.pill.open, .pill.running, .pill.active, .pill.selected, .pill.in_progress {{ background: var(--accent-weak); color: #075e57; }}
.pill.needs_review, .pill.candidate {{ background: var(--warn-weak); color: var(--warn); }}
.pill.failed, .pill.blocked, .pill.contested {{ background: var(--bad-weak); color: var(--bad); }}
.section-list {{ display: grid; gap: 10px; margin: 0; padding: 0; list-style: none; }}
.item {{ border-top: 1px solid var(--line); padding-top: 10px; }}
.item:first-child {{ border-top: 0; padding-top: 0; }}
.item-title {{ display: flex; gap: 8px; align-items: center; justify-content: space-between; }}
.item-title strong {{ overflow-wrap: anywhere; }}
.tree {{ display: grid; gap: 12px; }}
.branch {{
  border-left: 3px solid var(--accent);
  padding-left: 12px;
}}
.experiment {{
  margin: 8px 0 0 12px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #fbfcfd;
}}
table {{ width: 100%; border-collapse: collapse; }}
th, td {{ border-top: 1px solid var(--line); padding: 8px; text-align: left; vertical-align: top; }}
th {{ color: var(--muted); font-weight: 600; }}
details {{ margin-top: 14px; }}
summary {{ cursor: pointer; color: var(--accent); font-weight: 600; }}
pre {{
  max-height: 520px;
  overflow: auto;
  padding: 12px;
  border-radius: 6px;
  background: #111827;
  color: #e5e7eb;
  font-size: 12px;
}}
@media (max-width: 900px) {{
  .layout {{ grid-template-columns: 1fr; }}
}}
</style>
</head>
<body>
<header>
  <h1>{title}</h1>
  <p class="muted">{objective}</p>
</header>
<main>
  <section id="summary" class="grid summary"></section>
  <section class="grid layout">
    <div class="grid">
      <section class="panel">
        <h2>Research Tree</h2>
        <div id="tree" class="tree"></div>
      </section>
      <section class="panel">
        <h2>Experiments</h2>
        <table id="experiments"></table>
      </section>
    </div>
    <div class="grid">
      <section class="panel">
        <h2>Open Work</h2>
        <ul id="options" class="section-list"></ul>
      </section>
      <section class="panel">
        <h2>Attention</h2>
        <ul id="attention" class="section-list"></ul>
      </section>
      <section class="panel">
        <h2>Knowledge</h2>
        <h3>Facts</h3>
        <ul id="facts" class="section-list"></ul>
        <h3 style="margin-top:16px">Axioms</h3>
        <ul id="axioms" class="section-list"></ul>
      </section>
    </div>
  </section>
  <details>
    <summary>Snapshot JSON</summary>
    <pre id="snapshot-json"></pre>
  </details>
</main>
<script type="application/json" id="dashboard-summary">{summary_json}</script>
<script type="application/json" id="program-snapshot">{snapshot_json}</script>
<script>
const summary = JSON.parse(document.getElementById("dashboard-summary").textContent);
const snapshot = JSON.parse(document.getElementById("program-snapshot").textContent);

function text(value) {{
  return value === null || value === undefined || value === "" ? "none" : String(value);
}}

function pill(value) {{
  const element = document.createElement("span");
  const normalized = text(value).replaceAll(" ", "_");
  element.className = `pill ${{normalized}}`;
  element.textContent = text(value);
  return element;
}}

function appendEmpty(target, label) {{
  const item = document.createElement("li");
  item.className = "item muted";
  item.textContent = label;
  target.appendChild(item);
}}

function renderSummary() {{
  const target = document.getElementById("summary");
  for (const stat of summary.stats) {{
    const panel = document.createElement("div");
    panel.className = "panel stat";
    panel.innerHTML = `<div class="value">${{stat.value}}</div><div class="label">${{stat.label}}</div>`;
    target.appendChild(panel);
  }}
}}

function renderTree() {{
  const target = document.getElementById("tree");
  const experimentsByBranch = new Map();
  for (const experiment of snapshot.experiments) {{
    const branchExperiments = experimentsByBranch.get(experiment.branch_id) || [];
    branchExperiments.push(experiment);
    experimentsByBranch.set(experiment.branch_id, branchExperiments);
  }}
  for (const branch of snapshot.branches) {{
    const branchElement = document.createElement("div");
    branchElement.className = "branch";
    const title = document.createElement("div");
    title.className = "item-title";
    const name = document.createElement("strong");
    name.textContent = `${{branch.slug}} - ${{branch.title}}`;
    title.appendChild(name);
    title.appendChild(pill(branch.status));
    branchElement.appendChild(title);
    const question = document.createElement("div");
    question.className = "muted";
    question.textContent = text(branch.question);
    branchElement.appendChild(question);
    for (const experiment of experimentsByBranch.get(branch.id) || []) {{
      const experimentElement = document.createElement("div");
      experimentElement.className = "experiment";
      const experimentTitle = document.createElement("div");
      experimentTitle.className = "item-title";
      const experimentName = document.createElement("strong");
      experimentName.textContent = `${{experiment.slug}} - ${{experiment.title}}`;
      experimentTitle.appendChild(experimentName);
      experimentTitle.appendChild(pill(experiment.status));
      experimentElement.appendChild(experimentTitle);
      branchElement.appendChild(experimentElement);
    }}
    target.appendChild(branchElement);
  }}
}}

function renderExperiments() {{
  const target = document.getElementById("experiments");
  target.innerHTML = "<thead><tr><th>Experiment</th><th>Status</th><th>Mode</th><th>Runs</th><th>Latest Decision</th></tr></thead>";
  const body = document.createElement("tbody");
  for (const experiment of snapshot.experiments) {{
    const latestDecision = experiment.decisions && experiment.decisions.length > 0 ? experiment.decisions[0].decision : "none";
    const row = document.createElement("tr");
    row.innerHTML = `<td><strong>${{experiment.slug}}</strong><br><span class="muted">${{text(experiment.title)}}</span></td><td></td><td>${{text(experiment.mode)}}</td><td>${{experiment.runs.length}}</td><td>${{latestDecision}}</td>`;
    row.children[1].appendChild(pill(experiment.status));
    body.appendChild(row);
  }}
  target.appendChild(body);
}}

function renderOptions() {{
  const target = document.getElementById("options");
  const options = snapshot.research_options.filter((option) => ["open", "selected", "in_progress"].includes(option.status));
  if (options.length === 0) {{
    appendEmpty(target, "No open options.");
    return;
  }}
  for (const option of options) {{
    const item = document.createElement("li");
    item.className = "item";
    const title = document.createElement("div");
    title.className = "item-title";
    const name = document.createElement("strong");
    name.textContent = option.slug;
    title.appendChild(name);
    title.appendChild(pill(option.classification));
    item.appendChild(title);
    const description = document.createElement("div");
    description.className = "muted";
    description.textContent = text(option.description);
    item.appendChild(description);
    target.appendChild(item);
  }}
}}

function renderAttention() {{
  const target = document.getElementById("attention");
  const candidateFacts = snapshot.facts.filter((fact) => fact.status === "candidate");
  if (candidateFacts.length === 0) {{
    appendEmpty(target, "No candidate facts or review items in this export.");
    return;
  }}
  for (const fact of candidateFacts) {{
    const item = document.createElement("li");
    item.className = "item";
    item.appendChild(pill("candidate"));
    item.append(` ${{fact.slug}} - ${{fact.statement}}`);
    target.appendChild(item);
  }}
}}

function renderKnowledge() {{
  const facts = document.getElementById("facts");
  const acceptedFacts = snapshot.facts.filter((fact) => fact.status === "accepted");
  if (acceptedFacts.length === 0) {{
    appendEmpty(facts, "No accepted facts.");
  }}
  for (const fact of acceptedFacts) {{
    const item = document.createElement("li");
    item.className = "item";
    item.textContent = `${{fact.slug}} - ${{fact.statement}}`;
    facts.appendChild(item);
  }}
  const axioms = document.getElementById("axioms");
  if (snapshot.axioms.length === 0) {{
    appendEmpty(axioms, "No axioms.");
  }}
  for (const axiom of snapshot.axioms) {{
    const item = document.createElement("li");
    item.className = "item";
    item.textContent = `${{axiom.slug}} - ${{axiom.statement}}`;
    axioms.appendChild(item);
  }}
}}

renderSummary();
renderTree();
renderExperiments();
renderOptions();
renderAttention();
renderKnowledge();
document.getElementById("snapshot-json").textContent = JSON.stringify(snapshot, null, 2);
</script>
</body>
</html>
"#,
        title = escape_html(title),
        objective = escape_html(objective),
        summary_json = escape_json_for_script(&summary_json),
        snapshot_json = escape_json_for_script(&snapshot_json),
    ))
}

fn dashboard_summary_json(snapshot: &Value) -> Value {
    let experiments = value_array(snapshot, "experiments");
    let options = value_array(snapshot, "research_options");
    let facts = value_array(snapshot, "facts");
    let axioms = value_array(snapshot, "axioms");
    let open_options = options
        .iter()
        .filter(|option| {
            matches!(
                value_str(option, "status"),
                Some("open" | "selected" | "in_progress")
            )
        })
        .count();
    let active_experiments = experiments
        .iter()
        .filter(|experiment| matches!(value_str(experiment, "status"), Some("planned" | "running")))
        .count();
    let candidate_facts = facts
        .iter()
        .filter(|fact| value_str(fact, "status") == Some("candidate"))
        .count();

    json!({
        "stats": [
            {"label": "Branches", "value": value_array(snapshot, "branches").len()},
            {"label": "Experiments", "value": experiments.len()},
            {"label": "Active Experiments", "value": active_experiments},
            {"label": "Open Work", "value": open_options},
            {"label": "Candidate Facts", "value": candidate_facts},
            {"label": "Axioms", "value": axioms.len()},
        ]
    })
}

fn value_array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn value_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_json_for_script(value: &str) -> String {
    value
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
}
