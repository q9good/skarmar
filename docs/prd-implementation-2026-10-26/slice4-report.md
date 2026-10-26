# Phase 2 切片 4 实施报告（A1-2：课记撤销入口与入站引用预检）

- 基线：起始提交 f1ecb3517c908566b4d9dfff21c58b8d8629fcd0
- 结束提交：c8a581e82003f02309e7f638865d6eba592c771e
- 分支：feat/tencent-informed-web
- 日期：2026-10-25
- 提交：fix: gate lesson withdrawal by capability
- 边界：未 push、未部署、未访问生产、未修改生产配置／迁移／导入映射，未扩展到 A2／B／C

## 一、服务端 capability 暴露

### 数据契约

crates/core/src/foundation/lesson_model.rs:178-185 新增 WithdrawalCapability：

- allowed：当前登录成员现在是否可撤销；
- reason_code：allowed／not_recorded／not_creator／outside_creation_day／referenced；
- reason：可直接展示给家人的说明；
- lesson_version：能力判定所依据的课记版本；
- references：阻止撤销的引用类型。

### 读取接口

- crates/core/src/lib.rs:53-55、255-257 新增已认证命令 FoundationLessonWithdrawalCapability。
- crates/http/src/lib.rs:506-519 新增 handler。
- crates/http/src/lib.rs:672-675 新增 GET /api/prd/lessons/{id}/withdrawal-capability。
- crates/api/src/store.rs:176-184 提供原生 SQLite 测试／适配器入口。
- 路由仍经过既有 require_session；capability 由服务端当前 actor 计算，客户端不能传 creator 身份。

### capability 判定逻辑

crates/core/src/foundation/lessons.rs:154-214 的 capability_for 按以下顺序判定：

1. 课记仍是 recorded；
2. actor 是 creator_id；
3. created_at 转为固定 +08:00 家庭业务日，必须等于服务器当前时间所属家庭业务日；
4. 无入站引用；
5. 未被认领整理；
6. 无家人“看到了”回应。

通过才返回 allowed=true。

同日判断使用服务器 created_at 与服务器 Utc::now()，都转为家庭业务时区；不使用补录上课日期，也不信任浏览器本地日期。

### 与事务最终权威一致

- crates/core/src/foundation/lessons.rs:578-592 的 LessonAction::Withdraw 在同一个数据库事务内再次调用 capability_for。
- capability 预检和最终写入不是两套规则。
- 非创建者／跨日维持 403；已引用／已撤销等维持 409。
- 即使客户端刚读到 allowed=true，只要提交前出现新引用，事务重算后仍拒绝。
- 客户端返回的 lesson_version 还会与当前详情版本核对；版本不一致时不展示撤销入口。

结果：服务端 capability 是 UX 预检，事务仍是最终权威。

## 二、客户端按 capability 渲染

### 改动

- apps/client/src/prd-lessons.ts 增加 WithdrawalCapability 类型。
- apps/client/src/prd-lesson-view.tsx:137 增加 capability／loading／error 状态。
- apps/client/src/prd-lesson-view.tsx:216-226 在打开课记详情或课记版本变化时读取 capability。
- 若返回的 lesson_version 与当前详情版本不一致，按不可撤销处理并提示刷新核对。
- 请求失败时采取 fail-safe：不显示撤销，只提供标记记错路径。

### 渲染规则

apps/client/src/prd-lesson-view.tsx:302：

- withdrawalLoading：只显示“正在核对”，不提前显示任何撤销按钮。
- capability.allowed=true：
  - 显示服务端原因；
  - 只显示“撤销这条课记”；
  - 不显示“标记记错”。
- capability.allowed=false 或 capability 暂不可读：
  - 不显示撤销；
  - 显示原因；
  - 未标记时显示“标记记错，请管理员处理”。
- 已标记时显示醒目状态，不重复显示必然无意义的标记按钮。

因此不可撤销课记不再出现“点了才失败”的撤销入口，AC-R04-05 的客户端缺口已关闭。

## 三、入站引用预检

### 哪些算入站引用

crates/core/src/foundation/lessons.rs:119-152 汇总入站引用：

1. prd_lesson_refs 现有统一引用账本：
   - knowledge：由课记整理出的经验／困难；
   - action：由课记产生的下一步；
   - feedback：以课记为来源的经验反馈／困难跟进；
   - plan：家庭计划通过 lesson_sources 引用课记；
   - 未来写入账本的其他 source_kind 也自动参与。
2. 复盘：
   - prd_reviews；
   - prd_review_revisions；
   - evidence 中 source.kind=lesson 且 ID 匹配。
