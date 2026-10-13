#!/usr/bin/env bash
set -euo pipefail
if [[ $# != 2 || "$2" != --replace ]]; then
  printf '%s\n' '用法: bash restore.sh /绝对路径/备份.db --replace（先备份现状，再停止服务并恢复）' >&2
  exit 1
fi
skarma_restore_file=$(realpath -- "$1")
[[ -f "$skarma_restore_file" ]] || { printf '%s\n' '备份文件不存在' >&2; exit 1; }
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
bash backup.sh
docker compose --profile tunnel stop cloudflared app
if ! docker compose run --rm -T --no-deps app restore-stdin --offline < "$skarma_restore_file"; then
  printf '%s\n' '恢复失败；服务保持停止。恢复前备份保存在 backups 目录。' >&2
  exit 1
fi
docker compose up -d app
if [[ -f secrets/cloudflare-tunnel-token ]]; then
  docker compose --profile tunnel up -d cloudflared
fi
printf '%s\n' '恢复完成；所有成员需要重新登录。'
