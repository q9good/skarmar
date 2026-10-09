#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

if [[ -x /workspace/.toolchains/cargo/bin/cargo ]]; then
  export RUSTUP_HOME=/workspace/.toolchains/rustup
  export CARGO_HOME=/workspace/.toolchains/cargo
  export PATH="/workspace/.toolchains/cargo/bin:$PATH"
  export XDG_CACHE_HOME="${XDG_CACHE_HOME:-/workspace/.cache}"
  export XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-/workspace/.cache/cloudflare-config}"
  export npm_config_cache="${npm_config_cache:-/workspace/.cache/npm}"
fi
export WRANGLER_SEND_METRICS=false
exec npx --no-install wrangler dev --env demo --local "$@"
