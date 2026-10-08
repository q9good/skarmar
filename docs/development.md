# 开发与当前验证范围

## 技术与依赖

- Rust 1.99.0（rust-toolchain.toml），Axum API，bundled SQLite，不依赖系统数据库服务。
- Node 24，Expo SDK 57、React Native 0.86、React 19 和 TypeScript。
- Cargo.lock 和 package-lock.json 固定当前安装依赖。
- Expo 依赖版本依据所安装 SDK 的 bundledNativeModules.json；在线 Expo 版本服务在当前
  网络策略下返回 Forbidden。静态构建、类型检查和实际浏览器流程单独验收，离线的
  expo install --check 不作为完整在线兼容性校验的替代。

## 运行

当前云环境可在仓库根目录执行 `bash tools/install-cloud.sh`，它将 Rust 和 npm cache
保存在可写的 `/workspace` 下，按 lockfile 安装并构建。以下通用步骤适用于已经安装
Rust 与 Node、且 npm cache 可写的开发机。

在仓库根目录执行：

```bash
npm ci
npm run web:build
bash tools/start-local.sh
```

启动脚本在云环境激活 `/workspace/.toolchains/` 中的 Rust；普通开发机使用已安装的
Rust。API 同时提供 Web 静态资源，开发端口为 3001，仅监听 127.0.0.1。
脚本默认使用 `.local/demo.db`，仅在没有目标时加入明确的示例目标，不清空现有数据库。
首次安装 Rust 时按 rust-toolchain.toml 安装对应版本及 rustfmt、clippy。

UI 修改需要热更新时，保持 API 运行，在 `apps/client` 执行：

```bash
EXPO_PUBLIC_API_URL=http://localhost:3001 EXPO_NO_TELEMETRY=1 npx expo start --web --offline --port 8081
```

这是本机开发配置；本机 localhost 与云环境开发端口不能直接作为手机公网服务。
公开使用前须实现登录、服务端授权并完成正式部署。Web 静态构建采用同源 API，
二期 App 的远程 API 地址与原生登录另行配置和验证。

## 检查

在仓库根目录执行：

```bash
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run typecheck
npm run web:build
```

Rust 集成测试覆盖草稿／完成规则、角色与目标范围、来源生成、历史快照、重复操作、
版本冲突、并发 HTTP 更新，以及故障注入后的数据库事务回滚。

浏览器验收用可删除的独立数据库运行服务器：

```bash
SKARMA_DATABASE=.local/validation.db SKARMA_DEMO=1 bash tools/start-local.sh
python tools/smoke_web.py
```

服务器必须从仓库根目录启动，验收库必须为空。每次运行可指定一个新的 validation
数据库文件；不要复用或删除实际使用的数据库来满足验收前提。
浏览器脚本依赖 Python Playwright 与 Chromium；当前云镜像已提供这两项。其他环境需
安装 Playwright，并通过 CHROMIUM_EXECUTABLE 指定 Chromium 的可执行文件位置。

该脚本在 390 × 844 的 viewport 验证实际 UI：计划与回填共用 ID、本机草稿恢复、
提交已在服务端完成但响应被网关错误替代后的重试、困难与经验生成、归档时快照及跟进来源跳转。
截图输出到被 Git 忽略的 `.local/screenshots/`。这是浏览器模拟尺寸，真机验收另行完成。

## 数据与阶段边界

两份需求附件与 token 位于仓库外。开发示例不导入真实个训正文，不将个人凭证作为
客户端配置。当前服务没有生产登录与档案授权，实际业务数据进入系统前应先完成这些
能力；具体未完项见 product-plan.md 的“本次工程选择与原型边界”。

后端为本地原型选择 SQLite 并把实体内容存为 JSON，关系与幂等操作使用 SQL 约束。
正式存储结构、查询索引、版本化迁移、修订记录和备份方案会随部署设计进一步确定。
首次初始化与重复启动可以保留数据库，不代表升级迁移及备份恢复已经通过验收。
