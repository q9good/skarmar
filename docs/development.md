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
bash tools/start-local.sh admin-init --username owner --name 家长
bash tools/start-local.sh
```

启动脚本在云环境激活 `/workspace/.toolchains/` 中的 Rust；普通开发机使用已安装的
Rust。API 同时提供 Web 静态资源，开发端口为 3001，仅监听 127.0.0.1。
本轮 worktree 位于 `/workspace/worktrees/skarmar-web`，分支 `feat/tencent-informed-web`。
为与原 checkout 的开发服务并行，可在此 worktree 根目录使用 `SKARMA_PORT=3002 bash tools/start-local.sh`。
脚本默认使用 `.local/demo.db`，仅在没有目标时加入明确的示例目标，不清空现有数据库。
首次使用先执行上面的 `admin-init`，隐藏输入密码；账号与启动服务使用同一数据库。已有账号时跳过初始化。
首次安装 Rust 时按 rust-toolchain.toml 安装对应版本及 rustfmt、clippy。

UI 修改需要热更新时，保持 API 运行，在 `apps/client` 执行：

```bash
EXPO_PUBLIC_API_URL=http://localhost:3001 EXPO_NO_TELEMETRY=1 npx expo start --web --offline --port 8081
```

这是本机开发配置；本机 localhost 与云环境开发端口不能直接作为手机公网服务。
已实现账号登录、管理员成员管理与服务端会话校验；主机正式部署见 [Tunnel 部署指南](host-deployment.md)。Web 静态构建采用同源 API，
二期 App 的远程 API 地址与原生登录另行配置和验证。

## 检查

本地与 Cloudflare 的配置、启动及双存储验收命令见 [双环境运行说明](runtime-storage.md)。
`npm run cf:dev` 使用 Worker 入口的本机模拟，与本地 SQLite 服务共享业务和 API；不执行远端部署。

在仓库根目录执行：

```bash
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python -m unittest discover -s tools -p 'test_*.py' -v
npm run typecheck
npm run test:planning
npm run web:build
npm run test:host
npm run test:runtimes
```

Rust 集成测试覆盖草稿／完成规则、角色与目标范围、来源生成、历史快照、重复操作、
版本冲突、并发 HTTP 更新，以及故障注入后的数据库事务回滚；当前共 37 项，包含 8 项认证、2 项 Excel 写入及 6 项途径／经验关系验收。
认证覆盖会话 Cookie、退出和过期、来源校验、失败限速、成员权限、密码修改，以及排队的业务／导出请求在事务内重新验证身份。Excel 写入覆盖空空间工作簿与过长 Unicode 单元格拒绝。
新增目标验收覆盖状态与归档独立、长期内化保留、完成目标复习、复盘日期、目标历史的事务回滚、
旧字段兼容及显式重新保存计划后采用新目标版本。Python 测试使用合成数据和虚构凭证，验证字段
映射、分页完整性、悬空关系分类及读取工具的凭证／输出边界。

浏览器验收用可删除的独立数据库运行服务器：

```bash
SKARMA_DATABASE=.local/validation.db SKARMA_DEMO=1 SKARMA_AUTH=demo bash tools/start-local.sh
python tools/smoke_web.py
```

服务器必须从仓库根目录启动，验收库必须为空。每次运行可指定一个新的 validation
数据库文件；不要复用或删除实际使用的数据库来满足验收前提。
worktree 使用不同端口时，同时设置服务器的 `SKARMA_PORT=3002` 与验收脚本的
`SKARMA_TEST_URL=http://127.0.0.1:3002`。
浏览器脚本依赖 Python Playwright 与 Chromium；当前云镜像已提供这两项。其他环境需
安装 Playwright，并通过 CHROMIUM_EXECUTABLE 指定 Chromium 的可执行文件位置。

