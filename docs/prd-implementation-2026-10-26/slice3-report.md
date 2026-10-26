# Phase 2 切片 3 实施报告（A1-1：今天页业务日期与已记录信息）

- 基线：起始提交 ec0381044bff43ca6701cb841ee7e2d576754762
- 结束提交：f1ecb3517c908566b4d9dfff21c58b8d8629fcd0
- 分支：feat/tencent-informed-web
- 日期：2026-10-25
- 提交：feat: complete R01 today record metadata
- 边界：未 push、未部署、未访问生产、未修改 wrangler.toml／生产开关、未实施 A1-2／A2／B／C 或 L-2／L-3

## 一、业务日期

### 改动

- apps/client/src/foundation-view.tsx:228 继续用既有 localDate() 取得当前家庭业务日。
- apps/client/src/prd-today.ts:8-11 新增 businessDateLabel：
  - 输入 localDate() 返回的 YYYY-MM-DD；
  - 仅根据该日历日期计算星期；
  - 输出“YYYY-MM-DD 周X”。
- apps/client/src/foundation-view.tsx:239 在“今天”页标题同一紧凑行显示业务日期，并提供 accessibilityLabel“家庭业务日期”。
- 日期没有另起一张卡或增加一个页头区块，避免增加首屏高度。

### 时区处理

日期来源只有 apps/client/src/draft.ts:17-23 的 localDate()：

- Intl.DateTimeFormat 显式 timeZone=Asia/Shanghai；
- 不使用浏览器本地时区；
- 新代码没有引入第二套“当前日期”计算。
- prd-today.ts 只负责把已经确定的家庭业务日期加上星期，不重新读取本机时间。

时间显示也统一使用 Asia/Shanghai：

- apps/client/src/prd-today.ts:13-17 的 businessTime 对服务器 created_at 使用 timeZone=Asia/Shanghai，显示 HH:mm。
- 家庭干预已有明确时段标签时直接显示夜间／上午／下午／晚上，不伪造成精确创建时刻。

### 实测结果

tools/smoke_prd_today_web.py:12 创建 America/Los_Angeles 浏览器上下文；行 35-42 用独立 Intl 期望值核对页面“家庭业务日期”，结果 PASS。

纯逻辑测试：

- tools/test-prd-client.mjs:6-15。
- 固定输入 2026-10-25，验证显示 2026-10-25 周日。
- 验证 UTC 时间 12:04／11:00 按 Asia/Shanghai 显示 20:04／19:00。

结果：PASS。

## 二、今天已记录

### 改动

#### 统一数据模型

新增 apps/client/src/prd-today.ts：

- 行 22-27：筛选当天 status=recorded 的机构课记。
- 行 28-32：筛选当天未删除／未取消的家庭干预、共享计划和总结草稿。
- 行 33-36：筛选当天未删除的瞬间观察。
- 行 37：按服务器 created_at 倒序，ID 作为稳定 tie-break。
- 行 18-21：服务器创建时间距当前 5 分钟内显示“刚保存 · 已同步”，其他显示“已同步”。

每条统一包含：

- 记录类型；
- 一句可辨识内容；
- 记录者；
- Asia/Shanghai 创建时间，或家庭干预时段；
- 刚保存／已同步状态。

#### 统一展示

新增 apps/client/src/prd-today-view.tsx：

- 行 10-12：单一“今天已记录”卡和空状态。
- 行 13-17：统一展示类型＋时间／时段、内容、记录者＋同步状态。
- 机构课程和家庭干预保留既有可访问查看入口：
  - 课程使用“查看课记：”语义；
  - 家庭记录使用“查看家庭干预：”语义。
- 瞬间观察在卡内直接展示完整短文本，不增加新的详情路由。

为避免重复：

- apps/client/src/prd-lesson-view.tsx 删除原“今天的上课记录”重复列表。
- apps/client/src/prd-session-view.tsx 删除原“今天的家庭干预”重复列表，保留主入口及“另建一次干预”。
- apps/client/src/prd-observation-view.tsx:65 今天模式只保留“记一个瞬间”入口，不再重复渲染瞬间卡。

### 作者／时间／状态展示

