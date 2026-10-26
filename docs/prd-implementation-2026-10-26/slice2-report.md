# Phase 2 切片 2 实施报告（A0 可靠性 + A0-2 + 测试基础设施）

- 基线：起始提交 4d65479cbae5b22178900b9ed54e641a339081b9
- 结束提交：ec0381044bff43ca6701cb841ee7e2d576754762
- 分支：feat/tencent-informed-web
- 日期：2026-10-25
- 提交：fix: harden A0 offline and overlay reliability
- 边界：未 push、未部署、未修改 wrangler.toml、未设置生产 SKARMA_MODEL、未访问生产账号或业务数据、未实施 L-2／L-3

## 一、覆盖层 Back 竞态修复

### 问题

原实现先渲染可交互 Modal，再在普通 useEffect 中异步执行 prune().then(pushState)。首次打开或清理陈旧 sentinel 时，对话框可能已经可见，但 history entry 尚未登记；此时立即浏览器 Back 会离开应用。切片 1 与编排者均复现了 smoke_prd_observations_web.py 间歇跳到 about:blank。

### 改动

- apps/client/src/overlay.tsx:2 引入 useLayoutEffect。
- apps/client/src/overlay.tsx:44 增加 historyReady；原生平台默认 ready，Web 必须等 history sentinel 完成。
- apps/client/src/overlay.tsx:52-74：
  - 在 layout effect 中登记 layer；
  - 没有陈旧 sentinel／draining 时同步 pushState，并在浏览器绘制前设为 ready；
  - 需要清理陈旧 sentinel 时继续使用现有 drain 串行化，但在完成前保持 Modal 不可见、不可交互；
  - 组件提前卸载时 cancelled 阻止迟到的 pushState。
- apps/client/src/overlay.tsx:75 只有 historyReady=true 才显示 Modal。

该修复消除了“可见对话框没有对应 history entry”的状态，而不是用等待、重试或加延时掩盖。

### 新增的确定性竞态测试

文件：tools/smoke_prd_overlay_web.py。

如何确定性复现：

- 行 34-41 人工放入一个陈旧 sentinel，并临时截住 history.back，使 prune drain 必然保持 pending；不使用 sleep，也不依赖概率。
- 行 42-45 点击打开课记覆盖层，断言 Back 清理已请求，但对话框仍不可见且 URL 未离开应用。
- 行 46-48 放行真实 Back；清理结束后才显示对话框，并断言当前 history marker 有效。

该测试在旧实现下会看到已可交互对话框；新实现必须保持隐藏，因此锁定的是竞态根因。

### AC-R14-04 与 R14 导航要求逐条核对

| 要求 | 测试证据 | 结果 |
| --- | --- | --- |
| Back 只关闭顶部覆盖层 | smoke_prd_overlay_web.py:50-59，课记层＋课程选择层逐层 Back | PASS |
| 多层覆盖层逐层关闭 | 子层关闭后父课记仍可见且 marker 已变化，行 51-58 | PASS |
| 顶层才回到应用页面，不退出 | 每次关闭后 URL 等于原应用 URL，行 45、57、59、65、87 | PASS |
| Forward 不复活孤儿 sentinel | 行 61-65，Forward 后自动清理 orphan，无遮罩 | PASS |
| 刷新重载 | 行 67-76，在 sentinel entry reload 后 UI 不伪装恢复；再次打开先修复 orphan | PASS |
| 多次开关不积累 sentinel | 行 78-87，连续 8 次打开／Back，history.length 保持不变 | PASS |
| 竞态不是靠 sleep／retry | 测试通过截住／放行真实 history.back 构造 pending 状态 | PASS |

稳定性实测：

- smoke_prd_observations_web.py 单独连续 5 次：5/5 PASS，39 秒。
- npm run test:prd-web 完整套件连续 5 次：5/5 PASS。
- 每次 69 个 PASS 检查点；耗时分别 188、189、189、189、189 秒，总计 944 秒。
- 五轮均包含新增确定性 overlay smoke 及原 smoke_prd_observations_web.py。

