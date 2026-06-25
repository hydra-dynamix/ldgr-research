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
  adapter_root="${HOME:?HOME is required}/.ldgr/adapters"
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

mkdir -p "$install_root/templates" "$install_root/skills" "$install_root/docs" "$install_root/scripts"
cp "$script_dir/adapter.toml" "$install_root/adapter.toml"
cp "$script_dir/loop-prompt.md" "$install_root/loop-prompt.md"
cp "$script_dir/templates/"*.md "$install_root/templates/"
cp "$script_dir/docs/"*.md "$install_root/docs/"
cp "$script_dir/scripts/campaign_"*.sh "$install_root/scripts/"
chmod +x "$install_root/scripts/campaign_"*.sh
cp -R "$script_dir/skills/research-project-setup" "$install_root/skills/"

manifest_path="$install_root/adapter.toml"
if [ "$print_path" -eq 1 ]; then
  printf '%s\n' "$manifest_path"
else
  printf 'installed LDGR adapter `research`: %s\n' "$manifest_path"
  printf 'next: `ldgr-research profile discover` then `ldgr-research profile apply research`\n'
fi
