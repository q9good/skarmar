import assert from "node:assert/strict";
import { test } from "node:test";
import gateway from "./gateway.mjs";

function service(fetch) {
  return { VPC_SERVICE: { fetch } };
}

test("forwards login body, origin, request marker and cookie; returns secure cookies intact", async () => {
  const cookies = [
    "__Host-skarma_session=synthetic; Path=/; Secure; HttpOnly; SameSite=Lax",
    "second=synthetic; Expires=Wed, 21 Oct 2037 07:28:00 GMT; Path=/; Secure",
  ];
  const request = new Request("https://skarma-gateway.example.workers.dev/api/auth/login?next=%2Ftoday&next=a+b", {
    method: "POST",
    headers: {
      Origin: "https://skarma-gateway.example.workers.dev",
      Cookie: "__Host-skarma_session=old-synthetic",
      "Content-Type": "application/json",
      "X-Skarma-Request": "1",
    },
    body: JSON.stringify({ username: "synthetic", password: "synthetic-only" }),
  });
  const response = await gateway.fetch(request, service(async (upstream) => {
    assert.equal(upstream.url, "http://localhost:3001/api/auth/login?next=%2Ftoday&next=a+b");
    assert.equal(upstream.method, "POST");
    assert.equal(upstream.redirect, "manual");
    assert.equal(upstream.headers.get("Host"), "localhost:3001");
    assert.equal(upstream.headers.get("Origin"), "https://skarma-gateway.example.workers.dev");
    assert.equal(upstream.headers.get("Cookie"), "__Host-skarma_session=old-synthetic");
    assert.equal(upstream.headers.get("X-Skarma-Request"), "1");
    assert.deepEqual(await upstream.json(), { username: "synthetic", password: "synthetic-only" });
    const headers = new Headers({ "Content-Type": "application/json", "Cache-Control": "public, max-age=3600" });
    for (const cookie of cookies) headers.append("Set-Cookie", cookie);
    return new Response('{"ok":true}', { status: 200, headers });
  }));
  assert.equal(response.status, 200);
  assert.equal(response.headers.get("Cache-Control"), "no-store");
  assert.deepEqual(response.headers.getSetCookie(), cookies);
  assert.deepEqual(await response.json(), { ok: true });
});

test("never follows redirects or chooses an upstream from path, query or headers", async (t) => {
  const globalFetch = t.mock.method(globalThis, "fetch", () => {
    throw new Error("Global fetch must never be used");
  });
  for (const path of [
    "//other.invalid/secret?url=https%3A%2F%2Fother.invalid",
    "/https://other.invalid/secret?upstream=https://other.invalid",
    "/api/state?target=http://169.254.169.254/",
  ]) {
    let calls = 0;
    const response = await gateway.fetch(new Request(`https://gateway.example.workers.dev${path}`, {
      headers: { Host: "other.invalid", "X-Forwarded-Host": "other.invalid" },
    }), service(async (upstream) => {
      calls += 1;
      assert.equal(new URL(upstream.url).origin, "http://localhost:3001");
      assert.equal(upstream.headers.get("Host"), "localhost:3001");
      assert.equal(upstream.redirect, "manual");
      return new Response(null, { status: 302, headers: { Location: "https://other.invalid/secret" } });
    }));
    assert.equal(calls, 1);
    assert.equal(response.status, 302);
    assert.equal(response.headers.get("Location"), "https://other.invalid/secret");
  }
  assert.equal(globalFetch.mock.callCount(), 0);
});

test("keeps rejected origins and authentication responses for the app to enforce", async () => {
  const response = await gateway.fetch(new Request("https://gateway.example.workers.dev/api/state", {
    method: "DELETE",
    headers: { Origin: "https://foreign.invalid", "X-Skarma-Request": "1" },
  }), service(async (upstream) => {
    assert.equal(upstream.method, "DELETE");
    assert.equal(upstream.headers.get("Origin"), "https://foreign.invalid");
    return new Response('{"error":"forbidden"}', { status: 403, headers: { "Content-Type": "application/json" } });
  }));
  assert.equal(response.status, 403);
  assert.equal(response.headers.get("Cache-Control"), "no-store");
  assert.deepEqual(await response.json(), { error: "forbidden" });
});

test("streams assets and HEAD without converting the body or response metadata", async () => {
  const bytes = new Uint8Array([0, 255, 2, 128]);
  const response = await gateway.fetch(new Request("https://gateway.example.workers.dev/assets/app.bin?v=2"), service(async (upstream) => {
    assert.equal(upstream.method, "GET");
    assert.equal(upstream.body, null);
    return new Response(bytes, { headers: { "Content-Type": "application/octet-stream", ETag: '"asset-v2"' } });
  }));
  assert.deepEqual(new Uint8Array(await response.arrayBuffer()), bytes);
  assert.equal(response.headers.get("ETag"), '"asset-v2"');
  const head = await gateway.fetch(new Request("https://gateway.example.workers.dev/", { method: "HEAD" }), service(async (upstream) => {
    assert.equal(upstream.method, "HEAD");
    assert.equal(upstream.body, null);
    return new Response(null, { status: 200 });
  }));
  assert.equal(await head.text(), "");
});

test("returns a plain uncached 503 when service fails or binding is missing", async () => {
  for (const env of [service(async () => { throw new Error("private hostname, token and stack must not escape"); }), {}]) {
    const response = await gateway.fetch(new Request("https://gateway.example.workers.dev/api/state"), env);
    assert.equal(response.status, 503);
    assert.equal(response.headers.get("Cache-Control"), "no-store");
    assert.equal(await response.text(), "Service unavailable");
  }
});
