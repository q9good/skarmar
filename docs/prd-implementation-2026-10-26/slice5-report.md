# Phase 2 切片 5 实施报告（A2-1：目标 domain／due／基础帮助条件）

- 基线：`c8a581e`；结束提交：`402cfbb`（`feat: add goal domain due and support`）；日期：2026-10-25（UTC）
- 分支：`feat/tencent-informed-web`；提交后工作树干净；仅本地 commit，未 push。
- 测试数据边界：所有服务端、浏览器和导出测试均使用临时／合成数据库；未访问生产账号、生产绑定或业务数据。

## 一、domain（领域）

### PRD／附录 B 依据

附录 B 原文：

> `goal.domain | 领域／可选 | 需要标签帮助查找吗？ | 选择或自定义 | 单一标签控件`

R06 原文同时要求：

> “领域只有一个常用标签＋自定义控件，不双重录入。”

### 改动

- `crates/core/src/foundation/model.rs:41`：目标新增 `domain: Option<CatalogSnapshot>`，保存稳定 ID、目录版本、种类与名称快照。
- `crates/core/src/foundation/model.rs:121`、`crates/core/src/foundation/mod.rs:432`：目录新增 `CatalogKind::Domain`，与课程／老师共用 `id + version + active` 写入、重命名、停用和修订历史机制。
- `crates/core/src/foundation/catalog_v7.sql:1`、`crates/core/src/foundation/mod.rs:136`：新模型升级到 schema v7；目录唯一索引加入 `domain`，v1–v6 数据原样迁移。
- `crates/core/src/foundation/mod.rs:283`：目标写入时校验领域种类，并从当前仍在使用的领域目录生成可信快照；已有目标未改领域时保留历史快照，即使目录后来改名或停用，也不阻塞其他字段修订。
- `apps/client/src/foundation-view.tsx:51`：新增单一领域控件。选择已有项会替换当前单值；“自定义领域”先写入共享目录，再选入当前目标；不存在数组、多选或“选择＋另填文本”双重来源。
- `apps/client/src/foundation-view.tsx:280`、`apps/client/src/foundation-view.tsx:330`：提供领域目录维护入口，支持新增、改名、停用和恢复。
- `apps/client/src/foundation.ts:4`：前端类型显式保存目录快照；`newGoal()` 默认 `domain: null`。

### 选项集合来源、可选性与实测

PRD 未穷举领域集合，因此没有在前端硬编码领域名。领域项全部来自稳定共享目录，用户可选择或创建自定义项。空值为 `null`，只填名称和层级仍可保存。

实测覆盖：

- Rust 合成测试创建领域目录、保存目标、改名目录、部分更新目标并验证旧名称／版本快照仍保留，再显式清空领域。
- 浏览器测试实际创建“测试沟通互动”领域并选入目标，服务端回读确认 `kind=domain`，且目录仅一条稳定记录。
- 三轮完整浏览器套件连续全绿，每轮均包含领域创建与保存链路。

## 二、due（预期时间）

### 附录 B 依据

附录 B 原文：

> `goal.due | 目标日期／可选 | 预计何时回看目标？ | 不设截止也可以 | 不设统一发育期限`

### 改动

- `crates/core/src/foundation/model.rs:58`：目标新增可选 `due: Option<String>`。
- `crates/core/src/foundation/mod.rs:279`：非空日期走现有 ISO 业务日期校验，非法日期拒绝且不写入新版本。
- `apps/client/src/foundation-view.tsx:105`：在“更多设置”内复用已有 `DateField`；保留日历／8 位数字两种输入方式，没有新造日期控件。
- `apps/client/src/controls.tsx:18`：仅为现有 `DateField`补充可选的常驻问句、空值文案和可展开说明能力，其他调用保持原行为。
- 页面原文显示“预计何时回看目标？”、“不设截止也可以”和“不设统一发育期限”；长期方向也只是家庭自填的回看日期，不会自动生成截止日、倒计时、逾期告警或发育期限。

### 实测结果

浏览器用数字方式填写 `20270120`，保存后服务端为 `2027-01-20`；Rust 测试验证合法日期可保存、`2027-02-31` 被拒绝、显式 `null` 可清空。未填写时仍为 `null`，不影响保存。

## 三、基础帮助条件

### 现状核对

R06 要求目标表达“情境—期待行为—允许帮助—判断标准”。PRD 还明确：

> “目标标准、实际测量、帮助条件与指标版本分别保存。”

