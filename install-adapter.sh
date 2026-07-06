#!/usr/bin/env sh
set -eu

usage() {
  cat <<'EOF'
Usage: ./install-adapter.sh [--adapter-root PATH] [--install-root PATH] [--print-path]

Install the ldgr-research adapter bundle for LDGR discovery.

Options:
  --adapter-root PATH  Adapter root containing <slug>/adapter.toml
                       [default: $LDGR_HOME/adapters or ~/.ldgr/adapters]
  --install-root PATH  Exact bundle directory [default: <adapter-root>/research]
  --print-path         Print the installed manifest path only
  -h, --help           Show this help
EOF
}

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ -n "${LDGR_HOME:-}" ]; then
  adapter_root="$LDGR_HOME/adapters"
else
  adapter_root="${HOME:-.}/.ldgr/adapters"
fi
install_root=""
print_path=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --adapter-root)
      shift
      [ "$#" -gt 0 ] || { echo "--adapter-root requires a value" >&2; exit 2; }
      adapter_root="$1"
      ;;
    --install-root)
      shift
      [ "$#" -gt 0 ] || { echo "--install-root requires a value" >&2; exit 2; }
      install_root="$1"
      ;;
    --print-path)
      print_path=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

if [ -z "$install_root" ]; then
  install_root="$adapter_root/research"
fi

mkdir -p "$install_root/templates" "$install_root/skills" "$install_root/docs" "$install_root/scripts" "$install_root/prompts"
cp "$script_dir/adapter.toml" "$install_root/adapter.toml"
cp "$script_dir/loop-prompt.md" "$install_root/loop-prompt.md"
cp "$script_dir/loop-prompt.md" "$install_root/prompts/research-loop.md"
cp "$script_dir/templates/"*.md "$install_root/templates/"
cp "$script_dir/docs/"*.md "$install_root/docs/"
cp "$script_dir/scripts/campaign_"*.sh "$install_root/scripts/"
chmod +x "$install_root/scripts/campaign_"*.sh
cp -R "$script_dir/skills/research-project-setup" "$install_root/skills/"

configured_paths() {
  config="${HOME:-.}/.ldgr/config.json"
  key="$1"
  fallback="$2"
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$config" "${HOME:-.}" "$key" "$fallback" <<'PY'
import json
import pathlib
import sys

config_path, home, key, fallback = sys.argv[1:5]
home_path = pathlib.Path(home)
paths = []
try:
    with open(config_path, "r", encoding="utf-8") as handle:
        config = json.load(handle)
except Exception:
    config = {}
for harness in config.get("installed", []):
    for value in harness.get(key, []) or []:
        if not isinstance(value, str) or not value.strip():
            continue
        if value == "~":
            path = home_path
        elif value.startswith("~/"):
            path = home_path / value[2:]
        else:
            path = pathlib.Path(value)
        text = str(path)
        if text not in paths:
            paths.append(text)
if not paths:
    paths.append(str(home_path / fallback))
print("\n".join(paths))
PY
  else
    printf '%s\n' "${HOME:-.}/$fallback"
  fi
}

configured_paths prompt_paths ".ldgr/prompts" | while IFS= read -r prompt_root; do
  mkdir -p "$prompt_root"
  cp "$install_root/prompts/research-loop.md" "$prompt_root/research-loop.md"
  printf 'installed research prompts to %s\n' "$prompt_root"
done

configured_paths skill_paths ".pi/agent/skills" | while IFS= read -r skill_root; do
  mkdir -p "$skill_root"
  rm -rf "$skill_root/research-project-setup"
  cp -R "$install_root/skills/research-project-setup" "$skill_root/"
  printf 'installed research skills to %s\n' "$skill_root"
done

manifest_path="$install_root/adapter.toml"
if [ "$print_path" -eq 1 ]; then
  printf '%s\n' "$manifest_path"
else
  printf 'installed LDGR adapter `research`: %s\n' "$manifest_path"
  printf 'next: `ldgr research init` then `ldgr research agent-guide`\n'
fi
