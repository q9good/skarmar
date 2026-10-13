#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
umask 077
mkdir -p secrets
chmod 700 secrets
read -r -s -p 'Cloudflare Tunnel token（仅保存在本机）: ' skarma_tunnel_token
printf '\n'
if [[ -z "$skarma_tunnel_token" ]]; then printf '%s\n' 'Token 不能为空' >&2; exit 1; fi
skarma_token_tmp=$(mktemp secrets/.token.XXXXXX)
printf '%s' "$skarma_tunnel_token" > "$skarma_token_tmp"
unset skarma_tunnel_token
# Compose mounts this file as a secret for cloudflared's non-root process.
# On the host, the enclosing 0700 directory restricts access to its owner.
chmod 444 "$skarma_token_tmp"
mv -f -- "$skarma_token_tmp" secrets/cloudflare-tunnel-token
printf '%s\n' 'Tunnel token 已保存；运行中的隧道需重新创建容器以读取新 token。'