改动前，`GoalInput` 只有 `criteria`；“帮助条件”仅存在于后续观察指标／实际测量，目标本身没有清晰承载，步骤模板中的“哪个步骤需要什么帮助？”也不能覆盖所有模板。因此现状属于缺失，而不是已有字段换文案即可。

### 补充内容

- `crates/core/src/foundation/model.rs:45`：目标新增独立可选 `support: String`，与观察标准分开版本化。
- `apps/client/src/field-guidance.ts:43`：把项目原已存在的“预期帮助条件”引导收拢到 `goal.support`：
  - 问句：“准备在什么帮助条件下观察？”
  - placeholder：“例如：先不提示，需要时再示范”
  - 说明：允许的提醒或帮助与观察标准分开。
- `apps/client/src/foundation-view.tsx:101`：在观察标准之后显示“预期帮助条件（可选）”；空字符串可保存。
- `apps/client/src/foundation-view.tsx:307`、`apps/client/src/foundation-view.tsx:378`：目标列表和详情能回看已填帮助条件。

该字段没有变成必填，也没有新增五问校验。§1.3 的适度挑战说明仍属于示例／帮助内容，不会自动写入字段。

### 实测结果

Rust 测试验证帮助条件创建、部分更新保留、显式空字符串清空；浏览器真实填写并从服务端状态回读。名称＋层级的最小保存路径仍通过原有和新增测试。

## 四、版本化与旧记录默认值安全

- `domain`、`due`、`support`位于 `GoalInput`，因此沿用既有 `expected_version + operation_id + change_note` 事务与修订历史，不存在绕过版本控制的旁路表单。
- `Write::deserialize` 从原始 `mutation.value` 收集字段 presence mask；`merge_update` 只覆盖请求明确提供的字段。旧客户端省略三个字段时保留原值；显式 `null`／空字符串才表示清空。
- 三个字段都有 serde 默认值：旧记录读取为 `domain=None`、`due=None`、`support=""`，不会报错，也不会把缺失字段解释为用户主动清空。
- `apps/client/src/foundation.ts:57`：本机旧目标草稿允许缺少新增字段，避免升级后误判为损坏；新的草稿仍显式带安全默认值。
- `crates/api/tests/foundation.rs:429`：合成测试覆盖稳定领域快照、due 校验、support、presence mask 的省略／显式清空，以及不含新字段的旧目标反序列化。
- `crates/api/tests/foundation_lessons.rs:587`：从历史 v1 开发库逐级升级至 v7 后，目标和修订历史保持不变。

## 五、导入导出边界

### 导出侧

- 新模型目标表直接导出完整 `GoalInput`，新增字段自动进入“目标”工作表和“原始数据分段”。
- `crates/http/src/prd_export.rs:57`：补充人类可读列名“领域”“目标日期”；“帮助条件”沿用已有标签。
- `crates/core/src/foundation/export.rs:135`、`tools/verify-prd-export.py:6`：导出数据库版本标记更新为 7。
- `tools/smoke_prd_export_web.py:35`：真实浏览器下载 XLSX，并断言“填写 · 领域”“填写 · 目标日期”“填写 · 帮助条件”三列及其值存在；原始 JSON 仍保留完整目录快照。

### 明确未做

没有新增或修改 Excel 导入规则、字段映射或旧存储转换逻辑。L-3（Excel 导入映射）和 L-2（生产存储转换）仍是 PRD 未决项。

## 六、前端与 R15 三层引导

- `apps/client/src/foundation-view.tsx:93`：保留名称、层级主路径；domain、due 以及其他可选说明位于明确标注的“更多设置”折叠区。
- domain 常驻标签“领域／可选”、常驻问句“需要标签帮助查找吗？”、空值提示“选择或自定义”；由于附录 B 没有提供合成长例子，第三层使用可展开的原文控件说明，不伪造示例。
- due 常驻标签“目标日期／可选”、常驻问句“预计何时回看目标？”、空值提示“不设截止也可以”，可展开“不设统一发育期限”。
- support 通过 `GuidedField` 提供常驻标签、≤20 字问句、短 placeholder 和“看看怎么写（可选）”的合成完整示例。
- 示例／说明只存在于 placeholder 或组件本地展开状态；没有调用 `change`，因此不会进入 value、请求、统计或导出。
- `apps/client/src/foundation-view.tsx:236`：表单候选集仍由 `useStableOptions` 冻结；刚由用户创建并选中的领域按稳定 ID 纳入当前表单，后台刷新不会偷偷替换选择。
- 为消除完整浏览器套件发现的既有随机目录排序假设，测试改为按老师 ID 查找；课记卡增加只用于测试定位的稳定 `testID`（`apps/client/src/prd-lesson-view.tsx:315`），不改变业务行为或断言强度。