3. 目标测量／其他观察派生：
   - prd_observations；
   - prd_observation_revisions；
   - input.source.kind=lesson 且 ID 匹配。
4. 整理认领：Lesson.organization.claimed_by。
5. 家人回应：Lesson.acknowledgements。

references 排序去重后同时用于 capability reason 和事务判定。

当前模型没有独立的“课记关联另一课记”实体；家庭计划引用、知识／跟进／行动、复盘与测量均已覆盖。如果未来增加课记到课记关系，应写入 prd_lesson_refs，从而自动进入本判定。

### 与管理员删除的关系

- 管理员删除入口保持原能力。
- crates/core/src/foundation/lessons.rs:594-601 仍要求管理员权限，并用同一 inbound() 引用集合保护知识／行动／反馈／计划／复盘／测量来源。
- 本切片没有把创建者撤销例外扩展成普通成员删除。
- 整理认领／轻量回应仍阻止创建者撤销；管理员删除继续受已有来源保护边界控制。

### 并发新增引用

Rust 测试 crates/api/tests/foundation_lessons.rs:285-396：

1. capability 先返回 allowed=true；
2. 另一事务保存引用该课记的共享计划；
3. 课记版本不变，排除“仅版本冲突”假象；
4. 使用原课记版本提交撤销；
5. 最终事务返回 409，课记保持 recorded；
6. 再读 capability 返回 referenced，references=[plan]。

浏览器测试 tools/smoke_prd_lessons_web.py:104-112 同样通过真实 HTTP／UI 复现：

- 详情先显示撤销；
- 后台新增 plan lesson_source；
- 用户随后提交撤销；
- 服务端返回“课记已被整理、回应或其他记录引用”；
- 重新打开详情只显示标记记错，不再显示撤销。

结果：PASS。

## 四、标记记错路径

### 客户端操作

- 不可撤销时只显示“标记记错，请管理员处理”。
- 标记操作仍使用既有 MarkError 命令，不改变课记 status，不把该节自动剔除。
- 已标记后详情显示：
  - “已标记记错，等待管理员处理；本节仍计入课时”。

### 课时明细醒目显示

apps/client/src/prd-lesson-view.tsx:315：

- 课记卡新增独立 alert 文本；
- 深色错误文字＋粗体；
- 明确写“标记记错，等待管理员处理 · 本节仍计入课时”。

既有课时核对继续显示：

- 老师合计中的“待核对 N”；
- 标记课记仍计入节数；
- 点击可进入原课记详情。

浏览器测试 tools/smoke_prd_lessons_web.py:92-101 验证：

- 标记引用课记；
- 课堂记录卡显示醒目待处理文案；
- 课时核对显示“2 节 · 待核对 1”；
- 普通成员没有管理员删除，也没有撤销入口。

### 管理员删除仍可用

- 课记详情对 admin 继续显示“删除课记（管理员）”。
- 无引用的旧课记可由管理员删除：既有 Rust 测试 creation_day_not_business_day_controls_withdrawal_and_admin_can_delete_old_unreferenced_record 继续 PASS。
- 有引用时管理员删除仍被服务端引用保护拒绝；浏览器既有断言继续 PASS。
- 管理员仍可取消待核对标记。

## 五、AC-R04-05 验收

### 场景 1：创建者同日、无引用，可撤销

服务端：

- crates/api/tests/foundation_lessons.rs:285-313。
- capability.allowed=true、reason_code=allowed、references 为空。
- 同一课记随后撤销成功，status=withdrawn。

客户端：

- tools/smoke_prd_lessons_web.py:50-52。
- 创建者详情只显示撤销；
- 标记记错入口不存在。
- 后续无引用第二节从 UI 撤销成功。

结果：PASS。

### 场景 2：任一引用后 capability=false，UI 只给记错

服务端覆盖：

- 经验／困难和下一步：foundation_lessons.rs 测试行 268-282，references 含 knowledge、action。
- 家庭计划：foundation_lessons.rs:315-363，references=[plan]。
- 复盘与测量：foundation_lessons.rs:365-395，references 含 review、measurement。
- 非创建者：tools/prd-lesson-contract.mjs 真实 HTTP capability 返回 not_creator。
- 跨创建日：现有 creation_day 测试追加 capability=false、reason_code=outside_creation_day。

客户端覆盖：

- tools/smoke_prd_lessons_web.py:51 创建者无引用显示撤销。
- 行 54 非创建者不显示撤销，只显示记错。
- 行 73 引用后的创建者不显示撤销，只显示记错。
- 行 94 标记后课堂记录与核对视图醒目显示。

