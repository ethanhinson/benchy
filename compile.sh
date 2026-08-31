#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

cargo build --release --quiet
cp -f target/release/hexyl ./executable
