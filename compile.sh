#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"
cargo build --release
cp -f target/release/eva ./executable
chmod +x ./executable
