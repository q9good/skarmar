# Phase 2 切片 1 实施报告（A0-3：测试入口 + R15）

- 基线：起始提交 3ce27f9eb2d08b660b390bb6a3adb8d57f38324e
- 结束提交：4d65479cbae5b22178900b9ed54e641a339081b9
- 分支：feat/tencent-informed-web
- 日期：2026-10-25
- 提交：fix: align R15 controls and test runners
- 边界：只修改 A0-3；未 push、未部署、未改 wrangler.toml、未启用生产 SKARMA_MODEL、未访问生产账号或业务数据、未实施 L-2／L-3

## 一、测试入口修复

### 1. Node 22 无法直接导入 TypeScript

问题：

- test:planning、test:workflow、test:prd-client 直接导入 apps/client/src 下的 .ts。
- Node 22.11.0 原入口报 ERR_UNKNOWN_FILE_EXTENSION。

改动：

- 沿用仓库已有 node --test 方式，不引入 tsx／ts-node／新构建器。
- 使用 Node 22 内置的 --experimental-strip-types：
  - package.json:15：test:planning
  - package.json:21：test:workflow，并统一为 node --test
  - package.json:26：test:prd-client

验证：

- npm run test:planning：14/14 PASS。
- npm run test:workflow：6/6 PASS。
- npm run test:prd-client：49/49 PASS。
- Node 会输出 ExperimentalWarning 和 MODULE_TYPELESS_PACKAGE_JSON 警告，但退出码为 0；测试断言未删除或放宽。

### 2. PRD 浏览器测试固定调用 Python 2

问题：

- tools/check-prd-web.mjs 原来固定 spawn python。
- 本机 python 是 2.7.18，含中文的 Python 3 脚本会在解析阶段失败。

改动：

- tools/check-prd-web.mjs:6 使用 PYTHON3 环境变量，未设置时明确用 python3。
- tools/check-prd-web.mjs:20 用该解释器启动脚本。

验证：

- 浏览器脚本实际进入 Playwright，不再出现 Python 2 编码 SyntaxError。
- 最终 npm run test:prd-web 完整通过，耗时 185 秒。

### 3. 浏览器路径固定为不存在的 /usr/bin/chromium

问题：

- Playwright 已安装托管 Chromium，但 19 个 PRD smoke 脚本多数硬编码 /usr/bin/chromium；本机没有该可执行文件。

改动：

- 19 个 tools/smoke_prd_*_web.py 改为 executable_path=os.environ.get('CHROMIUM_EXECUTABLE')。
- 未显式配置时由 Playwright 使用其安装管理的 Chromium；需要自定义浏览器时仍可通过 CHROMIUM_EXECUTABLE 覆盖。
- 这是测试入口修复，没有改业务逻辑。

验证：

- 所有默认 PRD smoke 脚本均被真实 Chromium 执行。
- 最终完整浏览器套件 PASS。

### 4. 移除常驻刷新后调整既有 smoke

问题：

- smoke_prd_polish_web.py 依赖常驻“刷新”按钮同步通过 API 直接写入的合成课记。
- R15 要求删除该常驻入口。

改动：

- tools/smoke_prd_polish_web.py:35 改用仓库已有的 focus 事件同步模式，并等待 /api/prd/state 响应。
- 未删除断言，也没有降低课程结算／搜索验证强度。

验证：

- 单独运行 smoke_prd_polish_web.py PASS。
- 完整 test:prd-web 中该脚本 PASS。

### 5. 测试环境补充

- smoke_prd_export_web.py 和 smoke_prd_import_web.py 还依赖 openpyxl；本机最初未安装。
- 为完成本轮真实浏览器套件，使用 pip3 install --user openpyxl 安装 openpyxl 3.1.5 和 et-xmlfile 2.0.0。
- 这是本机测试依赖，不是仓库业务代码改动。

## 二、全量测试实跑结果

以下均为本轮、结束提交前后的真实运行结果。最终提交后工作树为空。

| 命令 | 结果 | 关键输出 | 耗时 |
| --- | --- | --- | ---: |
| npm run typecheck | PASS | tsc --noEmit，exit 0 | 5s |
| EXPO_NO_TELEMETRY=1 npm run web:build | PASS | Metro 301 modules；生成约 1MB bundle；Exported: dist | 2s（热缓存；首次 8s） |
| npm run test:planning | PASS | 14 tests，14 pass，0 fail | 1s |
| npm run test:workflow | PASS | 6 tests，6 pass，0 fail | <1s |
| npm run test:prd-client | PASS | 49 tests，49 pass，0 fail | 1s |
| npm run test:prd-web | PASS | 默认 19 个 Playwright 脚本全部通过；含新增 R15 smoke | 185s |
| npm run test:sync | PASS | 14 tests，14 pass，0 fail | 3s |
| cargo test --workspace | PASS | API／core／worker 共 148 tests，0 fail | 12s |
| python3 -m py_compile tools/smoke_prd_*.py | PASS | 全部 PRD Python 脚本语法通过 | <1s |

