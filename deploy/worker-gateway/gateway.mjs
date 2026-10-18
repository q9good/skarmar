const PRIVATE_SERVICE = "http://localhost:3001";

function unavailable() {
  return new Response("Service unavailable", {
    status: 503,
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "no-store",
    },
  });
}

export default {
  async fetch(request, env) {
    try {
      const incoming = new URL(request.url);
      const target = new URL(PRIVATE_SERVICE);
      // Assign path and query separately so neither can replace the service host.
      target.pathname = incoming.pathname;
      target.search = incoming.search;

      const headers = new Headers(request.headers);
      headers.set("Host", target.host);
      const proxyRequest = new Request(target, {
        method: request.method,
        headers,
        body: request.method === "GET" || request.method === "HEAD" ? undefined : request.body,
        redirect: "manual",
        signal: request.signal,
        // Needed by Node's streaming Request in tests; ignored by Workers.
        duplex: "half",
      });

      // The service binding is the only upstream. Never use global fetch here.
      const upstream = await env.VPC_SERVICE.fetch(proxyRequest);
      const response = new Response(upstream.body, upstream);
      if (incoming.pathname === "/api" || incoming.pathname.startsWith("/api/")) {
        response.headers.set("Cache-Control", "no-store");
      }
      return response;
    } catch {
      return unavailable();
    }
  },
};
