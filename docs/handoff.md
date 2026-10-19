# 云端与本地开发交接

日期：2026-10-19。代码、方案、任务状态和选定产物通过 GitHub 的同一分支交接：`feat/tencent-informed-web`。应用验收对应提交以 `docs/workspace-state.json` 为准；后续提交可以只更新交接文件，具体以 Git 历史为准。

## 本地接续

首次在电脑上建立工作区：

```bash
git clone --branch feat/tencent-informed-web https://github.com/q9good/skarmar.git
cd skarmar
npm run sync:status
```

同步命令只需要 Git 和 Node，不需要先安装应用依赖。已有这个分支的工作区执行：

```bash
npm run sync:pull
```

如果本地仓库还没有这些命令，在确认工作区干净且已经切换到该分支后，先执行一次 `git pull --ff-only origin feat/tencent-informed-web`。本地分支不同、含有改动或已分叉时，先保留并处理自己的工作；不要用 reset、强制切换或强制推送来同步。

然后在这个项目中新建或继续本地 Codex 会话，让它阅读根目录 `AGENTS.md`、本文件和 `docs/workspace-state.json`。可直接输入：

> 读取 AGENTS.md 和 docs/handoff.md，核对 git 状态及任务清单，从我指定的下一项任务继续；保留本地修改，完成后更新交接状态。

应用运行见 [开发说明](development.md)：`npm ci`、`npm run web:build`，首次建立本机账号后执行 `bash tools/start-local.sh`。Windows 使用支持 Bash 的开发环境，例如 WSL。运行进程、依赖缓存、账号密码与数据库由每个环境分别准备。

## 双向更新

在完成工作的那一端更新代码、方案、任务状态和产物；核对改动，明确提交需要交接的文件，再推送：

```bash
git status
# 核对并 git add 指定文件，然后 git commit
npm run sync:push
```

另一端执行 `npm run sync:pull`，再读取任务清单。不要同时推进同一项任务或改同一份交接状态而不先拉取；已经分叉时人工合并／rebase 并保留双方工作。

| 命令 | 行为 |
| --- | --- |
| `npm run sync:status` | 读取本地分支、提交、工作区状态、任务快照并校验产物；不访问网络，领先／落后只对比上次获取的远端信息 |
| `npm run sync:pull` | 工作区干净、分支匹配时获取远端，验证产物并只做快进；拒绝分叉、本地独有提交和可能被覆盖的忽略文件 |
| `npm run sync:push` | 校验产物并推送已提交内容；不自动选择文件、提交或强推，远端已有新提交时由 Git 拒绝 |

这些命令按需运行。任务进度在提交、推送和另一端拉取后可见，不是两台电脑的实时文件镜像。

## 目前完成与待办

### 2026-10-19：skarma.vip 域名接入进行中

用户已在 Spaceship 购买 `skarma.vip`，授权接入现有 Cloudflare 应用。本地先安全快进到
云端最新研究提交 `640df50`，再进行本轮配置。操作、维护及待验收项见
[域名部署记录](custom-domain-deployment.md)（T26）。

已选 Free zone，Spaceship 与注册局 RDAP 均确认委派到本 zone 的 Cloudflare NS；旧 DS 已撤。
用户确认后删除两条扫描导入的停放页 A 记录，现有 Worker 已绑定根域 Production Custom Domain，
添加精确新来源并发布配置，原存储绑定与空间保留。经旧入口验证新域 Origin 的匿名 logout=200、
非法来源=403、health=200。本次未上传应用代码，T19 的界面更新仍未发布。

截至 20:22（Asia/Singapore），zone 仍 Pending，代表性公共 DNS 仍缓存旧 NS，新域 HTTPS 未验收。
Cloudflare DNSSEC 签名已开启，新 DS 尚未向 Spaceship 提交；应在 zone Active、旧 NS 缓存窗口
结束后恢复并验证 DNSSEC。大陆无代理登录、读取和保存仍需实际网络验收，不能把绑定已保存
当成访问问题已解决。后续云端发布前须补入新来源和 Custom Domain 配置，私密 Wrangler 文件不随 Git。

用户准备在 Codex Cloud「Set up skarmar」继续开发，本次材料入口见
[2026-10-19 云端开发交接](cloud-development-handoff-2026-10-19.md)。其中明确记录最新使用范围、
libp2p 调研边界、暂停部署、已提交的置顶代码，以及仍未提交并独立保留的导入工作区。

用户已明确只有家人使用，老师不操作应用；家人使用 iPhone / Android 混合设备，后续同步须覆盖两端。
课程中的授课老师是记录字段，不以老师账号、老师登录或老师端录入作为后续设计前提。
CloudKit 不作为主路线；继续比较原生 App + 网盘同步与微信小程序 + 云开发。