## 二、A0-2(a) 离线队列取消

### 改动

#### 状态机

apps/client/src/prd-outbox.ts:87-106 新增 LessonOutbox.cancel：

- 仅 entry.status=queued 且 attempted===false 可取消。
- uncertain、blocked、confirmed、linked 及不存在的 entry 一律拒绝。
- 所有读取／校验／删除／保存都在既有队列 lock 内，和 flush／其他窗口串行。
- 保存前再次 decodeOutbox 校验，存储失败不会把内存状态伪装成已取消。

#### 目录依赖级联

- 取消 catalog 时，若任一待上传课记仍引用它，直接拒绝；用户必须取消相应课记。
- 取消 lesson 时读取其 course_id／teacher_id：
  - 仅删除同队列中 queued 且 attempted=false 的 catalog；
  - 仅当该 catalog 已不被任何其他待上传课记引用时删除；
  - 被另一课记共享的目录项保留；
  - uncertain／blocked／confirmed／linked 依赖不删除。

#### UI

apps/client/src/prd-outbox-view.tsx:62-80：

- 只有真正可取消的 entry 显示“取消待上传”入口。
- 被课记引用的新增目录显示依赖说明，不单独提供取消。
- uncertain 明示“服务器响应未知，必须核对同一次操作，不能取消”。
- 取消前打开“取消待上传内容”确认覆盖层；说明只删除本机队列，不撤销／删除服务器记录。
- 成功后显示清理了多少孤儿新增选项；失败保留原数据。

### uncertain 不可丢弃如何保证

三层保证：

1. LessonOutbox.cancel 的强校验要求 status=queued 且 attempted===false，服务层调用也无法越过。
2. UI 不给 uncertain／blocked 显示取消按钮。
3. 单元测试比较取消失败前后的原始队列字符串，确认失败不写盘。

证据：

- tools/test-prd-client.mjs:187-192 验证：
  - 正被课记引用的 catalog 不能单独取消；
  - uncertain 取消拒绝且 raw 完全不变；
  - actor／space 不匹配后进入 blocked 的队列也不能取消。
- tools/smoke_prd_offline_web.py:52-59 在真实浏览器丢响应形成 uncertain，断言无取消入口，再用原 operation 核对回执。

### 目录依赖级联如何处理

- tools/test-prd-client.mjs:178-186：
  - 两条离线课记共享同一课程／老师；
  - 取消第一条只删除课记，两个共享目录仍保留；
  - 取消第二条时删除课记和两个最终孤儿目录；
  - 全程没有发送网络写入。
- tools/smoke_prd_offline_web.py:35-39：
  - 离线新建课程、老师和课记；
  - 显式确认取消课记；
  - 队列变空；
  - 清理 2 个孤儿目录；
  - 服务端 lessons／catalog 均仍为空。

### 跨账号阻断

- 队列存储 key 继续由 space＋user 组成：apps/client/src/prd-outbox-view.tsx:26-36。
- 取消只操作当前 Provider 绑定的 scope key。
- 其他账号重新加载看不到原账号队列，也没有取消入口：tools/smoke_prd_offline_web.py:68-72。
- 原有 flush 在 actor／space 不符时转 blocked 的保护保持不变。

### 实测结果

- npm run test:prd-client：52/52 PASS；相较切片 1 新增 3 项（14 天边界、取消级联、不可丢弃）。
- smoke_prd_offline_web.py：PASS，包含取消、孤儿目录、uncertain、跨账号、双窗口与重放。
- 五轮完整浏览器套件全部 PASS。

## 三、A0-2(b) 可信会话 14 天续期提示

### 架构选择

提示策略放在客户端：

