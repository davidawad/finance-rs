#!/usr/bin/env sh
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$repo_root"
cargo run --quiet --example showcase -- docs/showcase/finance-rs-demo.svg