用户进一步提出 libp2p 主同步、网盘仅备份，已完成 [可行性评估](libp2p-family-sync-proposal.md)。
当前官方有 RN / 原生 WebRTC 路线，Expo 57 / RN 0.86 真机兼容尚未验证；仅备份模式
在无设备在线重叠或持久保存变更的第三节点时须等待同步。未实施或替换正式架构。
用户随后明确可用之前 Tailscale 故障的长期在线电脑作为家庭中心同步节点，持久接收再转交
变更；它当前仍不可达。云端先用模拟节点开发，再验收真实主机和手机网络。

2026-10-19 云端补充 [微信小程序成本对比](wechat-mini-program-costs.md)（T23）：
核对官网个人版限时 19.9 元/月与两类云托管计费差别；238.8 元仅为同价连续 12 个月的条件式
年化，未确认具体微信环境适用套餐、认证费或 Rust 容器全包价。家庭节点可省新增服务器租金，
原生 iOS 常规分发另有会员成本。上一轮微信体验环境发布后 15 天到期规则本轮未重新读取正文，
保留为历史结果，不将普通免费环境续期规则套到正式小程序。仅文档调研，未改应用或生产部署。

2026-10-19 按用户提供的四个链接完成 [CF 大陆访问方案核对](cloudflare-mainland-link-review.md)
（T24）：掘金正文已通过公开接口读到，提出 OCI 亚洲 VPS 反代原 Worker；另三份资料与官方
China Network 入门、产品列表及 FAQ 已交叉检查。优先自有域名直绑对照，必要时单一入口经
亚洲 VPS 访问原 Worker / DO，保留 Rust 和现有数据。已检查精确来源、Host Cookie、同源
API、草稿 origin 和 DO 标识；Nginx 示例仅供计划评审，未执行或验收。T20 大陆访问目标
仍在进行中，未购买、配置或部署资源，生产继续暂停。

2026-10-19 用户进一步倾向自有域名，已完成 [新增五链接核对与接入建议](cloudflare-custom-domain-options.md)
（T25）。五链接实际对应四篇独立原文：零度为客户端代理节点，腾讯为静态博客 CDN，
PlumePHP 的部分菜单与套餐建议已用官方文档纠正。先验证 Free zone + 原 Worker Custom
Domain，不预购 Pro / Argo，不先新增 VPS；SaaS 与优选 IP 仅作条件备选。已记录域名偏好
调整，不再将免域名作为当前硬约束；未提供 / 购买实际域名、改 DNS 或部署，T20 仍待真实验收。

2026-10-19 另完成 [长期免自有域名 / 服务器调研](no-owned-infrastructure-research.md)：
低用量成本优先时建议验证原生 App + 用户网盘 WebDAV；接受托管资源费时评估微信小程序
与云开发内置调用；全员苹果时考虑 CloudKit。网盘多人独立授权仍需原型；此前记录的微信
体验环境发布后 15 天到期规则，本轮未重新核对正文，具体见 T23 成本对比。
未实施同步、开通资源或改线上版本；不能将研究当实现验收。

2026-10-19 补充：不购买自有域名也有完整应用托管路线，已核对 Sealos、Railway、
Render 和 Zeabur 的默认地址、运行服务与持久化条件，见 [平台候选](default-https-platforms.md)。
尚未在候选平台部署或完成大陆无代理验收，生产发布继续暂停。
原云端「Set up skarmar」已提交的产品方案、技术设计和任务清单均能从 `f0ea443`
恢复；已在仓库外整理原始文档包并校验。索引见 [云端方案归档](cloud-document-archive.md)。
原任务的私有环境配置 ZIP 不在 Git，本次未取回。

结构化任务清单见 [workspace-state.json](workspace-state.json)，用稳定任务 ID 记录完成、进行中、受阻和待办。已实现登录与成员管理、训练闭环、十表 Excel、目的／途径／经验关系及规划环形图。相应设计、统计口径与验证入口均已在仓库文档内。

### 当前上线方向：完整 Cloudflare 托管

2026-10-19 用户要求先解决主要大陆用户的无代理访问，暂停新的云端应用发布。
“今天 → 长期关注”已置顶并提交 `bd5ed25`，本次类型检查和 Web 构建通过，但尚未上线；
该代码随 Git 同步，待入口方案确认后再发布。接入选项、当前证据与验收办法见
[大陆访问入口评估](mainland-access-plan.md)。代理下的 200 不代表大陆无代理访问已验收。

用户确认不再等待旧主机连接恢复，先上线空白云端实例，之后迁移旧数据。网页、Rust Wasm API 和 SQLite-backed `SkarmaSpace` 对象已发布到新的固定 Worker；运行不依赖主机、Tailscale 或 Mac。生产设置保持登录必需、无示例数据、精确 HTTPS 来源。配置模板在 `deploy/cloud-worker/`，完整步骤见 [云端部署与维护手册](cloudflare-hosted-deployment.md)。实际账号、配置与网址仍留私有运维记录。