配置了出站 HTTP 代理的开发环境，Python 的回环测试需要在 `no_proxy`／`NO_PROXY` 中包含
`127.0.0.1,localhost`，并保留原有条目。仅为本机验证设置这些例外；远端服务继续使用环境代理。

该脚本在 390 × 844 的 viewport 验证实际 UI：计划与回填共用 ID、本机草稿恢复、
提交已在服务端完成但响应被网关错误替代后的重试、困难与经验生成、归档时快照及跟进来源跳转；
也覆盖目标复盘、目标并发冲突时保留与备份填写内容、待复盘入口及已完成目标的候选范围。
截图输出到被 Git 忽略的 `.local/screenshots/`。这是浏览器模拟尺寸，真机验收另行完成。

[Excel 导出](excel-export.md) 已接入“我的账号”。`test:host` 验证实际手机尺寸浏览器下载、成员入口、未保存草稿排除、异常网关文件拒绝及会话失效；`test:runtimes` 用相同 XLSX 合约验证原生／Wasm 导出的完整内容、历史快照与关系、公式字符按文字保留以及下载权限。

[目的、途径与经验](purpose-pathway-experience.md) 增加独立途径目录和三组多对多关系。`test:host` 还运行 `smoke_knowledge_web.py`，验证手机多选、独立原则／细节、两类并发编辑时保留内容、归档及十表导出；`test:runtimes` 在两入口验证关系中途写入失败的事务回滚与重启保留。共享迁移 `004_pathways.sql` 保留旧经验来源与分类，不自动猜测绑定。

[规划总览](planning-overview.md) 用 `react-native-svg` 15.15.4 绘制环形图，版本依据 Expo SDK 的 `bundledNativeModules.json`。`npm run test:planning` 使用 Node 24 内置 TypeScript 支持运行 11 项统计测试；`test:host` 增加 `smoke_planning_web.py`，验证真实训练的筛选、父子去重、未覆盖提醒、途径范围、草稿保留与角色分配，以及空空间／合并图例和手机、桌面布局。该功能仅消费现有登录 API，不新增 SQL 迁移。

[跨环境交接](handoff.md) 用 Git 同步已提交的代码、方案、任务快照和选定合成截图。`npm run sync:status` 读取本地状态并校验产物，`sync:pull` 仅快进拉取，`sync:push` 推送已提交工作。`npm run test:sync` 的 14 项测试在独立临时仓库中验证双向同步、分叉与未提交内容保护、忽略文件保留及产物完整性，不使用用户实际仓库演练冲突。

## 数据与阶段边界

两份需求附件与 token 位于仓库外。开发示例不导入真实个训正文，不将个人凭证作为
客户端配置。当前账号授权针对一个共享空间；逐档案权限和正式数据导入仍需实现。
主机、真实域名与手机网络验收尚待执行，具体未完项见 product-plan.md 的“本次工程选择与原型边界”。

两种存储都把实体内容存为 JSON，关系与幂等操作使用 SQL 约束。共享迁移位于
`crates/core/migrations`，已验收旧原型库补建迁移版本记录、重复启动保留数据及拒绝更高版本库。
两个入口还通过同一套 API 合约验收，包括 SQL 中途失败全回滚和重启保留数据。
原生存储的一致性备份／离线恢复已有验收，Docker 镜像与部署脚本可用 `npm run test:container` 验证（先构建镜像）。
正式数据导入、更多版本升级、逐档案权限和真实主机公网访问仍待实现或验收。

腾讯线上核验和附件关系审计的命令、证据边界见 [腾讯核验说明](tencent-audit.md)。

2026-10-19 六页交互改进见 [页面流程](page-workflows.md)：`test:host` 增加 `smoke_page_workflows.py` 的双浏览器刷新／冲突、日期提醒、家庭干预总结、机构课时及停用选项验收。迁移 008 追加共享课程和老师选项，保留历史记录；前端纯逻辑另执行 `node --test tools/test-workflow.mjs`。
