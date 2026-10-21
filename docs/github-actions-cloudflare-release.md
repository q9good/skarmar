# 从 Codex Cloud 触发现有 Cloudflare 应用发布

更新：2026-10-22。云端通过 GitHub Actions 的真实发布及补验均已完成，T33 完成。
构建与发布运行在 GitHub runner，后续沿用下文的明确发布标记。
这条备用路径不修复 Codex 环境自身的凭据注入。原环境的 T12 继续受阻，
Mac 的 T32 版本为 `7398e1af-496e-415e-b113-7c6d0b68a45c`；随后云端首次真实上传与激活
已成功，新版本为 `d7925a8a-127b-4773-bc8a-445ce12460eb`，首轮公共检查失败及补验见下文。

## 原生环境与权限检查

本任务只读检查仍为 spec revision 4、最新观测、无网络密钥绑定，Wrangler whoami 未认证。
当前可读草案仍为空密钥列表，不能拿它覆盖另一新 Edit 会话中已保存的 Token。
诊断工具没有留存记录，可用工具仅有状态、诊断、草案读写，没有解除发布锁定的恢复操作。
不重新提交 stale_base 草案、不绕过禁用按钮、不创建 Token 或新环境。
此前 Bug 反馈已提交且收到 Thanks for your feedback，没有工单编号。

GitHub 仓库为 public、默认分支为 `main`。本次检查前没有工作流和 GitHub Environment。
当前 GitHub API 授权可读仓库和 Actions 工作流；读取 Actions 设置、仓库 Secrets 和
Variables 元数据返回 403 Resource not accessible by integration。仓库元数据虽显示
admin=true，也不能据此声称可管理 Secrets。Git 推送权限与 API 权限分别验证。

## 需要安全配置的两项 Secrets