结果：PASS。

### 场景 3：并发新增引用，事务仍阻止

- Rust 直接事务测试：PASS。
- 真实 HTTP＋浏览器 UI 测试：PASS。
- 撤销前预检 allowed 不会成为授权凭据；最终写操作重新计算。
- 课记最终仍为 recorded。

结果：PASS。

### 共享 HTTP 合约

tools/prd-lesson-contract.mjs：

- 创建者 capability allowed；
- 管理员／非创建者 capability not_creator；
- 整理生成知识＋下一步后 capability referenced；
- references 同时包含 knowledge、action；
- 原撤销／删除权限与错误码断言全部保留。

结果：native SQLite 浏览器／HTTP 流程 PASS；该合约也已纳入现有双运行环境契约入口，待 C-1 Worker 构建环境恢复后可同样对账。

## 六、全量测试实跑结果

全部为本轮真实运行。

| 命令 | 结果 | 关键输出 | 耗时 |
| --- | --- | --- | ---: |
| npm run typecheck | PASS | tsc --noEmit，exit 0 | 5s |
| EXPO_NO_TELEMETRY=1 npm run web:build | PASS | Metro 302 modules；Exported: dist | 3s |
| npm run test:planning | PASS | 14 tests，14 pass，0 fail | <1s |
| npm run test:workflow | PASS | 6 tests，6 pass，0 fail | <1s |
| npm run test:prd-client | PASS | 53 tests，53 pass，0 fail | 1s |
| npm run test:prd-web 第 1 次 | PASS | 71 个 PASS 检查点 | 195s |
| npm run test:prd-web 第 2 次 | PASS | 71 个 PASS 检查点 | 196s |
| npm run test:prd-web 第 3 次 | PASS | 71 个 PASS 检查点 | 195s |
| npm run test:prd-web 连续汇总 | PASS 3/3 | 无偶发失败；总计 586s | 586s |
| npm run test:sync | PASS | 14 tests，14 pass，0 fail | 4s |
| cargo test --workspace | PASS | 149 tests，0 fail | 16s |
| node tools/check-prd-maintenance.mjs | PASS | 离线导入、备份、错误模型拒绝、恢复与会话撤销 | 1s |

补充定向实跑：

| 命令 | 结果 | 关键输出 |
| --- | --- | --- |
| cargo test -p skarma-api --test foundation_lessons | PASS | 9/9，含 capability／同日／引用／并发 |
| smoke_prd_lessons_web.py | PASS | 8 个课程检查点，含 capability UI、待核对和并发引用 |
| cargo fmt --all --check | PASS | Rust 格式一致 |
| python3 -m py_compile tools/smoke_prd_lessons_web.py | PASS | Python 语法通过 |

## 七、本切片发现的新缺陷（未修）

没有发现新的范围内业务缺陷。

注意项：

- capability 读取复盘／测量历史时会扫描相应当前表与修订表。家庭规模下数据量有限且只在打开课记详情时执行，不进入全局 state 轮询。若未来数据显著增长，可在后续 schema 版本把所有 review／measurement 来源也规范化写入 prd_lesson_refs，以索引替代扫描；本切片不为性能预优化增加迁移。
- capability reason_code 是新 API 字段；客户端仍采取 fail-safe，未知／读取失败时绝不显示撤销。

## 八、后续切片建议

1. A2-1：目标 domain／due／基础帮助条件。
2. A2-2：总结新困难结构化保存成人回应、原因猜测与条件。
3. B-1：区分普通方法完整创建与计划／总结快捷创建。
4. C-1：安装 worker-build／Wasm target，恢复真实 Rust Worker／DO 对账，并验证新增 capability 合约。
5. 后续 schema 性能切片：如家庭历史规模需要，将 review／measurement lesson source 统一写入带索引的引用账本。

## 九、未做与原因

- 未改变物理删除权限：仍仅管理员。
- 未把标记记错自动剔除课时：PRD 明确要求待核对仍计并单列。
- 未增加 lesson capability 到全量 /prd/state：它依赖当前 actor 和服务器当前业务日，放入共享快照会产生过期／串号风险；按需详情读取更符合现有架构。
- 未修改 A2／B／C 业务能力。
- 未做真机手势验收：本轮为桌面 Chromium 手机视口模拟，不冒充真机。
- 未访问生产、未部署、未 push。

## BLOCKED-NEEDS

无。本切片 capability、客户端入口、入站引用保护、并发事务验收、三次完整浏览器套件和本地提交均已完成。
