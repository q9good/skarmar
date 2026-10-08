#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

export RUSTUP_HOME=/workspace/.toolchains/rustup
export CARGO_HOME=/workspace/.toolchains/cargo
export PATH="/workspace/.toolchains/cargo/bin:$PATH"
export npm_config_cache=/workspace/.cache/npm
export EXPO_NO_TELEMETRY=1
export CI=1
mkdir -p /workspace/.toolchains /workspace/.cache/npm

if [[ ! -x /workspace/.toolchains/cargo/bin/rustup ]]; then
  curl --fail --silent --show-error --location https://sh.rustup.rs -o /tmp/skarma-rustup-init.sh
  sh /tmp/skarma-rustup-init.sh -y --no-modify-path --profile minimal --default-toolchain 1.99.0
fi
rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy
npm ci --no-audit --no-fund
npm run typecheck
EXPO_OFFLINE=1 EXPO_PUBLIC_API_URL='' npm run web:build
cargo build --workspace --locked -j 4