- 服务端已权威保存 365 天 trusted session TTL，并通过 /auth/me 返回 expires_at 和 trusted_device。
- “到期前 14 天显示站内非阻断提示”是展示策略，不需要改变 Cookie／会话存储或认证事务。
- 保持服务端 365 天 TTL 和撤销语义不变，避免把 UI 提示期混入安全会话生命周期。

### 改动

- apps/client/src/session-renewal.ts:2-7：
  - TRUSTED_RENEWAL_NOTICE_SECONDS=14天；
  - 普通会话保持既有 1 天提示窗口；
  - sessionExpiresSoon 集中计算。
- apps/client/src/auth.tsx:65-70：
  - 统一复用 sessionExpiresSoon；
  - 顶部站内横幅仍为普通 View，不自动打开模态框；
  - SessionRenewalContext 与页内横幅使用同一结果。
- 原有 RenewSession 覆盖层只在用户主动点击后打开；重新登录会核对原 user／space，再关闭续期层，不卸载下面表单。

### 边界与原表单实测

- tools/test-prd-client.mjs:4-11：
  - trusted 剩余 14 天＋1秒：不提示；
  - trusted 恰好 14 天：提示；
  - 普通会话 1 天＋1秒／恰好 1 天边界保持原行为。
- tools/smoke_prd_auth_sessions_web.py:19-30：
  - 建立真实未提交目标草稿；
  - 模拟剩余 14 天＋60秒：站内续期入口不存在；
  - 模拟剩余 14 天−60秒：站内非阻断入口可见；
  - 从当前目标表单主动打开续期层并重新认证；
  - 续期后原表单仍挂载，输入“续期中的原表单”原样保留。
- 服务端 trusted Cookie 仍验证约 365 天、HttpOnly、SameSite=Strict；相关 Rust／浏览器测试继续通过。

结果：PASS。

## 四、测试基础设施

### Python 测试依赖声明

新增 tools/browser-test-requirements.txt：

- playwright==1.48.0
- openpyxl==3.1.5

该文件明确是浏览器／工作簿测试依赖，不是应用运行依赖。

docs/development.md:82-92 新增安装说明：

1. python3 -m pip install -r tools/browser-test-requirements.txt
2. python3 -m playwright install chromium
3. npm run test:prd-web

并明确：

- Playwright 默认使用其管理的 Chromium；
- CHROMIUM_EXECUTABLE 只用于覆盖；
- 不与 tools/tencent-requirements.txt 混用。

### Python 3 入口统一

使用与 check-prd-web.mjs 相同的 PYTHON3／python3 规则：

- tools/prd-transfer-contract.mjs:5-10
- tools/prd-export-contract.mjs:4-13
- tools/check-prd-maintenance.mjs:6-9

仓库 Node 工具中已没有 execFileSync('python')／spawn('python') 残留。

实测：

- transfer／export 合约在 test:prd-runtimes 的 native SQLite 阶段真实通过。
- node tools/check-prd-maintenance.mjs：PASS。
- python3 -m py_compile tools/smoke_prd_*.py：PASS。

## 五、全量测试实跑结果

以下全部是本轮真实运行，不沿用历史记录。

| 命令 | 结果 | 关键输出 | 耗时 |
| --- | --- | --- | ---: |
| npm run typecheck | PASS | tsc --noEmit，exit 0 | 5s |
| EXPO_NO_TELEMETRY=1 npm run web:build | PASS | Metro 301 modules；Exported: dist | 2s（首次 3s） |
| npm run test:planning | PASS | 14 tests，14 pass，0 fail | 1s |
| npm run test:workflow | PASS | 6 tests，6 pass，0 fail | <1s |
| npm run test:prd-client | PASS | 52 tests，52 pass，0 fail | <1s |
| npm run test:prd-web | PASS ×5 | 每轮 69 个 PASS 检查点；连续 5 轮全绿 | 188／189／189／189／189s；总 944s |
| smoke_prd_observations_web.py ×5 | PASS ×5 | 原竞态触发脚本连续全绿 | 总 39s |
| npm run test:sync | PASS | 14 tests，14 pass，0 fail | 4s |
| cargo test --workspace | PASS | 148 tests，0 fail | 12s |
| python3 -m py_compile tools/smoke_prd_*.py | PASS | 所有 PRD smoke 语法通过 | <1s |
| node tools/check-prd-maintenance.mjs | PASS | 离线导入、备份、错误模型拒绝、恢复及会话撤销 | 4s |
| npm run test:prd-runtimes | FAIL（环境前置） | native SQLite 13 组＋XLSX 导入导出全部 PASS；Miniflare 加载 DO 前缺 crates/worker/build/index.js | 15s |