| 类型 | 作者来源 | 时间来源 | 状态 |
| --- | --- | --- | --- |
| 机构课记 | Lesson.creator_name | Lesson.created_at → Asia/Shanghai HH:mm | 刚保存 · 已同步／已同步 |
| 家庭干预／计划／总结草稿 | Session.creator_name | Session.input.time_slot | 刚保存 · 已同步／已同步 |
| 瞬间观察 | creator_name，旧记录回退 audit.actor_name | created_at，旧记录回退 audit.at → Asia/Shanghai HH:mm | 刚保存 · 已同步／已同步 |

“刚保存”基于服务器创建时间，而不是本机是否刚点击按钮；所有条目都来自服务器已保存数据，因此“已同步”不会用于本机未上传草稿。

### 数据来源接口

没有新增轮询或新 API：

- 课记、家庭干预来自 FoundationWorkspace 已有的 /api/prd/state 数据。
- 瞬间观察继续由 ObservationHub 已有 /api/prd/process?view=observations&object_kind=moment 请求取得。
- apps/client/src/prd-observation-view.tsx:29 增加可选 reported callback。
- apps/client/src/prd-observation-view.tsx:61-62 只把已加载 moments 上报父层，不发第二份请求。
- apps/client/src/foundation-view.tsx:114-116 保存该既有结果；行 233-234 把 state＋moments 传给统一卡。
- 20 秒“有更新”检查保持原样，没有为今天卡增加冗余 timer。

### 实测结果

tools/smoke_prd_today_web.py:52-76：

1. 第二个账号“测试家人”创建目标、课程、老师、课记、正式家庭干预和瞬间观察。
2. 第一账号重新加载今天页。
3. 统一卡同时出现：
   - 合成语言课 · 合成王老师；
   - 合成家庭观察；
   - 合成瞬间观察。
4. 课记／瞬间显示 HH:mm；家庭干预显示“晚上”。
5. 三条均显示“记录者：测试家人 · 刚保存 · 已同步”。

结果：PASS。

## 三、首屏布局不回归

### 布局位置

保持 PRD 优先顺序：

1. 紧凑今天页头＋业务日期，同一行；
2. 长期方向一行；
3. 主按钮“记录上课”；
4. 主按钮“记录一次家庭干预”／“填写今天的总结”；
5. 新的“今天已记录”卡；
6. 干预草稿／另建入口及“给我的待办”；
7. 课记回执／草稿；
8. 次要“记一个瞬间”。

技术落点：

- apps/client/src/foundation-view.tsx:247-251 保持长期方向和 LessonHub。
- LessonHub 先渲染“记录上课”，再渲染 interventionEntry。
- apps/client/src/prd-session-view.tsx:80 先渲染家庭主按钮。
- apps/client/src/prd-session-view.tsx:81 才渲染 afterPrimary，即统一“今天已记录”。

因此记录数量增加只会向首屏下方增长，不会插到两个主按钮之前。

### AC-R01-01 复核

tools/smoke_prd_today_web.py:27-33 定义首屏断言：

- 两个主按钮均 100% 在 viewport 内；
- 宽高均至少 44px；
- “记录上课”纵向位置在家庭入口之前；
- 页面无横向滚动。

行 35-50 实测：

- 0 个长期方向；
- 3 个长期方向；
- 6 个长期方向，且长期栏显示“另有 3 项”。

视口：360×640 CSS px。

在加入 3 条当天记录后，行 66-68 再次执行同一 first_screen 断言。

结果：所有情形 PASS。

说明：这是桌面 Chromium 手机视口模拟，不冒充 iOS／Android 真机。

## 四、多浏览者

### 两账号共享刷新

tools/smoke_prd_today_web.py 使用两个独立浏览器 context：

- owner：360×640，浏览器时区 America/Los_Angeles。
- family：430×932，浏览器时区 Asia/Tokyo。

第二账号通过真实认证 Cookie 和现有 PRD API 创建业务记录；第一账号执行正常页面 reload，复用现有 state／observations 加载路径，没有测试专用刷新 API。

### 作者显示

服务端创建者字段保持原模型：

- 课记 Lesson.creator_name；
- 干预 Session.creator_name；
- 观察 creator_name／audit.actor_name。

第一账号看到第二账号新增的三类记录时，均显示“记录者：测试家人”，而不是当前查看者或固定名称。

### 实测结果

