# Phase 2 切片 6 实施报告（A2-2：总结新建困难的结构化字段）

- 基线：`402cfbb`；结束提交：`b3a6f71`（`feat: structure summary-created difficulties`）；日期：2026-10-25（UTC）
- 分支：`feat/tencent-informed-web`；提交后工作树干净；仅本地 commit，未 push。
- 测试边界：所有服务端、浏览器和导出检查均使用临时／合成数据库；未访问生产账号、生产绑定或业务数据。

## 一、结构化字段（观察/成人回应/原因猜测/条件）

### 附录 B 与 PRD 依据

附录 B 原文：

> `difficulty.observation | 实际情况／必填 | 大人怎样回应，后来呢？ | 例如：换玩具后离开了 | 详例含当时任务、发生什么、成人回应与结果`

> `difficulty.hypothesis | 原因猜测／可选 | 有什么尚未确定的猜测？ | 例如：可能有些累了 | 与观察分开`

> `knowledge.links | 关联目标或方法／必填 | 这条记录适用于哪里？ | 选择目标或方法 | 多选；当前上下文优先`

R03 原文要求总结同页保存观察、成人回应、原因猜测和条件；R05/D-1 又明确困难描述必须包含成人回应，且“原因猜测，不能把猜测写成事实”。附录 B 没有为 `difficulty.adult_response` 和 `difficulty.conditions` 单列两行，因此没有臆造新的治疗措辞：这两项沿用项目在 `field-guidance.ts` 中已经批准并用于过程记录的文案：

- 成人回应：问句“当时大人怎样支持或回应？”，placeholder“例如：先回应他的手势，再示范小步骤”。
- 条件：问句“在什么场景和帮助条件下观察？”，placeholder“例如：在熟悉的餐桌旁，提醒一次”。

### 改动与各字段实现

- `apps/client/src/prd-session-view.tsx:134`：困难的观察改为独立“实际情况／必填”输入，值仍进入既有 `NewKnowledge.content`；这是 Knowledge 的事实正文。
- `apps/client/src/prd-session-view.tsx:135`：新增独立“成人回应／可选”输入，保存到 `Knowledge.details.adult_response`。
- `apps/client/src/prd-session-view.tsx:136`：新增独立“原因猜测／可选”输入，保存到 `Knowledge.details.guess`。
- `apps/client/src/prd-session-view.tsx:137`：新增独立“本次条件／可选”输入，保存到 `Knowledge.details.conditions`。
- `apps/client/src/field-guidance.ts:133`、`apps/client/src/field-guidance.ts:143`：集中登记成人回应和条件的三层引导；动态编号字段由 `guidanceFor` 映射到固定 PRD 文案。
- `crates/core/src/foundation/session_model.rs:91`：总结新增 presence-aware 的 `knowledge_details: Option<BTreeMap<knowledge_id, Details>>`，与 `new_knowledge` 的稳定 ID 一一对应。
- `crates/core/src/foundation/sessions.rs:616`：完成总结时，从该映射取成人回应、猜测和条件，写入最终 `Knowledge.details`；观察正文保持在 `Knowledge.input.content`，没有把四项拼成一段。
- `crates/core/src/foundation/sessions.rs:419`：拒绝没有对应 `new_knowledge` 的孤立 details 项，避免错绑。

### 可选性与实测

观察正文仍为创建困难的唯一必填正文；成人回应、原因猜测、条件均可留空，不阻塞总结提交。浏览器真实填写四项并从 `/api/prd/state` 回读，确认分别落入：

- `knowledge.input.content`
- `knowledge.details.adult_response`
- `knowledge.details.guess`
- `knowledge.details.conditions`

随后在已完成总结中只修改成人回应，Knowledge 版本从 1 增至 2，其他记录不重复创建。

## 二、原话／推测分离

### 数据结构分离

- 观察到的事实：`Knowledge.input.content`。
- 成人实际回应：`Knowledge.details.adult_response`。
- 家庭推测：`Knowledge.details.guess`。
- 情境／前因：`Knowledge.details.conditions`。

服务端只从总结拥有的三项结构化字段更新对应 Details 字段；不会借总结入口修改知识状态、优先级、比较口径或其他隐藏属性。原因猜测为空时保持空，不会根据观察正文自动推断。

### 展示分离

- `apps/client/src/prd-followup-view.tsx:162`：困难详情分别显示“实际情况”“成人回应”“原因猜测（待核实）”“条件”；经验详情维持原展示。
- `apps/client/src/prd-session-view.tsx:177`：干预详情也逐项展示，原因猜测明确带“待核实”标记。
- 目标／知识列表仍只展示事实正文，不把猜测拼接成事实摘要。

