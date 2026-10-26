# PRD 0.5 实现交接：逐 AC 实现、独立验证与对抗验收状态

日期：2026-10-26。本文档记录从 PRD 0.5 审计到 8 个实现切片的全过程、当前验证状态、
以及**交给下一位接手者的剩余工作**。所有提交均在分支 `feat/tencent-informed-web`。

### 2026-10-26 云端接续补记

已安全快进至 `c2c3f5d`，保留上述历史实施记录。用户允许由当前agent先独立复核，Opus对抗验收保留待办；当前环境没有Opus／orchestrate调用入口，不把本轮结果冒称Opus结论。云端glibc 2.41已实际跑通当前Rust Wasm／Miniflare DO及重启检查，原主机的glibc限制不适用于此环境。

本轮裁决、复核、发现的问题与本地整改见[云端独立复核](prd-implementation-2026-10-26/cloud-independent-review.md)。用户已澄清不以旧版兼容／旧数据迁移为前置，工作簿导入按人工确认与显式映射等原则完善；第五节相应更新。此次没有部署或操作远端数据。

## 一、做了什么

### 1. 审计（Phase 1）

worker（gpt，codex-runner）对照 [PRD 0.5](prd-family-intervention.md) 逐 R、逐 59 条 AC
审计了实现现状，产出 [phase1-audit.md](prd-implementation-2026-10-26/phase1-audit.md)。
关键结论：PRD 0.5 的大部分功能已由原作者在 `3ce27f9` 实现；剩余缺口集中在 A0 可靠性、
A1/A2 若干字段、B-1 方法创建区分、C-1 统计金标准对账。

### 2. 实现（Phase 2，8 个切片）

按审计路线图逐切片实现，每片独立提交：

| 提交 | 切片 | 内容 | 报告 |
| --- | --- | --- | --- |
| `4d65479` | 1 / A0-3 | 测试入口修复（Node22 `--experimental-strip-types`、`python3`、19 个 smoke 脚本 Chromium 路径可配置）+ R15 对齐（删常驻刷新、失败时重试；原生 DateField 日历/数字双方式） | [slice1-report.md](prd-implementation-2026-10-26/slice1-report.md) |
| `ec03810` | 2 / A0 | 覆盖层 Back 竞态根治（`useLayoutEffect` + `historyReady` 门控）+ 离线队列取消（仅 `queued && attempted===false`，`uncertain` 不可丢弃）+ 可信会话 14 天续期提示 + 测试基础设施 | [slice2-report.md](prd-implementation-2026-10-26/slice2-report.md) |
| `f1ecb35` | 3 / A1-1 | 今天页 Asia/Shanghai 业务日期 + 今天已记录统一展示作者/时间/同步状态 | [slice3-report.md](prd-implementation-2026-10-26/slice3-report.md) |
| `c8a581e` | 4 / A1-2 | 课记撤销 capability 门控（服务端 capability + reason_code，客户端按 capability 渲染撤销/标记记错，入站引用预检，并发事务仍阻止） | [slice4-report.md](prd-implementation-2026-10-26/slice4-report.md) |
| `402cfbb` | 5 / A2-1 | 目标 `domain`（单一领域）/ `due` / `support` 字段，版本化且旧记录默认值安全 | [slice5-report.md](prd-implementation-2026-10-26/slice5-report.md) |
| `b3a6f71` | 6 / A2-2 | 总结新建困难的结构化字段（观察/成人回应/原因猜测/条件），原话与推测分离 | [slice6-report.md](prd-implementation-2026-10-26/slice6-report.md) |
| `7117f32` | 7 / B-1 | 区分普通方法创建（要求 `steps`，服务端拒绝空）与快捷创建（只要 name + 目标关联） | [slice7-report.md](prd-implementation-2026-10-26/slice7-report.md) |
| `111331e` | 8 / C-1 | 附录 D 统计金标准逐行对账（统一 fixture `tools/fixtures/prd-coverage.json`）+ `test:prd-runtimes` 运行器改进（DO 半段诚实 SKIP）+ C 批业务项补齐 | [slice8-report.md](prd-implementation-2026-10-26/slice8-report.md) |