- 跨账号课记、家庭干预、瞬间观察：PASS。
- 作者显示：3/3 正确。
- 创建时间／时段：3/3 正确。
- 刚保存／已同步状态：3/3 正确。
- 刷新后两个主按钮仍在 360×640 首屏：PASS。

## 五、全量测试实跑结果

全部为本轮真实运行。

| 命令 | 结果 | 关键输出 | 耗时 |
| --- | --- | --- | ---: |
| npm run typecheck | PASS | tsc --noEmit，exit 0 | 5s |
| EXPO_NO_TELEMETRY=1 npm run web:build | PASS | Metro 303 modules；Exported: dist | 2s（首次 5s） |
| npm run test:planning | PASS | 14 tests，14 pass，0 fail | <1s |
| npm run test:workflow | PASS | 6 tests，6 pass，0 fail | 1s |
| npm run test:prd-client | PASS | 53 tests，53 pass，0 fail；新增 Today 日期／记录映射测试 | <1s |
| npm run test:prd-web 第 1 次 | PASS | 70 个 PASS 检查点 | 194s |
| npm run test:prd-web 第 2 次 | PASS | 70 个 PASS 检查点 | 197s |
| npm run test:prd-web 第 3 次 | PASS | 70 个 PASS 检查点 | 189s |
| npm run test:prd-web 连续汇总 | PASS 3/3 | 无偶发失败；总计 580s | 580s |
| npm run test:sync | PASS | 14 tests，14 pass，0 fail | 4s |
| cargo test --workspace | PASS | 148 tests，0 fail | 11s |

补充定向实跑：

| 命令 | 结果 | 关键输出 | 耗时 |
| --- | --- | --- | ---: |
| smoke_prd_today_web.py | PASS | 上海业务日、0/3/6 首屏、三类记录、双账号作者刷新 | 6s |
| smoke_prd_lessons_web.py | PASS | 7 个课程工作流检查点 | 28s |
| smoke_prd_sessions_web.py | PASS | 7 个家庭干预工作流检查点 | 16s |
| python3 -m py_compile tools/smoke_prd_*.py | PASS | 新增脚本及全部既有脚本语法通过 | <1s |

第一次启动三连跑时，现有 lesson smoke 找不到原“查看课记：”语义，因为统一卡最初用了通用 accessible label。该问题不是通过修改测试解决，而是在 prd-today-view.tsx:17 恢复既有课记／家庭详情的可访问入口语义；随后课程、干预定向测试和完整三连跑全部通过。

## 六、本切片发现的新缺陷（未修）

没有发现新的范围内业务缺陷。

注意项：

- “刚保存”当前定义为服务器 created_at 距查看端当前时间不超过 5 分钟，并容忍 60 秒服务器／设备时钟偏差。超过窗口仍显示“已同步”；不会影响业务状态。
- 瞬间观察复用已有 observations 请求，因此另一浏览者通过完整页面刷新可立即看到；现有 20 秒检查仍只提示“观察有更新”，不自动挪动用户正在操作的列表。这与 R14 的稳定编辑语义一致，不在本切片改为实时插入。

## 七、后续切片建议

1. A1-2：课记撤销 capability 与入站引用预检。
2. A2-1：目标 domain／due／基础帮助条件字段。
3. A2-2：总结新困难的成人回应、原因猜测、条件结构化保存。
4. B-1：区分普通方法完整创建与计划／总结快捷创建。
5. C-1：安装 worker-build／Wasm target 后完成真实 Rust Worker／DO 对账及附录 D 全量验收。
6. 真机验收：iOS Safari／Android Chrome 的 360 类视口、系统字体和返回手势。

## 八、未做与原因

- 未改 A1-2 课记撤销逻辑：不属于本切片。
- 未给瞬间观察新增详情路由：PRD 要求今天列表展示，当前短文本已可直接阅读；新增路由会扩大范围。
- 未增加 WebSocket／SSE／新轮询：硬约束要求复用现有 state 语义，R14 也禁止编辑时后台更新挪动列表。
- 未做真机验证：本轮使用桌面 Chromium 360×640／430×932 模拟，不冒充真机。
- 未修改生产配置、迁移、导入映射或部署资源。
- 未 push。

## BLOCKED-NEEDS

无。本切片代码、提交、双账号验收和三次连续全量浏览器测试均已完成。
