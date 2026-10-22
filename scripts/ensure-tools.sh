#!/usr/bin/env bash
set -euo pipefail

if [[ $# -eq 0 ]]; then
  echo "usage: scripts/ensure-tools.sh <tool>..." >&2
  exit 2
fi

hint_for() {
  case "$1" in
    cargo|rustc|rustup) echo "https://rustup.rs" ;;
    cargo-clippy) echo "rustup component add clippy" ;;
    rustfmt) echo "rustup component add rustfmt" ;;
    node) echo "Node 24 LTS — https://nodejs.org" ;;
    pnpm) echo "pnpm 10.33 — https://pnpm.io/installation" ;;
    docker) echo "https://docs.docker.com/engine/install — the runner user must be in the docker group" ;;
    kubectl) echo "https://kubernetes.io/docs/tasks/tools/" ;;
    kustomize) echo "https://github.com/kubernetes-sigs/kustomize/releases" ;;
    *) echo "unknown tool" ;;
  esac
}

missing=()
for tool in "$@"; do
  if path="$(command -v "$tool" 2>/dev/null)"; then
    echo "ensure-tools: $tool at $path"
  else
    missing+=("$tool")
  fi
done

if [[ ${#missing[@]} -gt 0 ]]; then
  echo "::error::missing ${#missing[@]} tool(s): ${missing[*]}" >&2
  for tool in "${missing[@]}"; do
    printf '  %-11s %s\n' "$tool" "$(hint_for "$tool")" >&2
  done
  exit 1
fi
