# Phase 2 切片 7 实施报告（B-1：普通方法创建与快捷方法创建区分）

- 基线：`b3a6f71`；结束提交：`7117f32`（`feat: distinguish full and quick method creation`）；日期：2026-10-25（UTC）
- 分支：`feat/tencent-informed-web`；提交后工作树干净；仅本地 commit，未 push。
- 测试边界：所有服务端、浏览器和导出检查均使用临时／合成数据库；未访问生产账号、生产绑定或业务数据。

## 一、普通方法创建（要求 steps）

### 附录 B 依据

附录 B 原文：

> `method.steps | 具体做法／可选 | 大人具体怎样帮助？ | 例如：先跟着他的玩法 | 专业来源不作先决条件`

> `method.observe | 观察意图／可选 | 想改善什么，怎样观察？ | 例如：愿意再试一次 | 可引用R06模板`

> `method.scope | 适用条件／可选 | 什么时候适合使用？ | 例如：熟悉的游戏中 | 留意任务和支持条件`

R07 同时规定“方法至少有名称、做法和明确目标关联”；附录 B 的 steps 在通用字段层面可渐进补充，但本切片裁决要求方法页的“普通完整新建”必须填写它，计划／总结快捷创建是唯一例外。

### 改动

- `crates/core/src/foundation/model.rs:82`：`MethodInput.steps` 作为新普通方法的规范“具体做法”字段；新增可选 `observe`（`model.rs:86`）和 `scope`（`model.rs:88`），旧记录缺字段时 serde 默认空字符串。
- `crates/core/src/foundation/mod.rs:348`：服务端收到 `method_create_mode=full` 且 `steps.trim()` 为空时，返回 422“普通新建方法须填写具体做法”。
- `apps/client/src/foundation-view.tsx:124`：方法页完整表单使用“具体做法／必填”，保存到 `steps`；`observe`、`scope` 为独立可选输入。
- `apps/client/src/foundation-view.tsx:240`、`foundation-view.tsx:363`：普通新建时 steps 为空会显示清晰提示并禁用保存；提示同时说明计划／总结快捷新增可只填名称。
- `apps/client/src/foundation-view.tsx:233`、`foundation-view.tsx:249`：新建方法页草稿和提交请求显式携带 `method_create_mode=full`；旧的未提交完整草稿在真正发送前补上 full，已形成 pending 的原请求则保持字节不变。
- `apps/client/src/field-guidance.ts:335`：具体做法、观察意图、适用条件映射到附录 B 的固定问句和示例。
- `apps/client/src/foundation-view.tsx:116`：保留旧 foundation 记录的兼容显示。历史 `description` 仍可读；一旦用户在新表单修改具体做法，就写入规范 `steps`，不丢失旧内容。

### 实测

- Rust 对抗测试通过公共 JSON wire 创建普通方法：空 steps 被 422 拒绝；补上 steps 后保存成功，并精确保存 observe／scope。
- 浏览器方法页在空 steps 时显示提示且保存按钮禁用；填写“具体做法／必填”后请求被捕获并断言 `method_create_mode=full`。
- 保存后服务端回读 steps、observe、scope；再次修改具体做法后方法版本由 1 升到 2，修订历史仍可见。

## 二、快捷方法创建（name + 目标关联）

### 改动及与既有路径的关系

R02／R03 的计划／总结快捷方法创建在前序实现中已经存在，本切片没有重建 UI 或增加第二套快捷流程，只核对并补齐服务端模式：

- `apps/client/src/prd-session-view.tsx:170`：既有快捷覆盖层仍只有“新方法名称”输入；方法请求显式携带 `method_create_mode=quick`。
- 计划阶段使用当前计划已明确选择的目标 ID；总结阶段使用本次标记为实际发生的目标 ID。最终仍由服务端既有 bindings 校验目标存在、不可重复且数量为 1–100。
- `newMethod()` 的 steps／observe／scope 默认空；quick 模式允许空 steps。
- 快捷创建成功后，方法按既有逻辑立即选入当前计划或实际方法，保存 Session 时仍引用该方法的明确版本。
- 快捷名称输入映射到附录 B 的 `method.title` 文案：“怎样称呼这个做法？”／“例如：示范后少提醒”；没有错误复用目录项文案。

### 实测