进入 [仓库 Actions Secrets 设置](https://github.com/q9good/skarmar/settings/secrets/actions)，
选择 New repository secret。只在安全字段输入，不贴到聊天、Issue、Git 或公开日志：

| 名称 | 内容 |
| --- | --- |
| `CLOUDFLARE_API_TOKEN` | 复用持有人今天创建的 `codex-cloud` Token；不要为绕过平台故障再创建一个 |
| `SKARMA_WRANGLER_TOML` | Mac 已成功发布时使用的完整私密 `deploy/cloud-worker/wrangler.toml`，包含明确 `account_id`、原主来源、`https://skarma.vip` 额外来源和 Custom Domain；不能填写 example 或根目录开发配置 |

GitHub 无法从另一个平台取出加密保存的 Token。若原值已不在持有人的密码管理器等安全
保存位置，不能要求代理提取；须先由持有人决定凭据恢复方式，不擅自生成／轮换。
GitHub Secrets 为该 runner 提供环境变量，Codex 的目的域名注入规则不会自动转移到 GitHub。
私密 TOML 不应含任何密码／Token；脚本拒绝 vars 中的 password/secret/token/cookie 字段。

复用 Token 时保留原账户范围和 `api.cloudflare.com` 目标。按 Cloudflare 官方 GitHub CI
指导，Edit Cloudflare Workers 模板是已文档化的基线。本流程需要的能力是：

| 操作 | 权限边界 |
| --- | --- |
| 读取现有版本／部署、上传代码和静态资源、切换版本流量 | 原账户 Workers Scripts Edit（包括读取；部分界面显示 Write） |
| 读取原 Custom Domain、workers.dev、script settings | 现有 Workers 元数据只读；不改域名或 DNS |
| `wrangler whoami` 人工诊断 | 可能另需 Account Settings Read；实际发布通过配置里的明确 account_id 定位，不靠枚举账户 |

本流程不执行 Workers Routes 写入、Zone/DNS 写入、KV 管理、DO 数据查询或数据库创建。
官方 OpenAPI 的本轮端点定义没有列出细分 permission group，尚无真实 Token 调用证明
缩减权限后的组合可用；因此不声称已实际验收最小 Token。先复用原 Token 做受控 preflight，
如 403，再依据具体被拒端点核对现有权限，不先扩到全账户或要求 Global API Key。

## 发布触发

[工作流](../.github/workflows/cloudflare-release.yml) 对共享分支每次 push 跑隔离 guard 测试，
**普通提交不会发布**。当前工作流尚未进入默认 `main`，GitHub 的 `workflow_dispatch`
要求默认分支存在工作流，不能只添加文件就声称 Run workflow 已可用。

不修改默认分支时，使用明确的 push 发布指令：先提交所有工作并按仓库流程检查，然后：

```bash
# 仅在决定发布且两项 Secrets 就绪后执行；此标记将触发生产发布。
git commit --allow-empty -m 'Release reviewed application [deploy-cloudflare]'
npm run sync:push
```

以触发 push 的最新提交 SHA 为唯一发布源码，不能另选未审查 ref。普通验证任务不接触
Secrets；发布 job 只允许原仓库和共享分支、缺任何一项 Secret 立即失败，且不安装工具。
工作流权限只有 contents:read；不需要 GITHUB_TOKEN 的 Actions 写权限来响应 push。
同一生产目标串行执行，排队时不取消已开始的发布，构建后若分支已前进则中止过时任务。
当前授权是否允许实际提交工作流与运行 Actions，须以 Git 推送和真实运行回执为准。

工作流进入默认分支且当前授权允许 Actions write 后，可改用：

```bash
gh workflow run cloudflare-release.yml --repo q9good/skarmar \
  --ref feat/tencent-informed-web -f source_sha="$(git rev-parse HEAD)"
```

这不授予未来所有业务改动自动上线的授权；每次发布应按持有人当前任务范围执行。

若上传和激活已成功、只需补验公共访问，不要重复部署。提交监测修复后，可用明确的
`[verify-cloudflare:<实际激活版本UUID>]` push标记。该模式通过API读取并锁定预期版本，
重新构建Web以比较静态资源，跳过Rust安装/测试/构建，不调用版本上传或流量切换。
dispatch入口可用时，也可指定`verify_version`输入。回执的`source_sha`是验收脚本提交，
`deployed_source_sha`从原线上版本注解读取，不能将验收提交说成已上传的应用源码。

## 版本发布与数据保护

1. 固定官方 Actions 提交 SHA、Node 24、Rust 1.99.0、worker-build 0.8.7 与 npm/Cargo 锁文件。
   执行类型检查、Web 构建、Rust 测试、Wasm Clippy，再进入发布步骤。
2. 只在 runner 临时写入忽略的私密配置（权限 0600）。校验原 Worker、入口、生产开关、
   `SkarmaSpace/SKARMA_DB/default`、两类精确来源、v1 DO 迁移和固定构建命令。
3. 读取原应用部署及版本元数据；不存在、分流状态、配置 vars 与线上不同或 bootstrap
   secret 意外残留即停止。不查询对象内容、账号或训练记录。
4. 保存旧版本号及内存中的原命名空间／配置元数据。执行实际 Wrangler dry-run。
   再检查部署和路由状态，避免覆盖构建期间 Mac／Dashboard 的并发变更。
5. 使用 `wrangler versions upload --keep-vars` 只上传新版本与静态资源；CLI 对不存在的
   Worker 或待应用的 DO 迁移会拒绝。比较候选版本的命名空间、全部原变量、秘密名称及
   runtime 元数据，任何不同都不切换流量。
6. 再次确认原部署没变，调用官方 `POST /accounts/{account_id}/workers/scripts/skarma-web/deployments`
   将唯一候选版本设为100%流量，不附带force。没有使用 CLI `versions deploy`，因为本轮
   核对其4.149.0实现会额外同步非版本化日志设置。此处只发版本部署请求，不调用
   `triggers deploy` 或 PATCH settings，不改 Custom Domain、DNS、workers.dev、日志设置或定时任务。
   部署后核对原绑定／settings／域名元数据与实际激活版本。
7. 首页排除已识别的Cloudflare Analytics标签及其附带换行后与构建逐字节比较，引用JS
   始终逐字节比较。未知脚本、应用变化或重复Analytics标签仍失败。监测客户端明确标识为
   `SkarmaReleaseCheck/1.0`，不伪装浏览器，不关闭站点防护。health=200、匿名 me/state=401、未知 API=404；
   无 Cookie 的合法来源 logout=200、非法来源／无请求标记=403。不登录或新增线上测试数据。

保留旧代码版本不等于整库备份。当前用户明确这些是测试数据、无需备份，故本轮允许
受控重发而不建立备份；将来数据性质变化须重新按部署任务确定备份要求。不能用 Excel
或元数据对比冒充完整备份。应用 SQL 迁移与 DO 命名空间迁移不同；未来增加 DO 迁移、
修改绑定／来源或引入其他资源，应单独评审，本流程默认拒绝。

GitHub concurrency 只能约束本工作流，两次只读检查之间仍存在极短竞争窗口。发布期间
不要同时从 Mac/Dashboard 切换版本。失败不自动重试上传／切换，不自动代码回滚或恢复 DB。
如果流量切换成功但公共检查被 1010 等边缘规则阻塞，回执会记录已部署但验收未完成，
不能把红色 workflow 一概解释为未部署，也不能把 dry-run 当作线上成功。

## 日志与验收证据

`tools/cloudflare_release.py` 捕获 Wrangler 可能含变量值的原始输出，不打印或上传。
公开 artifact 仅为 `.local/cloudflare-release/receipt.json`，记录源码 SHA、旧／上传／
实际激活版本号、阶段、匿名检查及脱敏错误。临时私密配置和日志在 job 最后删除。
不上传整个 `.local/`、工作区、配置、真实数据库或账号截图。

本地 15 项隔离测试通过，覆盖目标／空间／来源／bootstrap／命令拒绝、候选绑定变化、
两次并发检查、dry-run 不算上线、已切换后公共验收失败，以及 JS 字节和来源合约。
使用合成账户／来源实际执行 `wrangler versions upload --dry-run --keep-vars` 退出 0，
version_id=null，gzip 1536721 bytes；没有访问生产或实际上传。
工作流 YAML 结构和 Actions 固定 SHA 检查通过。真实 Actions 首次运行已通过：源码 `e486243dcf6974611d645e0912a91c5f3f381f0f`，
[run 39608250391](https://github.com/q9good/skarmar/actions/runs/39608250391) 的 validate 成功，
15项guard测试实际执行；普通提交的 release job 为 skipped。实际推送工作流与push触发权限
已证明可用，不能将该绿色验证run称为Cloudflare发布成功。
### 首轮真实发布与监测修复

用户在GitHub安全页面完成两项Secrets配置，值未读取或复制到本任务。Mac只读校验
原TOML与7398e1af线上版本的vars/runtime匹配后，云端使用明确标记提交
`d6834fab1db81b94b73cb9ad9b70af766c039496`触发
[run39615145891](https://github.com/q9good/skarmar/actions/runs/39615145891)。固定依赖、
Web构建、Rust测试和Wasm Clippy通过；实际CF授权、原存储及配置核对、上传、激活成功。
新版本`d7925a8a-127b-4773-bc8a-445ce12460eb`，首轮receipt的state为
`deployed_public_checks_pending`，并非未发布。完整run因后续公共首页非200而红色结束。
[首轮脱敏回执](artifacts/deployment/cloudflare-release-20261021-initial-receipt.json)已保存。

本云端对照普通客户端：Python-urllib默认UA得到1010/403，明确标识的监测UA及curl均200。
同时查明公开首页增加一个Cloudflare Analytics脚本和换行，除此之外与构建完全一致，
实际JS文件逐字节相同。修复明确监测UA，并只移除已核实的特定外部Analytics空内容script
及紧随换行；没有宽泛忽略HTML或JS差异。未知/伪造/重复标签有拒绝测试。
新增“仅补验现有版本”路径，21项隔离测试及本云端完整匿名smoke通过。
未改WAF/BIC/其他访问防护、未扩Token权限、未重复上传或切换版本。
### Actions 最终补验已通过

修复提交 `a788b18077866d44f2ebbeebc07f0bdea00861aa` 触发
[run 39617784667](https://github.com/q9good/skarmar/actions/runs/39617784667)，
validate 和 release 作业均成功。回执为 `verified_existing_version`，实际激活版本仍是
`d7925a8a-127b-4773-bc8a-445ce12460eb`，原部署源码仍为 `d6834fa`；补验未上传或切换版本。
补验执行类型检查、Web 构建、配置和版本元数据核对及公开检查，按设计跳过 Rust 安装／
测试／构建；Rust 等完整发布检查已在首轮真实发布中通过，不能混称为本次补验重跑。

[最终脱敏回执](artifacts/deployment/cloudflare-release-20261021-verified-receipt.json)
确认首页只排除一个严格匹配的 CF Analytics 标签后与构建一致，实际 JS 字节一致，
health=200、匿名身份与状态=401、合法来源通过、非法来源及缺标记被拒绝。
原存储、来源和域名元数据核对通过；本轮未查询或写入业务数据。

2026-10-22 本地通过 GitHub 连接器独立复核这次运行的两个作业、成功步骤和日志中的
最终回执，并安全同步修复。云端已报告补验成功，随后在最后整理文档时因用量限制中断；
本地据实际运行证据补齐本节和 T33 状态，不将这次文档收尾中断视为发布失败。

## 官方依据

- [Cloudflare：GitHub CI 与 Token](https://developers.cloudflare.com/workers/ci-cd/external-cicd/github-actions/)
- [Cloudflare：API 权限分组](https://developers.cloudflare.com/fundamentals/api/reference/permissions/)
- [Cloudflare 官方 API schema](https://github.com/cloudflare/api-schemas)
- [GitHub：workflow_dispatch 事件限制](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_dispatch)
- [原云环境发布故障](environment/cloudflare-token-publish-investigation.md)
- [Mac 实际发布回执](cloudflare-release-2026-10-21.md)
