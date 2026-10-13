// Disposable native host, browser login, consistent backup and offline restore.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { mkdtemp, mkdir, readFile, rm } from 'node:fs/promises';
import { spawn, execFileSync, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { resolve } from 'node:path';
import { ownerPassword } from './auth-contract.mjs';

const root = resolve(import.meta.dirname, '..');
const binary = resolve(root, 'target/debug/skarma-api');
await mkdir(resolve(root, '.local'), { recursive: true });
const scratch = await mkdtemp(resolve(root, '.local/host-contract-'));
const database = resolve(scratch, 'host.db');
const env = { ...process.env, SKARMA_DATABASE: database, SKARMA_STORAGE: 'sqlite', SKARMA_AUTH: 'required', SKARMA_DEMO: '1', SKARMA_SPACE: 'host-validation', SKARMA_PRODUCTION: '0', SKARMA_PUBLIC_URL: '' };
let child;
async function start(settings = {}) {
  let log = '';
  child = spawn(binary, [], { cwd: root, env: { ...env, SKARMA_PORT: '0', ...settings }, stdio: ['ignore', 'ignore', 'pipe'] });
  child.stderr.on('data', data => { log += data.toString(); });
  for (let i = 0; i < 100; i++) {
    const match = log.match(/listening on port (\d+)/);
    if (match) return `http://127.0.0.1:${match[1]}`;
    if (child.exitCode !== null) throw new Error(log);
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw new Error('Host startup timed out');
}
async function stop() { if (child?.pid && child.exitCode === null) { child.kill('SIGTERM'); await once(child, 'exit'); } }
let origin;
async function login() {
  const response = await fetch(origin + '/api/auth/login', { method: 'POST', headers: { 'content-type': 'application/json', 'x-skarma-request': '1' }, body: JSON.stringify({ username: 'owner', password: ownerPassword }) });
  assert.equal(response.status, 200); await response.json(); return response.headers.get('set-cookie').split(';')[0];
}
async function state(cookie) { const response = await fetch(origin + '/api/state', { headers: { cookie } }); assert.equal(response.status, 200); return response.json(); }
async function addGoal(cookie, title) {
  const response = await fetch(origin + '/api/goals', { method: 'POST', headers: { cookie, 'x-skarma-request': '1', 'content-type': 'application/json' }, body: JSON.stringify({ id: randomUUID(), title, level: 'long', parent_id: null, area: '', criteria: '' }) });
  assert.equal(response.status, 200); await response.json();
}
try {
  execFileSync('bash', ['tools/start-local.sh', 'admin-init', '--username', 'owner', '--name', '测试管理员', '--password-stdin'], { cwd: root, env, input: ownerPassword + '\n', stdio: ['pipe', 'ignore', 'pipe'] });
  origin = await start();
  execFileSync('python3', ['tools/smoke_auth_web.py'], { cwd: root, env: { ...process.env, SKARMA_TEST_URL: origin }, stdio: 'inherit' });
  let cookie = await login(); await addGoal(cookie, 'Before snapshot');
  const snapshot = await state(cookie);
  const backup = resolve(scratch, 'snapshot.db');
  execFileSync(binary, ['backup', backup], { env, stdio: ['ignore', 'ignore', 'pipe'] });
  assert.notEqual(spawnSync(binary, ['backup', backup], { env }).status, 0, 'backup must not overwrite');
  await addGoal(cookie, 'After snapshot'); const changed = await state(cookie);
  assert.equal(changed.goals.length, snapshot.goals.length + 1);
  await stop();
  assert.notEqual(spawnSync(binary, ['restore-stdin', '--offline'], { env, input: Buffer.alloc(0) }).status, 0, 'empty restore must fail');
  const future = resolve(scratch, 'future.db');
  execFileSync('python3', ['-c', 'import sqlite3,shutil,sys\nshutil.copyfile(sys.argv[1],sys.argv[2])\nwith sqlite3.connect(sys.argv[2]) as db: db.execute("INSERT INTO schema_migrations VALUES (99)")', backup, future]);
  assert.notEqual(spawnSync(binary, ['restore-stdin', '--offline'], { env, input: await readFile(future) }).status, 0, 'unsupported-version restore must fail');
  origin = await start(); assert.deepEqual(await state(cookie), changed); await stop();
  execFileSync(binary, ['restore-stdin', '--offline'], { env, input: await readFile(backup), stdio: ['pipe', 'ignore', 'pipe'] });
  origin = await start();
  assert.equal((await fetch(origin + '/api/state', { headers: { cookie } })).status, 401, 'restore must revoke old sessions');
  cookie = await login(); assert.deepEqual(await state(cookie), snapshot);
  execFileSync(binary, ['healthcheck'], { env: { ...env, SKARMA_PORT: new URL(origin).port }, stdio: ['ignore', 'ignore', 'pipe'] });
  await stop();
  const demoDatabase = resolve(scratch, 'demo.db');
  origin = await start({ SKARMA_DATABASE: demoDatabase, SKARMA_AUTH: 'demo', SKARMA_SPACE: 'demo' });
  execFileSync('python3', ['tools/smoke_web.py'], { cwd: root, env: { ...process.env, SKARMA_TEST_URL: origin }, stdio: 'inherit' });
  console.log('PASS native host: live SQLite backup, overwrite refusal, invalid restore preservation, full-state recovery, session revocation and healthcheck');
} finally { await stop(); await rm(scratch, { recursive: true, force: true }); }
