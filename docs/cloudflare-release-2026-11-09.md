# 七项交互与课堂优先级迭代发布（2026-11-09）

状态：**已上线并通过公开检查**。入口：**https://skarma.vip**。
用户明确授权逐条核对后直接普通部署。应用`2eed906ceb5a2694a5eecaee50696773e702ab7e`，功能、七项实际结果与9图见[U01–U07](iteration-2026-11-09.md)。

## D01发布验收

| 步骤 | 验收 | 实际结果 |
| --- | --- | --- |
| 精确来源 | 原分支最新源码、保护和产物、期间不移动分支 | 应用2eed906，触发ac9a3de；41项发布保护、9项示例保护、219项SHA先通过，remote基线核对；run完成前分支不变 |
| 原配置 | 比较原Worker／DO／default、域名精确来源、变量／runtime／路由 | runner比较原线上配置及候选保持一致后才激活，Secrets不变，私密配置清理成功 |
| 真实发布 | validate／release成功，上传与实际激活一致 | [run42583812805](https://github.com/q9good/skarmar/actions/runs/42583812805)均success，`64921193-38e0-4d50-98d7-f223a44d0eb1`，deployed_and_verified，100%流量 |
| 独立公开检查 | 首页／JS字节、health／匿名／来源、PRD启动及代际 | runner和云端独立检查通过，发布前后formal-20261101完成标记保持 |
| 数据保留 | 清理、初始化、示例恢复、旧目标删除、补关联及维护演练跳过 | 全部skipped，未登录生产账号、读取业务或写入样本；两份脱敏结果登记后221项SHA |

## 版本与回执

- 应用：`2eed906ceb5a2694a5eecaee50696773e702ab7e`；部署源码：`ac9a3de698db76cecf8979155db29ee7cb252a9e`（在已验收应用上增加发布文档与CI构建诊断，业务代码不变）。
- 原版本：`d4e115b1-8270-42c6-97e7-112ac119d2fd`；上传并实际激活：`64921193-38e0-4d50-98d7-f223a44d0eb1`。
- [脱敏回执](artifacts/deployment/cloudflare-release-20261109-receipt.json)／[独立公开结果](artifacts/deployment/cloudflare-release-20261109-independent-checks.json)。

runner重新执行类型检查、Web生产构建、185项Rust工作区测试及Wasm Clippy，读取原生产配置和已激活版本，构建候选后再次比较存储绑定、变量、runtime与域名路由，一致才切换100%。分支保持触发SHA至run结束，之后才记录和推送结果。

## 首轮失败与修复

首轮71bc58f/run42582451059的认证与原配置核对通过，回执停在preflight_passed，Wrangler dry-run失败，没有确认上传或激活；[首次脱敏回执](artifacts/deployment/cloudflare-release-20261109-first-attempt.json)保留。本机同一构建演练通过。随后增加无凭据的release构建步骤与限定错误分类诊断（4项脱敏测试），保持凭据／绑定／业务代码不变后受控重试，以本次实际成功回执为准；没有把重试成功反推首轮具体根因。

## 数据与验收范围

保留原Worker、SkarmaSpace／SKARMA_DB／default、skarma.vip及精确来源、账号和全部正式业务／历史／冻结引用、schema v7、formal-20261101及草稿／离线队列键。优先级规则存原prd_settings，实体与观察字段可选；旧档位固定读取换算、旧段落按一条保留，不做清库、自动拆分、补种、初始化或重建资源。

独立公开检查确认首页仅去严格匹配的CF Analytics标签后与本机本次构建一致，JS逐字节相同；health200／ok、匿名身份／状态401、未知API404，合法Origin退出200、非法Origin或缺标记403；随机未签发会话退出验证PRD启动，不注销家人会话。正式数据代际完成标记前后保持。

没有生产账号登录、逐条读取／比对真实业务或写入验收样本；数据保留依据为普通发布、原存储和配置不变、稳定代际及维护入口全关闭，不能声称真实记录逐条对账。本轮业务验收来自合成数据：185Rust、94客户端、类型／Web、双Clippy、Worker构建、双运行环境完整合约／重启和12组相关浏览器；扩充旧客户端携带items改文本后3专项再次通过。9图逐张检查仅合成，checks保留生成时deployed:false，不改写为线上业务截图。

沿用原GitHub Secrets发布通路；Cloud原生环境T12、Opus独立验收／真机／读屏和implementationCommit不变。后续继续普通发布，禁止重跑历史维护标记。
