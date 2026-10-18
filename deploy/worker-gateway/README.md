# Worker gateway to the existing host

This small Worker publishes a stable `workers.dev` HTTPS address and forwards
requests through a Workers VPC Service to the app at `http://localhost:3001` on
the host running `cloudflared`. The existing native app and SQLite database stay
on that host. This gateway does not use the separate Durable Object app in the
root Wrangler configuration.

## Configure and deploy

1. Create a named Cloudflare Tunnel and run its connector as a persistent service
   on the app host. Confirm it is connected. The host must be able to reach
   Cloudflare; the Worker cannot fix a disconnected connector.
   Workers VPC requires `cloudflared` 2025.7.0 or later and QUIC on outbound UDP
   7844. HTTP/2 is not supported for this connection. The included user-service
   example forces IPv4 for hosts whose existing proxy TUN covers IPv4 only;
   adjust that choice for the actual network. Keep the run token in a private
   mode-0600 file and use `--token-file`, rather than putting it in command-line
   arguments. Enable the service and the user's lingering manager as appropriate
   for the host, so it reconnects without an open SSH session.
2. Create an HTTP Workers VPC Service for that tunnel, with hostname `localhost`
   and HTTP port `3001`. It grants this binding access to the one app service.
3. Copy `wrangler.example.toml` to `wrangler.toml` in this folder. Replace
   `REPLACE_WITH_VPC_SERVICE_ID` with that service ID. Set the account and Worker
   name for the intended account. The local file is ignored by Git.
4. Run from the repository root:

   ```sh
   node --test deploy/worker-gateway/test.mjs
   npx wrangler deploy --config deploy/worker-gateway/wrangler.toml --dry-run --outdir .local/worker-gateway-dry-run
   npx wrangler deploy --config deploy/worker-gateway/wrangler.toml
   ```

5. Add the exact deployed HTTPS origin, such as
   `https://skarma-gateway.ACCOUNT_SUBDOMAIN.workers.dev`, to the host app's
   `SKARMA_ADDITIONAL_PUBLIC_URLS` (comma separated), keeping its existing
   `SKARMA_PUBLIC_URL`. Restart the app after the configuration change. No wildcard
   origin is allowed. Keep `SKARMA_PRODUCTION=1` and `SKARMA_AUTH=required`.
6. Check homepage and health access, unauthenticated `/api/state` rejection,
   login, an authorized write, and rejection of a foreign `Origin`. Keep real
   credentials and account configuration outside Git.

The gateway preserves request method, path/query, body, `Origin`, cookies and
`X-Skarma-Request`. The backend continues to authenticate sessions and check the
exact allowed origin. Secure `__Host-skarma_session` cookies pass through without
rewriting; each browser hostname has its own login session. API responses receive
`Cache-Control: no-store`. Upstream redirects are returned without following them.
The upstream URL is fixed in code; request parameters and headers cannot select
another service. Errors produce a plain 503 without internal error details.

Workers VPC is currently beta. The connector must remain online for requests to
reach the host; the stable Worker address alone does not provide host availability.

Official setup references:

- [Workers VPC get started](https://developers.cloudflare.com/workers-vpc/get-started/)
- [Private API / website example](https://developers.cloudflare.com/workers-vpc/examples/private-api/)
- [VPC Tunnel transport requirements](https://developers.cloudflare.com/workers-vpc/configuration/tunnel/)