### 实测结果

浏览器测试保存：

- 实际情况：“等待时离开了座位”
- 成人回应：“我先暂停并陪他休息”
- 原因猜测：“可能有些累了”
- 条件：“换了新材料”

随后打开知识详情，逐项断言四个显示区域；原因猜测以“原因猜测（待核实）”显示。服务端 JSON 断言四个字段互不混写。

## 三、进入 Knowledge 与版本化

### Knowledge 保存与关联

- 新建困难继续使用既有 `NewKnowledge` 和 `Knowledge`，没有新建平行实体。
- 观察正文进入 `Knowledge.input.content`；成人回应、猜测、条件进入既有 `Knowledge.details`。
- `goal_ids`／`method_ids` 仍通过既有 bindings 校验；总结快速创建默认关联当前明确实际目标／方法。
- `source_session_id`、`source_session_version`、`source_summary` 继续记录来源总结及版本。
- `crates/core/src/foundation/sessions.rs:581` 和 `crates/core/src/foundation/followups.rs:300`：生成精简来源快照时同步移除 details 映射，避免清空 `new_knowledge` 后留下孤立结构化数据。

### expected_version／operation_id／presence 语义

- 父总结仍走 `SessionWrite.expected_version`、`operation_id`、`change_note` 和事务内 operation receipt；同一 operation 重试返回同一结果。
- 子 Knowledge 使用既有 `knowledge_versions` 做并发保护；正文或三项 Details 任一发生变化都要求匹配当前 Knowledge 版本，并生成 `prd_output_revisions` 新版本。
- Session 是聚合命令，不使用实体 `Write.provided_fields` 的逐字段 wire mask。为保持相同的“缺失不等于清空”语义，本切片把 `knowledge_details` 设计为 `Option`：
  - 整个字段缺失：视为旧客户端未提供，保存父总结时保留现有 Knowledge Details。
  - 映射中存在该知识 ID：只将提交的成人回应、猜测、条件应用到 Details；空字符串可显式清空。
- `apps/client/src/prd-session-view.tsx:57`、`apps/client/src/prd-session-view.tsx:67`：打开已完成总结时载入 Knowledge 最新 input、Details 和版本；后台出现独立修订时，前端先提示冲突，不覆盖当前填写。

### 旧记录安全

- `knowledge_details` 使用 serde default + `Option`；旧 Summary 无该字段时反序列化为 `None`。
- `apps/client/src/prd-sessions.ts:25`：本机旧草稿允许省略该字段；若提供，则校验每个 Details 对应本次知识 ID。
- Rust 测试以旧 payload 反序列化并断言 `knowledge_details.is_none()`；随后模拟旧客户端省略 details 但修改父总结，确认已有结构化字段和 Knowledge 版本均保持不变。

## 四、0–N 条原子

既有 AC-R03-04 事务路径保持不变：

1. 父 Session 先在同一事务写入；
2. `outputs()` 依次创建／修订 0–N 条 Knowledge 和 Action；
3. 任一输出失败，父记录和此前已写输出全部回滚；
4. 成功后写 operation receipt；相同 `operation_id` 重试不重复产出。

`crates/api/tests/foundation_sessions.rs:202` 新增对抗测试：

- 同一总结创建两条带不同成人回应、猜测、条件的困难；
- 在第二条 Knowledge 插入前触发合成数据库错误；
- 断言父 Session 和第一条 Knowledge 一并回滚；
- 移除触发器后用同一请求保存，再用同一 operation 重试；
- 最终恰好两条 Knowledge，均关联当前目标和来源 Session，没有第三条重复产出；
- 后续仅修订第一条成人回应时，第一条升到第 2 版，第二条仍为第 1 版。

## 五、前端与 R15 三层引导

- 四项均为独立 `GuidedField`，不是一大段自由文本。
- 每项都有常驻标签、常驻问句、约 15 字短 placeholder 和“看看怎么写（可选）”：
  - 实际情况：附录 B 的“换玩具后离开了”。
  - 成人回应：既有合成例“先回应他的手势，再示范小步骤”。
  - 原因猜测：附录 B 的“可能有些累了”。
  - 条件：既有合成例“在熟悉的餐桌旁，提醒一次”。
