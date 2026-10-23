// Targeted UI creation check on fresh native SQLite and local Rust Worker/DO.
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, rm } from 'node:fs/promises';
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { resolve, extname, sep } from 'node:path';
import { Miniflare } from 'miniflare';
import { ownerPassword } from './auth-contract.mjs';

const root = resolve(import.meta.dirname, '..');
await mkdir(resolve(root, '.local'), { recursive: true });
const scratch = await mkdtemp(resolve(root, '.local/relation-selftest-'));
const mutations = process.argv.includes('--mutations');
const lessons = process.argv.includes('--lessons');
const restoreDemo = process.argv.includes('--restore-demo');
const output = resolve(root, restoreDemo ? '.local/demo-restore-selftest-evidence' : lessons ? '.local/lesson-links-selftest-evidence' : mutations ? '.local/mutation-selftest-evidence' : '.local/relation-selftest-evidence');
await mkdir(output, { recursive: true });
const binary = resolve(root, 'target/debug/skarma-api');
const bindings = { SKARMA_STORAGE: 'sqlite', SKARMA_AUTH: 'required', SKARMA_DEMO: '0', SKARMA_PRODUCTION: '0', SKARMA_PUBLIC_URL: '', SKARMA_SPACE: 'relation-selftest' };
const env = { ...process.env, ...bindings, SKARMA_DATABASE: resolve(scratch, 'native.db'), SKARMA_PORT: '0' };
let child, mf, gateway;
async function stopNative() {
  const current = child; child = undefined;
  if (current && current.exitCode === null && current.signalCode === null) { current.kill('SIGTERM'); await once(current, 'exit'); }
}
async function startNative() {
  let log = '';
  child = spawn(binary, [], { cwd: root, env, stdio: ['ignore', 'ignore', 'pipe'] });
  child.stderr.on('data', chunk => { log += chunk.toString(); });
  for (let i = 0; i < 100; i++) {
    const match = log.match(/listening on port (\d+)/);
    if (match) return `http://127.0.0.1:${match[1]}`;
    if (child.exitCode !== null) throw new Error(log);
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw new Error('Temporary native server did not start');
}
async function verifyRestart(origin, expected) {
  const login = await fetch(origin + '/api/auth/login', { method: 'POST', headers: { 'content-type': 'application/json', 'x-skarma-request': '1' }, body: JSON.stringify({ username: 'owner', password: ownerPassword }) });
  assert.equal(login.status, 200); await login.json();
  const cookie = login.headers.get('set-cookie').split(';')[0];
  const response = await fetch(origin + '/api/state', { headers: { cookie } });
  assert.equal(response.status, 200); assert.deepEqual(await response.json(), expected);
}
async function browserCheck(origin, backend) {
  const destination = resolve(output, backend);
  await rm(destination, { recursive: true, force: true });
  const browser = spawn('python3', [restoreDemo ? 'tools/smoke_restore_demo.py' : lessons ? 'tools/smoke_lesson_links_web.py' : 'tools/smoke_relations_web.py'], { cwd: root, env: { ...process.env, SKARMA_TEST_URL: origin, SKARMA_RELATION_OUTPUT: destination, SKARMA_TEST_MUTATIONS: mutations ? '1' : '0', ...(restoreDemo ? { SKARMA_TEST_LOGIN: JSON.stringify({username:'owner',password:ownerPassword}) } : {}) }, stdio: 'inherit' });
  const [code] = await once(browser, 'exit');
  assert.equal(code, 0, `${backend} browser relationship test failed`);
  return readFile(resolve(destination, 'result.json'), 'utf8').then(JSON.parse);
}
try {
  execFileSync(binary, ['admin-init', '--username', 'owner', '--name', '关联自测管理员', '--password-stdin'], { cwd: root, env, input: ownerPassword + '\n', stdio: ['pipe', 'ignore', 'pipe'] });
  let origin = await startNative();
  const native = await browserCheck(origin, 'native');
  await stopNative(); origin = await startNative();
  await verifyRestart(origin, native.snapshot);
  await stopNative(); console.log('PASS native restart: the newly created graph is unchanged');

  // Same-origin local gateway serves the built web assets; all API work runs in
  // the actual Rust Worker and a temporary Miniflare SQLite Durable Object.
  const dist = resolve(root, 'apps/client/dist');
  gateway = createServer(async (req, res) => {
    try {
      if (req.url.startsWith('/api/')) {
        const chunks = []; for await (const chunk of req) chunks.push(chunk);
        const response = await mf.dispatchFetch(new URL(req.url, origin), { method: req.method, headers: req.headers, body: ['GET', 'HEAD'].includes(req.method) ? undefined : Buffer.concat(chunks) });
        res.writeHead(response.status, Object.fromEntries(response.headers));
        res.end(Buffer.from(await response.arrayBuffer())); return;
      }
      let file = resolve(dist, '.' + decodeURIComponent(new URL(req.url, origin).pathname));
      if (!file.startsWith(dist + sep)) file = resolve(dist, 'index.html');
      let bytes; try { bytes = await readFile(file); } catch { file = resolve(dist, 'index.html'); bytes = await readFile(file); }
      const type = { '.js': 'text/javascript', '.html': 'text/html', '.json': 'application/json', '.png': 'image/png', '.svg': 'image/svg+xml' }[extname(file)] ?? 'application/octet-stream';
      res.writeHead(200, { 'content-type': type, 'cache-control': 'no-store' }); res.end(bytes);
    } catch { res.writeHead(500); res.end('Temporary validation gateway failed'); }
  });
  gateway.listen(0, '127.0.0.1'); await once(gateway, 'listening');
  origin = `http://127.0.0.1:${gateway.address().port}`;
  const options = { modules: true, modulesRoot: root, scriptPath: resolve(root, 'crates/worker/build/index.js'),
    modulesRules: [{ type: 'ESModule', include: ['**/*.js', '**/*.mjs'] }, { type: 'CompiledWasm', include: ['**/*.wasm'] }],
    compatibilityDate: '2026-07-30', durableObjectsPersist: resolve(scratch, 'cf'),
    durableObjects: { SKARMA_DB: { className: 'SkarmaSpace', useSQLite: true } },
    bindings: { ...bindings, SKARMA_STORAGE: 'durable_object', SKARMA_PUBLIC_URL: origin, SKARMA_INITIAL_ADMIN_USER: 'owner', SKARMA_INITIAL_ADMIN_NAME: '关联自测管理员', SKARMA_INITIAL_ADMIN_PASSWORD: ownerPassword } };
  mf = new Miniflare(options); await mf.ready;
  const worker = await browserCheck(origin, 'worker');
  await mf.dispose(); mf = new Miniflare(options); await mf.ready;
  await verifyRestart(origin, worker.snapshot);
  console.log('PASS Rust Worker/DO restart: the newly created graph is unchanged');
  console.log('PASS targeted relationship self-test on both backends; evidence contains synthetic data only');
} finally {
  await stopNative();
  if (gateway) { gateway.closeAllConnections(); await new Promise(resolve => gateway.close(resolve)); }
  await mf?.dispose(); await rm(scratch, { recursive: true, force: true });
}