补充运行：

- node tools/check-prd-web.mjs smoke_prd_r15_web.py：PASS。
- node tools/check-prd-web.mjs smoke_prd_polish_web.py：PASS。
- 在完整套件首次运行时，smoke_prd_observations_web.py 曾因浏览器 Back 竞态失败；隔离复现两次后，最终完整 npm run test:prd-web 又通过。该竞态没有通过删断言或加等待掩盖，记录在第四节。

## 三、R15 与 AC-R15-01…04 对齐

### R15：不常驻刷新，只在失败时重试

改动：

- apps/client/src/foundation-view.tsx:233 删除业务页页头常驻“刷新”按钮。
- apps/client/src/foundation-view.tsx:235 仅在 syncError 存在时显示错误卡和“重试同步”。
- tools/smoke_prd_r15_web.py:37-47 验证正常状态没有“刷新”，合成 /api/prd/state 失败后才出现“重试同步”，重试成功后入口消失。

结果：PASS。

### 原生日期双方式

改动：

- apps/client/src/controls.tsx:12-17 新增安全 ISO 日期校验、Asia/Shanghai 当前月份、月份步进、8 位数字格式化。
- apps/client/src/controls.tsx:18-38 原生 DateField 支持：
  - 日历／数字输入切换；
  - 未完成数字输入保留；
  - 8 位数字转 YYYY-MM-DD；
  - 日历月份前后移动、44px 日期按钮、清除日期；
  - 同一时刻只显示一种输入方式。
- Web 继续使用既有 controls.web.tsx 双方式，不引入第二套 Web 控件。

结果：

- TypeScript PASS。
- Web 日期既有 smoke 全部通过。
- 原生控件完成静态类型与源码核对；本切片没有原生真机运行环境，未冒充真机验证。

### AC-R15-01：示例不进入请求或导出

实现／证据：

- 既有 GuidedField／ExampleHint 保持 placeholder 与 value 分离。
- tools/smoke_prd_r15_web.py:55-70 在目标表单保留空观察标准。
- tools/smoke_prd_r15_web.py:111-117 保存后通过真实 API state 核对：
  - criteria 仍为空字符串；
  - 保存对象 JSON 不含“例如：”示例文本。

实测结果：PASS。

仍缺：无。Excel 导出对未提交输入／示例的既有 smoke 也在完整套件中通过。

### AC-R15-02：已有输入时，问句与完整例子仍可读取

实现／证据：

- tools/smoke_prd_r15_web.py:63-70：
  - 先填写生活意义；
  - 常驻问句“这会帮助哪方面生活？”仍可见；
  - 展开“合成示例 · 例如：更独立地吃饭”；
  - 原输入保持不变。

实测结果：PASS。

仍缺：无。

### AC-R15-03：200% 缩放且键盘开启，操作栏不遮挡焦点

改动／实测：

- 既有 apps/client/src/overlay.tsx 继续使用 visualViewport 高度、独立滚动区和固定 footer。
- 新增 tools/smoke_prd_r15_web.py:49-53：
  - 360×640、430×932、320×740 三种 CSS 视口；
  - 均无整页横向滚动。
- tools/smoke_prd_r15_web.py:86-94：
  - 通过 Chromium CDP Emulation.setPageScaleFactor=2 做 200% 页面缩放；
  - 640px 视口在 2× 下形成约 320 CSS px 可视范围；
  - 保存主按钮完整可见，无横向滚动。
- tools/smoke_prd_r15_web.py:96-109：
  - 聚焦生活意义输入框后，把可视区缩到 360×360 模拟软键盘占用；
  - 焦点保持；
  - 输入框和固定保存按钮均位于 visualViewport 内；
  - 两者边界不重叠。

实测结果：PASS（桌面 Chromium 模拟）。

仍缺：

- 不是 iOS Safari／Android Chrome 真机软键盘。
- 2× 是 Chromium 页面缩放模拟，不是操作系统级“仅文本缩放”或真机辅助功能字号。
- 因此本切片不把 AC-R15-03 写成真机验收完成；代码级／桌面浏览器验收完成，真机仍须后续留证。

### AC-R15-04：字号、热区和对比度

实测方式：

- tools/smoke_prd_r15_web.py:72-84 读取浏览器实际 computed style，不按源码常量推定。
- 验证：
  - 输入框 ≥16px；
  - 字段标签 ≥17px；
  - 常驻问句／说明 ≥14px；
  - 主按钮文字 ≥17px；
  - 保存按钮宽高均 ≥44px；
  - 输入文字、placeholder、标签、说明、主按钮文字分别与实际祖先背景计算对比度，全部 ≥4.5:1。

