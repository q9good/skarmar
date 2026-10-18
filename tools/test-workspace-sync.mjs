import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { pullWorkspace, pushWorkspace, readState, verifyArtifacts, workspaceStatus } from './workspace-sync.mjs';

const branch = 'feat/tencent-informed-web';
const git = (root, ...args) => execFileSync('git', ['-C', root, ...args], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
const head = root => git(root, 'rev-parse', 'HEAD');
function commit(root, path, content, message = 'fixture change') {
  writeFileSync(join(root, path), content);
  git(root, 'add', '--', path); git(root, 'commit', '-m', message);
}
function config(root) {
  git(root, 'config', 'user.name', 'Sync Fixture'); git(root, 'config', 'user.email', 'fixture@example.invalid');
  git(root, 'config', 'commit.gpgsign', 'false');
}
function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'skarma-sync-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const origin = join(root, 'origin.git'), cloud = join(root, 'cloud'), local = join(root, 'local');
  mkdirSync(origin); mkdirSync(cloud);
  git(origin, 'init', '--bare', `--initial-branch=${branch}`);
  git(cloud, 'init', `--initial-branch=${branch}`); config(cloud);
  commit(cloud, 'README.md', 'base\n');
  const implementationCommit = head(cloud);
  mkdirSync(join(cloud, 'docs'));
  const content = 'synthetic artifact\n';
  writeFileSync(join(cloud, 'docs/artifact.txt'), content);
  writeFileSync(join(cloud, 'docs/workspace-state.json'), JSON.stringify({ schemaVersion: 1, updatedAt: '2026-10-18', branch,
    implementationCommit, tasks: [{ id: 'T1', title: 'fixture task', status: 'done' }],
    artifacts: [{ path: 'docs/artifact.txt', sha256: createHash('sha256').update(content).digest('hex') }] }));
  git(cloud, 'add', '.'); git(cloud, 'commit', '-m', 'handoff fixture');
  git(cloud, 'remote', 'add', 'origin', origin); git(cloud, 'push', '-u', 'origin', branch);
  git(root, 'clone', origin, local); config(local);
  return { root, origin, cloud, local };
}

