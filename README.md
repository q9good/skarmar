# skarma ⭐

> Every star shines at its own pace.

**skarma** is a lightweight, open-source tool for tracking the daily training 
and developmental progress of children on the autism spectrum (ASD). 
It helps parents, caregivers, and therapists log routines, milestones, 
and small daily wins — turning scattered notes into a clear, 
long-term record of growth.

## Why "skarma"?

The name comes from the Tibetan word *སྐར་མ* (*skar ma*), meaning **"star"** — 
a quiet nod to the Chinese term "星星的孩子" ("children of the stars"), 
a gentle and widely used way of referring to children with autism. 
Spelled almost identically to the English loanword *karma*, the name also 
carries a second meaning: **small, consistent actions accumulate into 
meaningful results over time** — which is exactly what daily training 
and consistent record-keeping are about.

## Features

- 📅 **Daily logging** — record training sessions, behaviors, and routines 
  in just a few taps.
- 📈 **Progress tracking** — visualize milestones and trends over weeks, 
  months, and years.
- 📝 **Caregiver notes** — capture context (mood, triggers, environment) 
  alongside each entry.
- 🔒 **Privacy-first** — your child's data stays local or under your own 
  control; no third-party tracking.
- 🧩 **Extensible** — designed to grow with new modules (e.g. visualization 
  dashboards, reminders, export reports) as the project evolves.

## Status

🚧 Early development — contributions, feedback, and ideas from parents, 
therapists, and developers are all welcome.

## Development prototype

The first phase is a mobile browser application built with React Native and Expo Web,
backed by a Rust HTTP API. iOS and Android apps are planned for phase two.

The prototype supports training plans, per-goal results, difficulty and experience
follow-up, draft recovery, and versioned goal reviews. It shares Rust business rules and API
routes across native SQLite and Cloudflare Durable Objects storage. Account login,
administrator member management, reusable pathways with many-to-many goal mappings,
experience bindings, and authenticated Excel export are implemented. Self-hosting supports
private HTTPS through Tailscale Serve and public access through Cloudflare Tunnel.
Remote host deployment is still pending.

- [Product plan and requirements (中文)](docs/product-plan.md)
- [Development and validation (中文)](docs/development.md)
- [Tencent snapshot and live audit status (中文)](docs/tencent-audit.md)
- [Cloudflare free deployment research and Rust verification (中文)](docs/cloudflare-deployment-research.md)
- [Native / Cloudflare configuration and startup (中文)](docs/runtime-storage.md)
- [Host deployment with Tailscale / Cloudflare, login and backups (中文)](docs/host-deployment.md)
- [Stable workers.dev gateway: deployment, network diagnosis and rollback (中文)](docs/cloudflare-workers-vpc-deployment.md)
- [Excel download and included data (中文)](docs/excel-export.md)
- [Purpose, reusable pathways and experience bindings (中文)](docs/purpose-pathway-experience.md)
- [Planning overview, goal progress and balanced training coverage (中文)](docs/planning-overview.md)
- [Cloud / local development handoff, tasks and artifacts (中文)](docs/handoff.md)

```bash
npm ci
npm run web:build
bash tools/start-local.sh admin-init --username owner --name Parent
bash tools/start-local.sh
```

The initialization command prompts for a password and uses the same database as the startup
script. Run it once. For a synthetic-data-only anonymous demo, explicitly set `SKARMA_AUTH=demo`.

For the Cloudflare runtime on your computer, install `worker-build` 0.8.7 and the
`wasm32-unknown-unknown` target, then run `npm run cf:dev`. See the runtime guide above.

## License

[MIT](./LICENSE)
