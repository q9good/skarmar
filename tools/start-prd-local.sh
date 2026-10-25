#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
# PRD development uses its own database. Existing legacy databases are rejected.
export SKARMA_MODEL=prd_v1
export SKARMA_DEMO=0
export SKARMA_AUTH=required
export SKARMA_DATABASE="${SKARMA_DATABASE:-.local/prd-development.db}"
exec bash tools/start-local.sh "$@"
