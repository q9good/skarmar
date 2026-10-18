# Full Cloudflare application

This deployment runs the static website, Rust Wasm API and SQLite-backed
`SkarmaSpace` Durable Object on Cloudflare. It has no dependency on the host or
the separate VPC gateway.

See the [Chinese deployment and operations guide](../../docs/cloudflare-hosted-deployment.md)
for setup, private first-administrator initialization, verification, updates,
data migration limits and rollback.

Copy `wrangler.example.toml` to the ignored `wrangler.toml`, set the intended
account and exact HTTPS origin, and run deployment from the repository root.
Keep production authentication enabled and sample data disabled. The bootstrap
password must be set with Wrangler secrets, and removed after the administrator
has been created and verified. Never put it in this directory or Git.