test('status reads task progress and verifies artifacts on a fresh checkout', t => {
  const { local } = fixture(t); const state = workspaceStatus(local);
  assert.equal(state.branch, branch); assert.equal(state.dirty, false);
  assert.equal(state.ahead, 0); assert.equal(state.behind, 0); assert.equal(state.artifacts.length, 1);
  assert.equal(state.state.tasks[0].status, 'done');
});
test('pull fast-forwards code, task progress and exact artifact bytes together', t => {
  const { cloud, local } = fixture(t);
  const state = readState(cloud); state.tasks[0].title = 'new milestone';
  const image = Buffer.from([0, 255, 10, 128, 0, 1]);
  writeFileSync(join(cloud, 'docs/artifact.txt'), image);
  state.artifacts[0].sha256 = createHash('sha256').update(image).digest('hex');
  commit(cloud, 'docs/workspace-state.json', JSON.stringify(state));
  git(cloud, 'add', 'docs/artifact.txt'); git(cloud, 'commit', '-m', 'new artifact'); git(cloud, 'push');
  const status = pullWorkspace(local);
  assert.equal(head(local), head(cloud)); assert.equal(status.state.tasks[0].title, 'new milestone');
  assert.deepEqual(readFileSync(join(local, 'docs/artifact.txt')), image); assert.equal(status.dirty, false);
});
test('local committed work pushes back and the cloud checkout can pull it', t => {
  const { cloud, local } = fixture(t);
  commit(local, 'local-plan.md', 'Local implementation plan\n'); pushWorkspace(local); pullWorkspace(cloud);
  assert.equal(head(local), head(cloud)); assert.equal(readFileSync(join(cloud, 'local-plan.md'), 'utf8'), 'Local implementation plan\n');
});
for (const mode of ['modified', 'staged', 'untracked']) {
  test(`pull preserves ${mode} files without stashing or resetting`, t => {
    const { local, cloud } = fixture(t); const before = head(local);
    commit(cloud, 'remote.md', 'remote\n'); git(cloud, 'push');
    const path = mode === 'untracked' ? 'notes.md' : 'README.md';
    writeFileSync(join(local, path), 'local work\n');
    if (mode === 'staged') git(local, 'add', path);
    assert.throws(() => pullWorkspace(local), /存在已修改/);
    assert.equal(head(local), before); assert.equal(readFileSync(join(local, path), 'utf8'), 'local work\n');
  });
}
test('divergence stops without changing either commit history', t => {
  const { cloud, local } = fixture(t);
  commit(cloud, 'cloud.md', 'cloud\n'); git(cloud, 'push'); commit(local, 'local.md', 'local\n');
  const before = head(local);
  assert.throws(() => pullWorkspace(local), /已分叉/); assert.equal(head(local), before);
  assert.equal(readFileSync(join(local, 'local.md'), 'utf8'), 'local\n');
});
test('local-only commits are preserved and require an explicit push', t => {
  const { local } = fixture(t); commit(local, 'local.md', 'local\n'); const before = head(local);
  assert.throws(() => pullWorkspace(local), /尚未推送/); assert.equal(head(local), before);
});
test('another branch is not switched or merged automatically', t => {
  const { local } = fixture(t); git(local, 'switch', '-c', 'local-task');
  assert.throws(() => pullWorkspace(local), /请先切换/); assert.throws(() => pushWorkspace(local), /请先切换/);
  assert.equal(git(local, 'branch', '--show-current'), 'local-task');
});
test('ignored files are protected, including incoming paths starting with spaces', t => {
  const { local, cloud } = fixture(t); const path = '  ignored file.txt';
  writeFileSync(join(local, '.git/info/exclude'), `*ignored file.txt\n`); writeFileSync(join(local, path), 'private local copy\n');
  commit(cloud, path, 'tracked remote file\n'); git(cloud, 'push'); const before = head(local);
  assert.throws(() => pullWorkspace(local), /本地忽略文件/); assert.equal(head(local), before);
  assert.equal(readFileSync(join(local, path), 'utf8'), 'private local copy\n');
});
test('incorrect remote artifacts are rejected before updating the checkout', t => {
  const { cloud, local } = fixture(t); const before = head(local);
  commit(cloud, 'docs/artifact.txt', 'changed without matching hash\n'); git(cloud, 'push');
  assert.throws(() => pullWorkspace(local), /远端产物校验失败/); assert.equal(head(local), before);
  assert.equal(workspaceStatus(local).dirty, false);
});
test('artifact verification rejects paths and symlinks outside the repository', t => {
  const { root, local } = fixture(t); writeFileSync(join(root, 'outside.txt'), 'outside fixture\n');
  const state = readState(local);
  state.artifacts[0].path = '../outside.txt';
  assert.throws(() => verifyArtifacts(local, state), /超出仓库/);
  if (process.platform !== 'win32') {
    symlinkSync(join(root, 'outside.txt'), join(local, 'docs/link.txt'));
    state.artifacts[0].path = 'docs/link.txt'; assert.throws(() => verifyArtifacts(local, state), /超出仓库/);
  }
});
test('push refuses dirty files and cannot overwrite newer remote commits', t => {
  const { local, cloud, origin } = fixture(t);
  writeFileSync(join(local, 'README.md'), 'not committed\n');
  assert.throws(() => pushWorkspace(local), /请先提交/);
  commit(local, 'README.md', 'local commit\n'); commit(cloud, 'cloud.md', 'cloud commit\n'); git(cloud, 'push');
  const before = git(origin, 'rev-parse', `refs/heads/${branch}`);
  assert.throws(() => pushWorkspace(local)); assert.equal(git(origin, 'rev-parse', `refs/heads/${branch}`), before);
});
test('status can run offline without a cached remote-tracking ref', t => {
  const { local } = fixture(t); git(local, 'branch', '-r', '-d', `origin/${branch}`);
  const status = workspaceStatus(local); assert.equal(status.ahead, null); assert.equal(status.behind, null);
});
