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
rustup target add wasm32-unknown-unknown
export XDG_CACHE_HOME=/workspace/.cache
export XDG_CONFIG_HOME=/workspace/.cache/cloudflare-config
if ! command -v worker-build >/dev/null || [[ "$(worker-build --version)" != "0.8.7" ]]; then
  cargo install worker-build --version 0.8.7 --locked --jobs 4
fi
npm ci --no-audit --no-fund
npm run typecheck
EXPO_OFFLINE=1 EXPO_PUBLIC_API_URL='' npm run web:build
cargo build --workspace --locked -j 4
(cd crates/worker && worker-build --release)
