#!/bin/sh
set -eu
rustc -O -o executable src/main.rs