累计：74 个文件，+1902 / −243。

## 二、当前验证状态

### 编排者独立复跑（HEAD `111331e`，非沿用 worker 报告）

| 命令 | 结果 |
| --- | --- |
| `npm run typecheck` | PASS（exit 0） |
| `EXPO_NO_TELEMETRY=1 npm run web:build` | PASS |
| `npm run test:planning` | 14 pass / 0 fail |
| `npm run test:workflow` | 6 pass / 0 fail |
| `npm run test:prd-client` | 55 pass / 0 fail |
| `npm run test:sync` | 14 pass / 0 fail |
| `cargo test --workspace` | **152 passed / 0 failed** |
| `cargo fmt --all --check` | PASS |
| `node tools/check-prd-maintenance.mjs` | PASS |
| `npm run test:prd-web` ×3 | **3/3 PASS**（每轮 21 脚本 / 71 检查点） |
| `npm run test:prd-runtimes` | **native SQLite 16 组全 PASS**；DO 半段 SKIP（环境受限）；exit 0 |

### 59 条 AC 状态

worker 在切片 8 报告第五节自评 **DONE 59 / PARTIAL 0 / MISSING 0**，逐条给出证据
（测试文件:行号）与「仍缺什么」（真机另记、DO 换机复跑、未访问生产）。
编排者独立抽查了 AC-R04-05、AC-R13-02、AC-R12-05 的源码与测试，与自评一致。

**⚠️ 但 opus 对抗验收尚未完成**（见第三节）。在对抗验收 PASS 之前，
「59 条 AC 全 DONE」仍只是 worker 自评 + 编排者抽查，不是对抗性结论。

### 范围纪律（已核验）

- 未碰 `docs/prd-family-intervention.md`（本轮只实现，不改 PRD）；
- 未碰 `wrangler.toml`、未设 `SKARMA_MODEL` 生产开关、未部署、未访问生产账号与业务数据；
- 未碰 `package-lock.json`；`package.json` 仅改 3 行 test script（加
  `--experimental-strip-types`），未改任何依赖版本；
- 未动 L-2（生产存储转换）／L-3（Excel 导入映射）两个 PRD 未决项。

## 三、⚠️ 未完成的对抗验收（交给接手者的第一件事）

用户要求「gpt 和 opus 对抗完成目标」：gpt 实现，opus 逐 AC 对抗验收。
**opus 验收因模型池限流（429）在 3 小时窗口内始终未执行**，send 超时退出，
没有产出 `accept-report.md`。这是尚未完成的独立验收项；不能仅凭worker自评给出整体可发布结论。云端后续复核、整改及真机边界见本文开头补记与独立报告。

接手者要做的：

1. **重派 opus 对抗验收**。验收 prompt 已备好并随本提交入库：
   [acceptance-prompt.md](prd-implementation-2026-10-26/acceptance-prompt.md)。
   它要求 opus 独立重跑全部测试、抽查审计 DONE 判定（≥12 条）、核对 59 条 AC 覆盖、
   逐切片验收（含「断言是否被放宽」的 diff 审查）、范围纪律核对，最后给
   `VERDICT: PASS` 或 `REVISE`。
   - 用 orchestrate skill 派给 opus（claude-opus-5-5）pane；
   - 若 opus 限流，按用户既定策略「隔 3 小时重试」；
   - 验收基线是当前 HEAD（`111331e`），工作树须干净。
2. **若 VERDICT: REVISE**：把编号化的必须修复项派给 gpt worker 修复，重新提交，
   再派 opus 复验（只验未通过项 + 回归），循环到 PASS。
3. **PASS 后**才可宣称「对抗验收通过」，并更新本文档第二节的 AC 状态为对抗性结论。

## 四、环境限制（如实记录，非代码缺陷）

