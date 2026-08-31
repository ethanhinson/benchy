#!/bin/sh
# Official ProgramBench eval for one packaged arm tree.
# Requires linux/amd64, Docker, and the programbench CLI.
set -eu

arm="${1:?usage: official_eval.sh <arm> [output-dir] [slice-dir]}"
out="${2:-official-results}"
slice="${3:-slice-b-1-official}"
root="$(cd "$(dirname "$0")/.." && pwd)"
src="$root/artifacts/$slice/$arm"

if [ ! -d "$src" ]; then
  echo "missing official tree: $src" >&2
  exit 2
fi

mkdir -p "$out"
uvx programbench eval "$src" --workers 1 --docker-cpus 2 -o "$out"
uvx programbench info "$out/$arm"
