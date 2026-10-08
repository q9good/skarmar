#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

if [[ -x /workspace/.toolchains/cargo/bin/cargo ]]; then
  export RUSTUP_HOME=/workspace/.toolchains/rustup
  export CARGO_HOME=/workspace/.toolchains/cargo
  export PATH="/workspace/.toolchains/cargo/bin:$PATH"
fi

export SKARMA_DEMO="${SKARMA_DEMO:-1}"
export SKARMA_DATABASE="${SKARMA_DATABASE:-.local/demo.db}"
exec cargo run --locked -p skarma-api
