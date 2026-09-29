#!/usr/bin/env bash
set -euo pipefail
[[ $# == 1 ]] || { echo 'Usage: build-native.sh DESTINATION' >&2; exit 1; }
source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
crate=$source_dir/../anton-runtime
cargo build --release --locked --offline --manifest-path "$crate/Cargo.toml" --target-dir "$crate/target"
install -m 755 -- "$crate/target/release/anton-runtime" "$1"