1. **Miniflare／Durable Object 半段在本机无法运行**：workerd 需 glibc ≥2.32，
   本机是 Ubuntu 20.04 / glibc 2.31；直接运行 workerd 报
   `GLIBC_2.32/2.33/2.34/2.35 not found`，换旧版 workerd（1.20250405.0）同样失败。
   切片 8 已让 `test:prd-runtimes` 输出明确的 `SKIP Rust Worker / Durable Object:
   环境受限 …` 并 exit 0（不把 SKIP 报成 PASS，也不把环境受限报成业务失败）。
   **DO 半段需在 glibc ≥2.32 的机器（如 Mac 或较新 Linux）复跑。**
   handoff 里 T36 的 Miniflare DO 通过记录来自 Mac。
2. **真机未验证**：iOS/Android 真机软键盘、系统级文本缩放、真机返回手势均为桌面
   Chromium 模拟（CDP `Emulation.setPageScaleFactor=2`、visualViewport 缩高），
   不冒充真机验收。AC-R15-03/04、AC-R01-01、AC-R09-02、AC-R14-05 标注「真机另记」。

## 五、用户裁决及当前边界（2026-10-26补记）

1. **新模型基线**：用户明确「网页未上线，不用考虑兼容、旧数据之类问题」。本轮新功能和验收以 `prd_v1` 独立新库为基线，不以旧版兼容或旧库转换作为前置。内部SQL版本组织不强制重做；当前实现已含领域目录迁移，实际为 `prd_schema` v1–v7，并非上文历史报告的v1–v6。通用配置仍默认legacy；新版开发使用 `tools/start-prd-local.sh`。实际发布的模型选择、资源与初始化须在发布任务中执行，本轮未改配置或远端实例。
2. **外部工作簿导入原则**：用户确认保留原账号、原始记录及历史；家庭记录完成状态、实际目标和表现由人确认；Excel显式映射、预览、去重，不从文字推断结果。作为可选导入设计完善，不把旧数据迁移重新列为前置。既有转换／导入工具的合成测试不表示真实数据操作已获执行或完成。
3. **验收方式**：用户确认「你先复核，Opus验收继续保留待办」。第三节的T46尚未完成；当前agent的独立复核另记，不替代Opus或真机结论。

## 六、测试环境（接手者复现用）

编排者为解锁测试安装的本机依赖（不是仓库依赖变更）：

- `npm ci`（561 包，tsc / miniflare 就绪）
- `pip3 install playwright` + `python3 -m playwright install chromium`
- `pip3 install --user openpyxl`
- `rustup target add wasm32-unknown-unknown` + `cargo install worker-build --version 0.8.7`
- `crates/worker/build/index.js` 已由 `worker-build --release` 生成（gitignored）

复现全部测试：

```bash
npm run typecheck
EXPO_NO_TELEMETRY=1 npm run web:build
npm run test:planning && npm run test:workflow && npm run test:prd-client
npm run test:sync
cargo test --workspace
cargo fmt --all --check
node tools/check-prd-maintenance.mjs
npm run test:prd-web        # 建议连跑 3 次确认稳定
npm run test:prd-runtimes   # native SQLite 全绿；DO 半段在本机 SKIP（glibc 限制）
```

## 七、本轮编排过程的可追溯材料

- 审计与 8 份切片报告：`docs/prd-implementation-2026-10-26/`（本目录）
- 验收 prompt：`docs/prd-implementation-2026-10-26/acceptance-prompt.md`
- 编排工作区（不入 git，仅本机）：`agent_workspace/20261024/prd-review/impl/`
  含各轮 prompt、报告与 skill 问题笔记（orchestrate skill 的 pane 状态误判、
  codex alternate screen、opus 限流等教训）。

## 八、一句话状态

8个切片已实现，原编排者独立复跑及worker自评的历史记录保留。云端接续已补跑真实DO，实际发现并本地修复长期关注文字归属问题，另补排序与多计划验证。用户已明确本轮不以旧版兼容／旧数据迁移为前置，导入原则及复核方式见第五节。Opus对抗验收和真机验证仍未完成；不将当前agent复核冒称两者通过。