### test:prd-runtimes 详细结果

第一次尝试：

- native SQLite 前 13 组通过；
- prd-export-contract 仍固定调用 Python 2，已在本切片统一修复。

第二次尝试：

- native SQLite 的认证、课记、计划总结、知识复盘、队列、actor guard、365 天会话、观察测量、XLSX 导入、队列快照、字段 presence、方法比较期、课记反馈全部 PASS；
- PRD XLSX verifier PASS；
- authenticated Excel export PASS；
- 随后 Miniflare 报 ENOENT：crates/worker/build/index.js 不存在。

环境判断：

- miniflare 已安装并成功启动到读取 Worker 入口阶段。
- 当前机器没有 worker-build 命令，也没有预生成 Worker build 目录。
- 因而本轮没有执行 Rust Worker／DO 业务断言；失败是缺少构建工具／产物，不是已执行 DO 断言失败。
- 按硬约束，本切片没有安装 Worker 发布工具或修改业务逻辑来强行跑通 C-1。

## 六、本切片发现的新缺陷（未修）

没有发现新的 A0 业务缺陷。

发现一个 C-1 环境前置缺口：

- 位置：tools/check-prd-runtimes.mjs:31 期望 crates/worker/build/index.js。
- 现象：worker-build 0.8.7 未安装，构建产物不存在。
- 影响：本机可完成 native SQLite 全合约，但无法进入 Miniflare Durable Object 合约。
- 建议：C-1 开始前按 docs/runtime-storage.md 安装锁定版本 worker-build 0.8.7 和 wasm32-unknown-unknown target，执行 crates/worker 的 worker-build --release，再重跑 test:prd-runtimes。
- 本切片未修原因：属于双运行环境构建／验收前置，不属于本切片三项业务范围；prompt 明确要求失败时如实区分，不为跑通修改业务逻辑。

## 七、后续切片建议

1. C-1 前置：安装锁定的 worker-build／Wasm target、生成本机 Worker artifact，完成真实 Rust Worker／DO 对账；不触发部署。
2. A1-1：今天页补 Asia/Shanghai 业务日期和统一作者／记录时间。
3. A1-2：课记撤销 capability 与入站引用预检。
4. A2-1：补目标 domain／due／基础帮助条件。
5. A2-2：总结新困难结构化保存成人回应、猜测与条件。
6. B-1：区分普通方法完整创建与计划／总结名称快捷创建。
7. C-1 完整阶段：附录 D、Excel、SQLite／DO、移动浏览器全量交叉验收。

## 八、未做与原因

- 未安装 worker-build、未生成 Worker 发布产物：属于 C-1 构建前置；已尝试 runtime test 并精确记录阻塞点。
- 未执行真实 iOS／Android 系统 Back／手势：本轮是桌面 Chromium history API 实测，不冒充真机。
- 未修改 A1／A2／B／C 业务能力。
- 未修改生产配置、迁移策略、Excel 业务映射或生产数据。
- 未部署、未 push。

## BLOCKED-NEEDS

本切片本身无阻塞，代码、提交及要求的 A0 测试已完成。

可选的 C-1 前置阻塞：若下一切片要求完成 test:prd-runtimes 的 DO 部分，需要先提供或安装 worker-build 0.8.7 与 wasm32-unknown-unknown target，并本地生成 crates/worker/build/index.js。