本次 Wasm Clippy、Web 构建、TypeScript、发布包 dry-run，以及原生 / 真正 Rust Wasm 与 Miniflare DO 的共用合约全部通过；合约覆盖注册审批／拒绝、会话和权限、独立上课记录、十一表 Excel、SQL 失败回滚、并发版本和重启持久性。线上首页与 JS、健康 200、匿名 state / me 401、未知 API 404、合法来源空会话 logout 200、非法来源或无标记 403 均通过，API no-store。持有人已亲自安全设置密码，并明确确认 `owner` 成功登录；随后删除临时 bootstrap secret，Wrangler 返回删除成功。旧 Mac Quick Tunnel 已停止，关闭后新云端健康检查仍为 200。实际手机 Wi-Fi／蜂窝验收尚未执行。

### 旧主机与 VPC 网关的恢复记录

应用已经在用户常开主机原生部署，通过 Tailscale Serve 私有 HTTPS 访问；新版本在独立目录构建、先备份再切换，现有管理员和数据保留。固定入口采用 Workers VPC：`workers.dev` → Worker → VPC Service → 主机的命名 Tunnel → 原生应用和现有 SQLite，无需自有域名。网关位于 `deploy/worker-gateway`，5 项转发测试和 dry-run 通过。命名 Tunnel 的专用 token 已由用户安全保存，VPC Service 和固定 Worker 已发布，精确来源已加入应用配置。主机的 fake-IP DNS 导致初次连接超时，进程专用 hosts 覆盖后曾建立 4 条 QUIC 连接，ready 返回 200，公开首页经主机代理返回 200。

截至本次排障，固定入口随后返回通用 503；Mac 到主机的新 SSH、原有私有 HTTPS 和实际 IP ping 也超时。此前成功的是默认 DISCO 发现探测，不能写成 TSMP 数据探测成功。进一步核对：管理页两台设备在线、密钥有效、既有规则允许全部设备连接；Mac UDP 与中继可用且无本机健康告警，但显式 `--tsmp` 和 `--peerapi` 探测均超时，主机数据接收为 0，WireGuard 握手未完成。本机 Tailscale 已保留设置重新连接、清除网络映射缓存并 rebind，仍未恢复；Mac 的精确主机代理绕过项已补回。T16 的后续恢复需要主机本机或其他可用管理通道检查 `tailscaled`；当前用户只能操作 Mac，不要再让用户反复从 Mac 新建 SSH 来执行远端恢复。

尚未完成旧网关的稳定重连验收。用户目前只能操作 Mac，已知主机其他地址的 SSH 也均超时；本机日志显示发出连接被允许，但对方数据握手无回应。T09 与 T16 作为旧主机恢复事项保留受阻记录，不再阻挡 T17 新云端实例上线。旧 Mac Quick Tunnel 已停止，旧 VPC 网关仍属于独立的历史入口，不能代替新云端网址。Mac 对 Worker 域名的系统 DNS 与公共 DNS 不同，指定正确公共地址的单次 curl 能到达网关；Python 默认客户端还遇到 Cloudflare 1010，需要与应用的来源 403 区分。详情与复现命令见 [固定 Cloudflare 网址部署手册](cloudflare-workers-vpc-deployment.md)。主机地址、资源清单与私有配置保留在私有部署记录，不写入公开交接文件。日常上课记录已独立于训练计划上线，包含在十一表 Excel 导出中。

数据库迁移版本 `006` 预留给尚未合并的腾讯文档导入工作区，日常上课记录使用 `007` 且不依赖 `006`。后续合并导入时须将 `006` 放在 `007` 前，并验证已运行 `007` 的数据库仍能安全补跑 `006`；不要单独把仅支持版本 6 的旧分支部署到版本 7 数据库。

规划总览的途径页显示明确适用范围和经验数；每次训练实际选择的途径、版本快照和真实使用次数仍待接入。原生 iOS／Android、语音和游戏属于后续阶段。

云环境启动说明曾因旧草案 `stale_base` 未能保存。不要重复提交旧草案；从当前环境设置新建 setup 会话后读取新草案、核对并合并。私有配置导出留在原会话附件／私有备份中，不进入 Git；本地开发可以直接按仓库说明继续，不依赖该旧草案保存成功。

## 产物与证据

[示例截图目录](artifacts/validation/README.md) 随 Git 同步，三张图片为明确的合成验收数据。校验值记录在任务清单中。原始 `.local/` 截图与日志不会自动加入仓库。

历史应用提交 `2749f64` 的 Rust 工作区测试、原生 Clippy、完整 `test:host` 通过；浏览器测试包含独立上课记录、修改和本机草稿刷新恢复。11 项规划统计与 14 项交接测试为此前通过记录。本次完整 Cloudflare 发布另通过前文列出的 Wasm 构建、Clippy 和双存储合约；历史主机仍待网络恢复。历史验收不能自动当作新修改的验收结果。

Git 同步覆盖已提交的文件。聊天记录、模型临时上下文、运行中任务、后台进程、未提交修改、真实业务数据库、`.env` 和私钥不会被此流程复制。新会话通过仓库里的交接文件恢复工作依据。