- 计划快捷创建和总结快捷创建均由真实浏览器执行，请求捕获断言 `method_create_mode=quick`。
- 服务端回读确认 steps 为空仍创建成功，并继承当前上下文目标关联。
- 计划快捷创建的响应丢失／页面重载继续使用同一 operation，最终只有一个方法和一条方法修订。
- 总结快捷创建的新方法被计为本次实际方法，Session 中保存其第 1 版快照；不会因为 full 路径的 steps 要求失败。

## 三、两条路径的服务端区分

### 显式创建模式

- `crates/core/src/foundation/model.rs:157`：新增 `MethodCreateMode::{Full, Quick}`。
- `Write.method_create_mode` 位于 mutation 外层，与 `method_change`（已有方法调整／更正文案）分开；不会污染持久化的 MethodInput。
- 公共 JSON 只接受字符串 `full` 或 `quick`；其他值在反序列化时拒绝。
- 外部新建方法未提供模式时，服务端拒绝，不会默认放宽为 quick。
- 创建模式用于非方法对象或用于已有方法修订时，服务端拒绝。
- quick 只放宽 steps；标题和明确目标关联仍使用同一服务端校验。
- fully specified 的 Rust 内部转换／导入命令以 `provided_fields=None` 标识为可信核心调用，不开放为公共 JSON 模式，也没有修改 L-2／L-3 的字段映射。

### 幂等与旧 operation

创建模式进入 `Write` 的标准序列化和 `legacy_json`，因此属于 operation identity：同一 operation 改 full／quick 会冲突。相同 quick 请求重试返回原 receipt，不重复创建。

旧 operation 没有该字段时，`None` 仍按旧字节序列化；operation receipt 查询发生在新建校验之前，因此部署前已经提交成功的旧请求仍可安全重放，不会被新模式校验破坏。

### 对抗实测

`crates/api/tests/foundation.rs:430` 覆盖：

- 无模式的外部方法创建被拒绝；
- full + 空 steps 被拒绝；
- full + steps／observe／scope 成功；
- quick + 名称／目标成功且 steps 为空；
- 同一 quick operation 重试仍只有一条实体／修订；
- 创建模式不能用于已有方法更新；
- 伪造 `import` 模式在反序列化阶段失败。

## 四、方法版本与排序不回归

### 版本和实践快照

两种创建路径最终都调用同一个 foundation `write()`：

- 写入 `prd_entities` 第 1 版；
- 写入 `prd_revisions`；
- 关联写入 `prd_method_goals`；
- 沿用 operation receipt 幂等和 expected_version 冲突保护。

方法编辑仍生成新版本。Session 的计划／实际方法继续保存完整 Method 快照和版本；既有 AC-R07-02 测试证明方法改成第 2 版后，旧实践仍引用第 1 版。

为统一新规范字段：

- `crates/core/src/foundation/reviews.rs:699`：复盘中“调整做法”写入规范 steps，并保留方法版本事件。
- `crates/core/src/foundation/mod.rs:400`：具体做法或 scope 变化继续产生 `practice_adjusted` 分段。
- `crates/core/src/foundation/reviews.rs:861`、`crates/core/src/foundation/export.rs:105`：实践依据和导出优先读取规范 steps，同时兼容旧 description。
- `crates/http/src/prd_export.rs:73`：工作簿列名明确为“具体做法”“观察意图”“适用条件”。

### 排序与心得不回归

- `npm run test:prd-client` 的五级排序测试继续覆盖：手动置顶／困难／超期／普通 active／仅关联已达成目标；已达成目标关联方法仍排在普通 active 后，不因创建模式变化调整 rank。
- 方法使用统计仍只看实际 Session／Lesson 快照，排除计划、删除和未来记录。
- 浏览器“记一条心得”仍明确关联方法；“据此调整方法”保留精确 Knowledge 版本作为 basis snapshot，并生成方法新版本。
- 完整浏览器三轮和 Rust workspace 均通过 AC-R07-02／03 相关既有测试。

## 五、前端与 R15 三层引导

### 普通完整表单

- 方法名称：问句“怎样称呼这个做法？”；placeholder“例如：示范后少提醒”。
- 具体做法／必填：问句“大人具体怎样帮助？”；placeholder“例如：先跟着他的玩法”。
- 观察意图／可选：问句“想改善什么，怎样观察？”；placeholder“例如：愿意再试一次”。
- 适用条件／可选：问句“什么时候适合使用？”；placeholder“例如：熟悉的游戏中”。
- 每个文本字段由 `GuidedField` 提供常驻标签、常驻问句、短 placeholder 和可展开合成示例。

### 快捷表单