## 七、全量测试实跑结果

以下均为本轮在 operator host 上真实执行。浏览器套件每个脚本使用隔离的临时 SQLite 数据库；耗时按整秒记录。

| 命令 | 结果 | 关键输出 | 耗时 |
| --- | --- | --- | ---: |
| `npm run typecheck` | PASS | client `tsc --noEmit`，0 error | 5s |
| `EXPO_NO_TELEMETRY=1 npm run web:build` | PASS | Metro Web bundle 导出成功（约 1 MB） | 2s |
| `npm run test:planning` | PASS | 14 tests，14 pass，0 fail | <1s |
| `npm run test:workflow` | PASS | 6 tests，6 pass，0 fail | 1s |
| `npm run test:prd-client` | PASS | 53 tests，53 pass，0 fail | <1s |
| `npm run test:prd-web`（连续第 1 次） | PASS | 21 个脚本，71 条 PASS 输出 | 197s |
| `npm run test:prd-web`（连续第 2 次） | PASS | 21 个脚本，71 条 PASS 输出 | 198s |
| `npm run test:prd-web`（连续第 3 次） | PASS | 21 个脚本，71 条 PASS 输出 | 193s |
| `npm run test:sync` | PASS | 14 tests，14 pass，0 fail | 5s |
| `cargo test --workspace` | PASS | 合计 150 passed，0 failed | 15s |
| `cargo fmt --all --check` | PASS | 无格式差异 | 1s |
| `node tools/check-prd-maintenance.mjs` | PASS | 离线 CLI、备份／恢复、模型隔离与会话撤销检查通过 | 1s |
| `python3 -m py_compile tools/smoke_prd_*.py` | PASS | 全部 smoke 脚本编译通过 | <1s |

附加检查：

- `worker-build --release`：PASS，Rust Worker/Wasm release bundle 成功生成，约 49.72s；产物为 gitignored。
- `npm run test:prd-runtimes`：native SQLite 合约和 XLSX 合约全部通过；Worker/DO 阶段在 Miniflare 启动前被宿主环境阻断，详见 `BLOCKED-NEEDS`。
- `git diff --check`、`python3 -m py_compile tools/verify-prd-export.py`：PASS。
- 提交后 `git status --short --branch` 仅显示分支 ahead 5，无未提交文件。

## 八、本切片发现的新缺陷（未修）

无 A2-1 范围内的未修代码缺陷。

完整浏览器套件曾暴露两个测试定位假设（目录数组首项、同日同课按钮顺序）；均已改为稳定 ID 定位并在之后连续三轮完整套件中验证。没有删除测试或放宽业务断言。

## 九、后续切片建议

- A2-2：按既定范围实现总结中的结构化困难，不在本提交顺带处理。
- B-1：继续处理方法创建区分。
- C-1：在具备兼容 GLIBC 的 Worker 测试宿主上补跑双运行环境全量合约。
- L-3：只有 PRD 明确 Excel 导入字段决策后，才为 domain／due／support 增加导入映射。
- 若产品后续给出统一领域目录，应以目录种子／管理流程加入，继续保持稳定 ID，不改成前端硬编码列表。

## 十、未做与原因

- 未访问、部署或修改生产；未修改 `wrangler.toml` 生产绑定或生产 `SKARMA_MODEL`。
- 未修改 legacy `crates/core/src/model.rs` 或 `migrations/001–010`。
- 未做 L-2 生产存储转换、L-3 Excel 导入映射。
- 未做 A2-2、B-1、C-1 的业务实现。
- 未预置硬编码领域集合：PRD 未穷举选项，按要求采用稳定共享目录。
- 未把长期方向的 due 解释为系统截止日，也未增加逾期压力提示。
- 未把基础帮助条件做成必填或五问流程。

## BLOCKED-NEEDS

可选的 Worker/DO 双运行环境验收受宿主运行库阻断，不是应用测试失败：

- `worker-build --release` 已成功。
- native SQLite 端完整合约、导出和重启校验已通过。
- Miniflare 启动 `workerd` 时连续两次报 `write EPIPE`；直接执行安装的 `workerd` 显示宿主缺少 `GLIBC_2.32`、`GLIBC_2.33`、`GLIBC_2.34`、`GLIBC_2.35`。
- 需要在提供兼容 GLIBC 的 runner／容器中重跑 `npm run test:prd-runtimes`；本切片没有通过修改应用代码或生产配置规避该环境限制。