实测结果：PASS。

本次实际样式对应：

- 输入正文 16px；
- 标签 17px；
- 说明在 Readability 包装后不低于 14px；
- 主按钮文字 17px；
- 关键色值的对比度约为：正文／白底 11.69、placeholder／白底 5.84、辅助文字／浅底 5.63、白字／主绿 5.40。

仍缺：不同系统字体渲染的真机视觉复核；这不影响 CSS 数值／颜色对比断言。

## 四、本切片发现的新缺陷（不属于本切片范围，未修）

### 1. 覆盖层 history 哨兵存在短时 Back 竞态

- 位置：apps/client/src/overlay.tsx:58-62；触发脚本 tools/smoke_prd_observations_web.py:34-35。
- 现象：
  - revision history 对话框已可见；
  - useEffect 中的 pushState 尚未完成时立即调用浏览器 Back；
  - 两次隔离复现中页面跳到 about:blank，后续删除按钮自然不存在；
  - 最终完整套件又通过，说明是时序竞态而非稳定失败。
- 影响：极快的系统返回操作可能退出应用，而不是只关闭顶部覆盖层；涉及 R14／AC-R14-04，不属于本切片指定的 R15-03／04。
- 建议归属：后续 A0 可靠性切片。应让覆盖层在可交互前完成哨兵登记，或让 Back 处理在 sentinel pending 状态也能拦截；新增确定性竞态测试，不应仅给现有测试加 sleep。
- 本切片处理：未改生产代码、未删除现有断言、未隐藏失败；最终完整套件 PASS 但保留此发现。

### 2. Python 浏览器测试依赖未在仓库集中声明

- 位置：tools/smoke_prd_export_web.py:6、tools/smoke_prd_import_web.py:5，以及多份 Playwright 脚本。
- 现象：环境虽然安装了 Playwright，但首次跑到导出脚本时缺 openpyxl。
- 影响：新机器上的 test:prd-web 仍依赖人工知道 pip 包集合。
- 建议归属：后续测试基础设施切片，增加只用于测试的 Python requirements／安装说明，至少声明 playwright、openpyxl；不要并入腾讯业务依赖 tools/tencent-requirements.txt。
- 本切片处理：只在本机安装 openpyxl 3.1.5 以完成实跑，没有扩展业务范围。

### 3. 其他 PRD 合约脚本仍有固定 python 调用

- 位置：tools/prd-transfer-contract.mjs:9。
- 现象：该脚本通过 execFileSync('python', ...) 生成工作簿；本机 python 是 Python 2，且 openpyxl 安装在 Python 3 环境。
- 影响：未来执行 test:prd-runtimes 可能在数据转换合约处遇到同类解释器问题。
- 建议归属：C-1 双运行环境验收／测试基础设施；本切片的必跑清单不含 test:prd-runtimes，且硬约束要求不扩大到 L-2／L-3，因此未修改。

## 五、后续切片建议（按依赖顺序）

1. A0 可靠性补片：修复 overlay sentinel pending 期间的 Back 竞态，确定性验证 AC-R14-04。
2. A0-2：补离线未尝试上传的取消能力、可信设备到期前 14 天提示。
3. 测试基础设施：集中声明 Python smoke 依赖；统一剩余合约脚本的 Python 3 入口。
4. A1-1：今天页补业务日期和统一的作者／记录时间展示。
5. A1-2：课记撤销 capability 与入站引用预检。
6. A2-1／A2-2：目标字段补齐；总结新困难结构化字段。
7. B-1：区分普通方法完整创建与计划／总结快捷创建。
8. C-1：恢复并实跑 SQLite／真实 Rust Worker／DO 双运行环境、附录 D 与导入导出全量对账。

## 六、未做与原因

- 未做 iOS／Android 真机软键盘：当前只有桌面 Chromium；已明确标注为 visualViewport 缩高模拟。
- 未做操作系统级文本-only 200%／辅助功能大字：当前为 Chromium 2× page scale；不冒充真机文字缩放。
- 未运行 test:prd-runtimes：不在本切片明确必跑清单，且会进入 C-1／L-2、L-3 邻接范围；本切片没有扩大。
- 未修 overlay Back 竞态：属于 R14 可靠性，不是本切片指定的 R15-03／04；已保留证据与后续建议。
- 未修改 PRD、生产迁移、Excel 业务映射、生产配置或部署资源。
- 未 push；只创建本地提交 4d65479。

## BLOCKED-NEEDS

无。本切片代码、提交和要求的最终测试矩阵已完成。真机键盘／真机文本缩放仍需后续在实际设备验收，但不阻塞本切片的桌面浏览器交付。
