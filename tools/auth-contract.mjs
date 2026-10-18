// Real HTTP authentication contract, shared by native and Worker validation.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { excelContract } from './excel-contract.mjs';
import { knowledgeContract } from './knowledge-contract.mjs';
import { lessonContract } from './lesson-contract.mjs';
export const ownerPassword = 'synthetic-owner-contract-password';
export async function authContract(label, send, expire) {
  async function call(method, path, body, cookie, expected = 200, origin = 'https://contract.local') {
    const response = await send(path, {
      method, headers: { 'x-skarma-request': '1', origin, ...(body === undefined ? {} : { 'content-type': 'application/json' }), ...(cookie ? { cookie } : {}) },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    assert.equal(response.status, expected, `${label}: ${method} ${path}`);
    assert.equal(response.headers.get('cache-control'), 'no-store');
    return { response, data: await response.json() };
  }
  async function login(username, password) {
    const { response, data } = await call('POST', '/api/auth/login', { username, password });
    assert.equal(data.token, undefined);
    assert.equal(data.user.password_hash, undefined);
    const cookie = response.headers.get('set-cookie');
    assert.match(cookie, /^__Host-skarma_session=[a-f0-9]{64};/);
    for (const attribute of ['Secure', 'HttpOnly', 'SameSite=Strict', 'Path=/']) assert.ok(cookie.includes(attribute));
    return cookie.split(';')[0];
  }
  await call('GET', '/api/state', undefined, undefined, 401);
  await call('GET', '/api/export.xlsx', undefined, undefined, 401);
  await call('POST', '/api/pathways', {}, undefined, 401);
  await call('POST', '/api/experiences', {}, undefined, 401);
  await call('PUT', `/api/lessons/${randomUUID()}`, {}, undefined, 401);
  await call('POST', '/api/auth/login', { username: 'owner', password: 'incorrect' }, undefined, 401);
  const owner = await login('owner', ownerPassword);
  await call('GET', '/api/state', undefined, owner);
  const application = {
    username: 'applicant', display_name: 'Contract applicant',
    email: 'applicant@example.test', password: 'synthetic-applicant-password',
  };
  const submitted = await call('POST', '/api/auth/register', application, undefined, 202);
  assert.deepEqual(submitted.data, { ok: true });
  assert.equal(submitted.response.headers.get('set-cookie'), null);
  await call('POST', '/api/auth/login', { username: 'applicant', password: application.password }, undefined, 401);
  await call('GET', '/api/auth/registrations', undefined, undefined, 401);
  const pending = (await call('GET', '/api/auth/registrations', undefined, owner)).data;
  assert.equal(pending.length, 1);
  assert.equal(pending[0].username, 'applicant');
  assert.equal(pending[0].email, application.email);
  assert.equal(pending[0].password_hash, undefined);
  await call('POST', `/api/auth/registrations/${pending[0].id}/approve`, undefined, owner);
  const applicant = await login('applicant', application.password);
  await call('GET', '/api/auth/registrations', undefined, applicant, 403);
  const rejected = { ...application, username: 'rejected', email: 'rejected@example.test' };
  await call('POST', '/api/auth/register', rejected, undefined, 202);
  const next = (await call('GET', '/api/auth/registrations', undefined, owner)).data;
  assert.equal(next.length, 1);
  await call('POST', `/api/auth/registrations/${next[0].id}/reject`, undefined, owner);
  await call('POST', '/api/auth/login', { username: 'rejected', password: rejected.password }, undefined, 401);
  const { data: member } = await call('POST', '/api/auth/users', {
    username: 'teacher', display_name: 'Contract teacher', role: 'member', password: 'synthetic-member-password',
  }, owner);
  let teacher = await login('teacher', 'synthetic-member-password');
  await call('GET', '/api/state', undefined, teacher);
  await lessonContract(label, async (method, path, body, expected = 200) => (await call(method, path, body, teacher, expected)).data);
  await knowledgeContract(label, async (method, path, body, expected = 200) => (await call(method, path, body, teacher, expected)).data);
  await excelContract(label, send, teacher);
  await call('PUT', `/api/auth/users/${member.id}`, { active: false, password: 'short' }, owner, 422);
  await call('GET', '/api/state', undefined, teacher);
  await call('GET', '/api/auth/users', undefined, teacher, 403);
  await call('POST', '/api/auth/users', { username: 'forbidden', display_name: 'Forbidden', role: 'admin', password: ownerPassword }, teacher, 403);
  const csrf = await send('/api/auth/logout', { method: 'POST', headers: { cookie: owner } });
  assert.equal(csrf.status, 403); await csrf.text();
  await call('POST', '/api/auth/logout', undefined, owner, 403, 'https://evil.test');
  await call('PUT', `/api/auth/users/${member.id}`, { active: false }, owner);
  await call('GET', '/api/state', undefined, teacher, 401);
  await call('GET', '/api/export.xlsx', undefined, teacher, 401);
  await call('PUT', `/api/auth/users/${member.id}`, { active: true, password: 'synthetic-reset-password' }, owner);
  teacher = await login('teacher', 'synthetic-reset-password');
  await call('PUT', '/api/auth/password', { current_password: 'synthetic-reset-password', password: 'synthetic-next-password' }, teacher);
  await call('GET', '/api/state', undefined, teacher, 401);
  teacher = await login('teacher', 'synthetic-next-password');
  await call('POST', '/api/auth/logout', undefined, teacher);
  await call('GET', '/api/state', undefined, teacher, 401);
  await expire();
  await call('GET', '/api/state', undefined, owner, 401);
  await call('GET', '/api/export.xlsx', undefined, owner, 401);
  console.log(`PASS ${label}: registration approval/rejection, login, secure cookies, CSRF, member permissions, disable/reset, password change, logout and expiry`);
}
