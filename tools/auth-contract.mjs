// Real HTTP authentication contract, shared by native and Worker validation.
import assert from 'node:assert/strict';
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
  await call('POST', '/api/auth/login', { username: 'owner', password: 'incorrect' }, undefined, 401);
  const owner = await login('owner', ownerPassword);
  await call('GET', '/api/state', undefined, owner);
  const { data: member } = await call('POST', '/api/auth/users', {
    username: 'teacher', display_name: 'Contract teacher', role: 'member', password: 'synthetic-member-password',
  }, owner);
  let teacher = await login('teacher', 'synthetic-member-password');
  await call('GET', '/api/state', undefined, teacher);
  await call('PUT', `/api/auth/users/${member.id}`, { active: false, password: 'short' }, owner, 422);
  await call('GET', '/api/state', undefined, teacher);
  await call('GET', '/api/auth/users', undefined, teacher, 403);
  await call('POST', '/api/auth/users', { username: 'forbidden', display_name: 'Forbidden', role: 'admin', password: ownerPassword }, teacher, 403);
  const csrf = await send('/api/auth/logout', { method: 'POST', headers: { cookie: owner } });
  assert.equal(csrf.status, 403); await csrf.text();
  await call('POST', '/api/auth/logout', undefined, owner, 403, 'https://evil.test');
  await call('PUT', `/api/auth/users/${member.id}`, { active: false }, owner);
  await call('GET', '/api/state', undefined, teacher, 401);
  await call('PUT', `/api/auth/users/${member.id}`, { active: true, password: 'synthetic-reset-password' }, owner);
  teacher = await login('teacher', 'synthetic-reset-password');
  await call('PUT', '/api/auth/password', { current_password: 'synthetic-reset-password', password: 'synthetic-next-password' }, teacher);
  await call('GET', '/api/state', undefined, teacher, 401);
  teacher = await login('teacher', 'synthetic-next-password');
  await call('POST', '/api/auth/logout', undefined, teacher);
  await call('GET', '/api/state', undefined, teacher, 401);
  await expire();
  await call('GET', '/api/state', undefined, owner, 401);
  console.log(`PASS ${label}: login, secure cookies, CSRF, member permissions, disable/reset, password change, logout and expiry`);
}
