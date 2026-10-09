# Rust Workers 本地验证探针

独立于应用工作区，仅用于验证 Rust + Axum 和 SQLite-backed Durable Objects 的事务接口。这里的 `/probe/*` 使用合成数据并通过 GET 执行测试动作，是本地验证接口。完整应用没有因此迁移或部署。

已验证版本：Rust 1.99、worker / worker-build 0.8.7、Axum 0.8、Wrangler 4.149.0。

在仓库根目录准备构建工具：

```bash
rustup target add wasm32-unknown-unknown
cargo install worker-build --version 0.8.7 --locked
```

启动本地 Worker（使用 Wrangler 的本地模式，无需远端部署）：

```bash
cd tools/cloudflare-probe
npx --yes wrangler@4.149.0 dev --local --ip 127.0.0.1 --port 8787
```

在另一个终端验证 HTTP 与事务：

```bash
cd tools/cloudflare-probe
python3 check.py
```

期望：健康接口 200；失败事务 409 后查询行数为 0；成功事务后查询行数为 2。结束时清空合成记录。

仅检查部署包，不执行远端部署：

```bash
npx --yes wrangler@4.149.0 deploy --dry-run --outdir build/dry-run
```

受限云环境中使用已配置的 Rust 路径，缓存目录可设置为可写的 `XDG_CACHE_HOME`、`XDG_CONFIG_HOME` 和 `npm_config_cache`，不需要改变系统 HOME 或关闭 TLS 验证。

该探针证明 Rust HTTP 和事务绑定可工作；完整训练业务、幂等/版本冲突、线上 CPU 和手机网络仍需另行验收。详见 [部署调研](../../docs/cloudflare-deployment-research.md)。
