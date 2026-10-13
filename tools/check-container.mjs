// Verify the actual deployment image and Compose workflow with isolated volumes.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { once } from 'node:events';
import { resolve } from 'node:path';
import { ownerPassword } from './auth-contract.mjs';

const root = resolve(import.meta.dirname, '..');
const deploy = resolve(root, 'deploy/host');
const project = 'skarma-validation-' + randomUUID().slice(0, 8);
const listener = createServer().listen(0, '127.0.0.1'); await once(listener, 'listening');
const port = listener.address().port; await new Promise(resolve => listener.close(resolve));
const env = { ...process.env, COMPOSE_PROJECT_NAME: project, COMPOSE_FILE: resolve(deploy, 'compose.yaml'),
  SKARMA_PUBLIC_URL: 'https://contract.local', SKARMA_LOCAL_PORT: String(port), SKARMA_SPACE: 'container-contract', SKARMA_PRODUCTION: '1' };
const origin = `http://127.0.0.1:${port}`;
const artifacts = new Set();
function rememberBackups(output) { for (const match of output.matchAll(/备份: (.+)/g)) artifacts.add(resolve(deploy, match[1].trim())); }
const compose = (args, input) => execFileSync('docker', ['compose', ...args], { cwd: deploy, env, input, stdio: ['pipe', 'pipe', 'pipe'] }).toString();
async function ready() {
  for (let i = 0; i < 100; i++) {
    try { if ((await fetch(origin + '/api/health')).ok) return; } catch {}
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error('Container did not become ready');
}
async function api(path, cookie, body) {
  const response = await fetch(origin + '/api' + path, { method: body === undefined ? 'GET' : 'POST',
    headers: { origin: 'https://contract.local', 'x-skarma-request': '1', ...(cookie ? { cookie } : {}), ...(body === undefined ? {} : { 'content-type': 'application/json' }) },
    body: body === undefined ? undefined : JSON.stringify(body) });
  assert.equal(response.status, 200, path); const data = await response.json(); return { response, data };
}
async function login() { const { response } = await api('/auth/login', undefined, { username: 'owner', password: ownerPassword }); return response.headers.get('set-cookie').split(';')[0]; }
async function verifyExport(cookie) {
  const state = (await api('/state', cookie)).data;
  const history = (await Promise.all(state.goals.map(goal => api(`/goals/${goal.id}/history`, cookie).then(result => result.data)))).flat();
  const response = await fetch(origin + '/api/export.xlsx', { headers: { cookie } });
  assert.equal(response.status, 200);
  assert.equal(response.headers.get('cache-control'), 'no-store');
  assert.equal(response.headers.get('content-type'), 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet');
  const file = Buffer.from(await response.arrayBuffer()).toString('base64');
  execFileSync('python3', ['tools/check_excel.py'], { cwd: root, input: JSON.stringify({ file, state, history }), stdio: ['pipe', 'inherit', 'inherit'] });
}
try {
  compose(['run', '--rm', '-T', '--no-deps', 'app', 'admin-init', '--username', 'owner', '--name', 'Contract owner', '--password-stdin'], ownerPassword + '\n');
  compose(['up', '-d', 'app']); await ready();
  assert.equal((await fetch(origin + '/api/state')).status, 401);
  assert.equal((await fetch(origin + '/api/export.xlsx')).status, 401);
  assert.equal(compose(['exec', '-T', 'app', 'id', '-u']).trim(), '10001');
  let cookie = await login(); assert.match(cookie, /^__Host-skarma_session=/);
  await api('/goals', cookie, { id: randomUUID(), title: 'Container snapshot', level: 'long', parent_id: null, area: '', criteria: '' });
  const snapshot = (await api('/state', cookie)).data;
  await verifyExport(cookie);
  compose(['restart', 'app']); await ready();
  assert.deepEqual((await api('/state', cookie)).data, snapshot);
  compose(['exec', '-T', 'app', 'skarma-api', 'healthcheck']);
  const output = execFileSync('bash', ['backup.sh'], { cwd: deploy, env, stdio: ['ignore', 'pipe', 'pipe'] }).toString();
  rememberBackups(output);
  const backup = resolve(deploy, output.match(/备份: (.+)/)[1].trim());
  await api('/goals', cookie, { id: randomUUID(), title: 'After snapshot', level: 'long', parent_id: null, area: '', criteria: '' });
  const restored = execFileSync('bash', ['restore.sh', backup, '--replace'], { cwd: deploy, env, stdio: ['ignore', 'pipe', 'pipe'] }).toString();
  rememberBackups(restored);
  await ready();
  assert.equal((await fetch(origin + '/api/state', { headers: { cookie } })).status, 401);
  assert.equal((await fetch(origin + '/api/export.xlsx', { headers: { cookie } })).status, 401);
  cookie = await login(); assert.deepEqual((await api('/state', cookie)).data, snapshot);
  await verifyExport(cookie);
  console.log('PASS deployment container: non-root, login, authenticated XLSX, persistent restart, healthcheck, backup and full-state restore');
} finally {
  compose(['down', '--volumes', '--remove-orphans']);
  for (const file of artifacts) await rm(file, { force: true });
}