- 只显示“新方法名称”，不显示也不校验 steps／observe／scope。
- 名称使用同一 `method.title` 三层引导，不使用普通表单之外的自创文案。
- 页面文案明确区分：“普通新建方法需写名称、具体做法并关联目标；计划或总结里的快捷新增可以先只写名称。”

### 示例安全

示例只在 placeholder 或本地展开状态中；不调用 `change`。浏览器测试在已填写具体做法后展开示例，断言输入值不变；服务端回读、Session、统计和 Excel 均不含示例文本。

## 六、全量测试实跑结果

以下为最终代码状态在 operator host 上真实执行。浏览器脚本各自使用隔离的临时 SQLite 数据库；耗时按整秒记录。

| 命令 | 结果 | 关键输出 | 耗时 |
| --- | --- | --- | ---: |
| `npm run typecheck` | PASS | client `tsc --noEmit`，0 error | 5s |
| `EXPO_NO_TELEMETRY=1 npm run web:build` | PASS | Metro Web bundle 导出成功 | 4s |
| `npm run test:planning` | PASS | 14 tests，14 pass，0 fail | <1s |
| `npm run test:workflow` | PASS | 6 tests，6 pass，0 fail | 1s |
| `npm run test:prd-client` | PASS | 53 tests，53 pass，0 fail | <1s |
| `npm run test:prd-web`（连续第 1 次） | PASS | 21 个脚本，71 条 PASS 输出 | 194s |
| `npm run test:prd-web`（连续第 2 次） | PASS | 21 个脚本，71 条 PASS 输出 | 196s |
| `npm run test:prd-web`（连续第 3 次） | PASS | 21 个脚本，71 条 PASS 输出 | 197s |
| `npm run test:sync` | PASS | 14 tests，14 pass，0 fail | 3s |
| `cargo test --workspace` | PASS | 合计 152 passed，0 failed | 12s |
| `cargo fmt --all --check` | PASS | 无格式差异 | <1s |
| `node tools/check-prd-maintenance.mjs` | PASS | 离线 CLI、备份／恢复、模型隔离与会话撤销检查通过 | 1s |
| `python3 -m py_compile tools/smoke_prd_*.py` | PASS | 全部 smoke 脚本编译通过 | <1s |

附加检查：

- `npm run test:prd-runtimes`：native SQLite 的完整合约、XLSX 和重启前置检查全部通过；Worker/DO 阶段到达已知 GLIBC 环境限制，见 `BLOCKED-NEEDS`。
- `git diff --check`：PASS。
- 提交后 `git status --short --branch` 仅显示分支 ahead 7，无未提交文件。

## 七、本切片发现的新缺陷（未修）

无 B-1 范围内的未修代码缺陷。

实施中发现若干合成测试 fixture 通过公共 wire 创建名称-only 方法却未声明来源；这些 fixture 已按其真实语义显式标记为 quick，没有放宽服务端校验或删除断言。

## 八、后续切片建议

- C-1：在具备 glibc ≥2.32 的 runner 上执行 Rust Worker／Durable Object 全量合约。
- 后续如新增其他上下文快捷入口，必须复用 `method_create_mode=quick` 并由服务端验证目标关联，不得另设无标记旁路。
- 待旧 foundation 方法逐步被家人编辑时，可自然写入规范 steps／observe／scope；不要无依据批量改写历史 description／steps 语义。

## 九、未做与原因

- 未重建 R02／R03 快捷方法 UI；前序路径已存在，本切片只补显式模式和服务端区分。
- 未修改方法五级排序规则、rank 数值或心得模型。
- 未修改 Session 方法快照结构或历史实践版本。
- 未修改 Excel 导入映射、生产存储转换、legacy 模型或 `migrations/001–010`。
- 未访问或部署生产，未修改 `wrangler.toml`、package-lock、workerd 版本或生产 `SKARMA_MODEL`。
- 未把 observe／scope 变成必填。
- 未为跑通 DO 降级依赖或规避宿主运行库限制。

## BLOCKED-NEEDS

可选的 Worker/DO 双运行环境验收受已知宿主限制阻断，不是应用测试失败：

- native SQLite 端完整合约、XLSX 和持久化前置校验已通过。
- Miniflare 启动 `workerd` 时返回 `write EPIPE`；本机 Ubuntu 20.04 / glibc 2.31，不满足 workerd 所需 glibc ≥2.32。
- 按本切片硬约束未降级 workerd、未改 package-lock；需在兼容 runner 中执行 C-1。
