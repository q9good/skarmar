#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
umask 077
mkdir -p backups
chmod 700 backups
skarma_backup_name="skarma-$(date -u +%Y%m%dT%H%M%SZ)-${RANDOM}.db"
docker compose exec -T app skarma-api backup "/data/backups/$skarma_backup_name"
docker compose cp "app:/data/backups/$skarma_backup_name" "backups/$skarma_backup_name"
chmod 600 "backups/$skarma_backup_name"
skarma_backup_source_hash=$(docker compose exec -T app sha256sum "/data/backups/$skarma_backup_name" | cut -d ' ' -f 1)
skarma_backup_copy_hash=$(sha256sum "backups/$skarma_backup_name" | cut -d ' ' -f 1)
[[ "$skarma_backup_source_hash" == "$skarma_backup_copy_hash" ]] || { printf '%s\n' '备份复制校验失败，容器内副本已保留' >&2; exit 1; }
docker compose exec -T app rm -f -- "/data/backups/$skarma_backup_name" "/data/backups/$skarma_backup_name-wal" "/data/backups/$skarma_backup_name-shm"
printf '备份: %s\n' "backups/$skarma_backup_name"