- `GuidedField` 的示例只在 placeholder／本地展开状态；不会调用字段 `onChange`。
- 浏览器测试先填写自有原因猜测，再展开合成示例，断言输入值未变化；服务端回读也不含示例文案。
- 成人回应、原因猜测、条件可跳过；服务端只要求困难事实正文和至少一个目标／方法关联。
- 新增／取消困难时，`new_knowledge` 和同 ID 的 `knowledge_details` 同一次本机草稿更新，避免孤立字段。
- 已完成总结重新打开时，结构化字段从 Knowledge 当前版本回填；更改后沿用原 Knowledge ID，不重放创建。

## 六、全量测试实跑结果

以下均为本轮在 operator host 上真实执行。浏览器套件每个脚本使用隔离的临时 SQLite 数据库；耗时按整秒记录。

| 命令 | 结果 | 关键输出 | 耗时 |
| --- | --- | --- | ---: |
| `npm run typecheck` | PASS | client `tsc --noEmit`，0 error | 5s |
| `EXPO_NO_TELEMETRY=1 npm run web:build` | PASS | Metro Web bundle 导出成功 | 2s |
| `npm run test:planning` | PASS | 14 tests，14 pass，0 fail | 1s |
| `npm run test:workflow` | PASS | 6 tests，6 pass，0 fail | <1s |
| `npm run test:prd-client` | PASS | 53 tests，53 pass，0 fail | <1s |
| `npm run test:prd-web`（连续第 1 次） | PASS | 21 个脚本，71 条 PASS 输出 | 194s |
| `npm run test:prd-web`（连续第 2 次） | PASS | 21 个脚本，71 条 PASS 输出 | 197s |
| `npm run test:prd-web`（连续第 3 次） | PASS | 21 个脚本，71 条 PASS 输出 | 198s |
| `npm run test:sync` | PASS | 14 tests，14 pass，0 fail | 3s |
| `cargo test --workspace` | PASS | 合计 151 passed，0 failed | 15s |
| `cargo fmt --all --check` | PASS | 无格式差异 | 1s |
| `node tools/check-prd-maintenance.mjs` | PASS | 离线 CLI、备份／恢复、模型隔离与会话撤销检查通过 | 1s |
| `python3 -m py_compile tools/smoke_prd_*.py` | PASS | 全部 smoke 脚本编译通过 | <1s |

附加检查：

- `worker-build --release`：PASS，Worker/Wasm release bundle 生成成功，50s；产物 gitignored。
- `npm run test:prd-runtimes`：native SQLite 的完整合约、XLSX 和重启前置检查均通过；Worker/DO 阶段受宿主 GLIBC 阻断，见 `BLOCKED-NEEDS`。
- `git diff --check`：PASS。
- 提交后 `git status --short --branch` 仅显示分支 ahead 6，无未提交文件。

## 七、本切片发现的新缺陷（未修）

无 A2-2 范围内的未修代码缺陷。

## 八、后续切片建议

- B-1：按既定范围处理方法快速创建与完整创建的区分。
- C-1：在具备兼容 GLIBC 的 runner 上补跑 Rust Worker／Durable Object 全量合约。
- 若后续扩展独立困难创建的同页体验，可复用本切片集中后的四项引导，不再复制硬编码文案。
- 保持 R11 困难跟进仍走既有反馈事件模型；不要把后续跟进事件反写覆盖本次初始观察。

## 九、未做与原因

- 未重复实现参与信号 chips；D-2 已由前序切片完成。
- 未改变 R11 跟进事件模型、经验阶段规则或知识状态机。
- 未实现 B-1、C-1。
- 未修改 Excel 导入映射、生产存储转换、legacy 模型或 `migrations/001–010`。
- 未访问或部署生产，未修改 `wrangler.toml` 或生产 `SKARMA_MODEL`。
- 未把原因猜测从正文自动抽取，也未将猜测用于统计或因果结论。
- 未让三个可选字段阻塞提交。
- 未新建并行“困难”实体；继续使用现有 Knowledge／Details 与输出修订机制。

## BLOCKED-NEEDS

可选的 Worker/DO 双运行环境验收受宿主运行库阻断，不是应用测试失败：

- `worker-build --release` 已成功。
- native SQLite 端完整合约、XLSX 和持久化前置校验已通过。
- Miniflare 启动 `workerd` 时返回 `write EPIPE`；直接运行安装的 `workerd` 明确显示宿主缺少 `GLIBC_2.32`、`GLIBC_2.33`、`GLIBC_2.34`、`GLIBC_2.35`。
- 需要在提供兼容 GLIBC 的 runner／容器中重跑 `npm run test:prd-runtimes`；本切片未通过修改应用代码或生产配置规避该环境限制。
