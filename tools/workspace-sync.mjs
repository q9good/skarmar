// Development handoff only: no database, credentials, chat history or running processes.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, realpathSync, statSync } from 'node:fs';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

function git(root, args) {
  const value = execFileSync('git', ['-C', root, ...args], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  return args.includes('-z') ? value : value.trim();
}
function parseState(root, value) {
  const state = JSON.parse(value);
  const statuses = new Set(['done', 'in_progress', 'blocked', 'todo']);
  if (state.schemaVersion !== 1 || typeof state.branch !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9._/-]*$/.test(state.branch)
    || !Array.isArray(state.tasks) || !Array.isArray(state.artifacts)
    || state.tasks.some(task => !task.id || !task.title || !statuses.has(task.status))) {
    throw new Error('交接状态格式错误，请核对 docs/workspace-state.json。');
  }
  git(root, ['check-ref-format', '--branch', state.branch]);
  return state;
}
export function readState(root) {
  return parseState(root, readFileSync(resolve(root, 'docs/workspace-state.json'), 'utf8'));
}
export function verifyArtifacts(root, state) {
  const base = realpathSync(root);
  return state.artifacts.map(artifact => {
    if (typeof artifact.path !== 'string' || isAbsolute(artifact.path)) throw new Error('产物必须使用仓库内相对路径。');
    const path = realpathSync(resolve(base, artifact.path));
    if (!path.startsWith(base + sep) || !statSync(path).isFile()) throw new Error(`产物超出仓库或不是文件：${artifact.path}`);
    const actual = createHash('sha256').update(readFileSync(path)).digest('hex');
    if (actual !== artifact.sha256) throw new Error(`产物校验失败：${artifact.path}。请核对并更新交接清单。`);
    return artifact.path;
  });
}
export function workspaceStatus(root) {
  const state = readState(root);
  const artifacts = verifyArtifacts(root, state);
  const branch = git(root, ['rev-parse', '--abbrev-ref', 'HEAD']);
  const commit = git(root, ['rev-parse', 'HEAD']);
  const dirty = Boolean(git(root, ['status', '--porcelain=v1', '--untracked-files=all']));
  const tracking = `refs/remotes/origin/${state.branch}`;
  let ahead = null, behind = null;
  try {
    [ahead, behind] = git(root, ['rev-list', '--left-right', '--count', `HEAD...${tracking}`]).split(/\s+/).map(Number);
  } catch { /* No local remote-tracking ref yet. Status never contacts the network. */ }
  return { branch, commit, dirty, targetBranch: state.branch, ahead, behind, artifacts, state };
}
export function pullWorkspace(root) {
  let state = readState(root);
  const branch = git(root, ['rev-parse', '--abbrev-ref', 'HEAD']);
  if (branch !== state.branch) throw new Error(`当前分支是 ${branch}，请先切换到 ${state.branch}；同步不会自动切换分支。`);
  if (git(root, ['status', '--porcelain=v1', '--untracked-files=all'])) {
    throw new Error('存在已修改、已暂存或未跟踪文件。请先提交或自行备份；同步不会自动 stash、reset 或覆盖。');
  }
  const tracking = `refs/remotes/origin/${state.branch}`;
  git(root, ['fetch', '--no-tags', 'origin', `refs/heads/${state.branch}:${tracking}`]);
  // Fetch may refresh refs, but divergence is rejected before touching any workspace file.
  try { git(root, ['merge-base', '--is-ancestor', 'HEAD', tracking]); }
  catch {
    try { git(root, ['merge-base', '--is-ancestor', tracking, 'HEAD']); }
    catch { throw new Error('本地和远端已分叉，请人工合并或 rebase；同步未修改工作区。'); }
    throw new Error('本地含有尚未推送的提交，请先核对并推送到 GitHub，再在另一环境拉取。');
  }
  const incomingState = parseState(root, git(root, ['show', `${tracking}:docs/workspace-state.json`]));
  if (incomingState.branch !== state.branch) throw new Error('远端交接分支配置已改变，请人工核对；同步未修改工作区。');
  for (const artifact of incomingState.artifacts) {
    if (typeof artifact.path !== 'string' || !artifact.path || isAbsolute(artifact.path)
      || relative(root, resolve(root, artifact.path)).startsWith('..')) throw new Error('远端产物路径无效。');
    const entry = git(root, ['ls-tree', tracking, '--', artifact.path]);
    if (!/^100(644|755) blob /.test(entry)) throw new Error(`远端产物缺失或不是普通文件：${artifact.path}`);
    const content = execFileSync('git', ['-C', root, 'show', `${tracking}:${artifact.path}`], { stdio: ['ignore', 'pipe', 'pipe'] });
    if (createHash('sha256').update(content).digest('hex') !== artifact.sha256) throw new Error(`远端产物校验失败：${artifact.path}；同步未修改工作区。`);
  }
  const incoming = git(root, ['diff', '--name-only', '-z', 'HEAD', tracking]).split('\0').filter(Boolean);
  // Git can overwrite ignored files while checking out a newly tracked path.
  // Check each incoming path without shell interpolation or command-length limits.
  for (const path of incoming) {
    if (git(root, ['ls-files', '--others', '--ignored', '--exclude-standard', '-z', '--', path])) {
      throw new Error(`远端变更会占用本地忽略文件：${path}。请先自行保留该文件；同步未修改工作区。`);
    }
  }
  git(root, ['merge', '--ff-only', '--no-edit', tracking]);
  state = readState(root);
  verifyArtifacts(root, state);
  return workspaceStatus(root);
}
export function pushWorkspace(root) {
  const status = workspaceStatus(root);
  if (status.branch !== status.targetBranch) throw new Error(`请先切换到 ${status.targetBranch}；同步不会自动切换分支。`);
  if (status.dirty) throw new Error('请先提交需要同步的代码、进度与产物；同步不会自动提交文件。');
  git(root, ['push', 'origin', `HEAD:refs/heads/${status.targetBranch}`]);
  return workspaceStatus(root);
}
function printStatus(status) {
  console.log(`分支：${status.branch} · ${status.commit}`);
  console.log(`工作区：${status.dirty ? '有未提交内容' : '干净'}`);
  console.log(status.ahead === null ? '远端比较：尚无本地跟踪信息；sync:status 不访问网络。' :
    `相对最后已获取的 origin/${status.targetBranch}：本地领先 ${status.ahead}，落后 ${status.behind}（非实时查询）`);
  console.log(`交接记录：${status.state.updatedAt} · 应用验收对应 ${status.state.implementationCommit}`);
  const labels = { done: '已完成', in_progress: '进行中', blocked: '受阻', todo: '待办' };
  for (const task of status.state.tasks) console.log(`[${labels[task.status]}] ${task.id} ${task.title}${task.blocker ? `：${task.blocker}` : ''}`);
  console.log(`产物校验通过：${status.artifacts.length} 项。下一步阅读 docs/handoff.md。`);
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const command = process.argv[2] ?? 'status';
  try {
    if (command === 'status') printStatus(workspaceStatus(root));
    else if (command === 'pull') { const status = pullWorkspace(root); console.log('远端获取与快进校验完成。'); printStatus(status); }
    else if (command === 'push') { const status = pushWorkspace(root); console.log('已提交内容已推送到 GitHub。'); printStatus(status); }
    else throw new Error('用法：node tools/workspace-sync.mjs status | pull | push');
  } catch (error) {
    console.error(error.stderr?.toString().trim() || error.message);
    process.exitCode = 1;
  }
}
