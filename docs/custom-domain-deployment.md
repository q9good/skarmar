# skarma.vip 接入现有 Cloudflare 应用

日期：2026-10-19。最近核对时间：20:41（Asia/Singapore）。本文记录本次实际操作与尚待验证的项目。

## 目标与边界

将在 Spaceship 注册的 `skarma.vip` 接入 Cloudflare 免费 DNS zone，并将根域直接绑定到
现有 `skarma-web` Worker，继续使用原来的账号和 SQLite Durable Object 数据。
本轮不依赖旧家庭电脑、Tailscale 或 Tunnel，也不购买额外服务器或付费加速套餐。

代码已从 Codex Cloud 安全同步到本地 `640df50`。这表示开发材料已同步，不表示该提交已发布。
本轮只调整域名、DNS 与允许的 HTTPS 来源；通过控制台保存变量会创建配置部署，
但没有上传新版应用代码。“今天 → 长期关注置顶”等尚未发布的应用改动仍须另行确认发布。

## 本次实操状态

| 项目 | 当前已确认的事实 | 后续核对 |
| --- | --- | --- |
| 域名注册 | 用户已在 Spaceship 购买 `skarma.vip` | 保持注册和续费由 Spaceship 管理，无需转移注册商 |
| Cloudflare zone | 免费 zone 已激活，概览显示域名已受 Cloudflare 保护 | 保持 Free 计划 |
| 原权威 NS | `launch1.spaceship.net`、`launch2.spaceship.net` | 作为变更记录保留，不与新 NS 混用 |
| 新权威 NS | Spaceship 已确认改为 `coleman.ns.cloudflare.com`、`harleigh.ns.cloudflare.com`；注册局 RDAP 也返回这两条 NS | Google 与 Cloudflare 公共递归均已返回新 NS；注册商的传播提示可能滞后 |
| DNSSEC | 旧 DS 已撤；zone 激活、两家递归采用新 NS 后，已将 Cloudflare 新 DS 保存到 Spaceship。注册局 RDAP 已为 delegationSigned=true，DS 与当前 DNSKEY 计算结果一致 | 20:41 Google 与 Cloudflare 的 DS / A 均 Status=0、AD=true，新签名链验证通过 |
| 停车记录 | 扫描导入两条根域 A：`54.149.79.189`、`34.216.117.25` | 用户确认后已删除；列表核对为 0 条，再由 Worker 创建接管记录 |
| 应用允许来源 | 已在原 Worker 添加 `SKARMA_ADDITIONAL_PUBLIC_URLS=https://skarma.vip` 并发布配置 | 旧入口及新域均验证合法 Origin logout=200、非法来源=403；用户已确认现有账号正常登录 |
| 现有存储 | 保留 `SKARMA_DB` 绑定和 `SKARMA_SPACE=default` | 不新建数据库、不重新初始化管理员 |
| 根域 Custom Domain | 原 Worker Domains 页已显示 `skarma.vip` Production，zone 概览显示 Connected Workers → skarma-web | 平台绑定已保存，实际 HTTPS、网页和 API 已通过 |
| TLS | 新域正常验证 HTTPS，浏览器显示完整登录页，健康与 JS=200；证书列表显示对应证书 Active | 本轮没有忽略证书校验 |
| 大陆访问 | 用户明确回复“已正常登录，手机也能直连” | 未分别登记 Wi-Fi / 蜂窝运营商和新记录保存刷新结果，保留 T20 进一步验收 |

域名入口已实际可用且得到用户确认；DNSSEC 公共验证也已通过。分运营商和保存刷新验收仍在 T20 跟进。

## 操作步骤

### 1. 在 Cloudflare 添加域名

在承载现有 Worker 的 Cloudflare 账号中添加 `skarma.vip`，采用免费 DNS zone。
核对扫描导入的记录；如已有邮件或其他业务，保留其 MX、TXT 等必要记录。
本次已识别的两个根域 A 地址属于原停车配置，用户已确认删除。

不要将根域简单 CNAME 到 `workers.dev`。本应用由 Worker 直接提供服务，应通过
Workers 的 Custom Domain 入口绑定，让 Cloudflare 管理对应 DNS 和证书。

### 2. 在 Spaceship 修改权威 NS

在 Spaceship 的 Advanced DNS 中选择 `skarma.vip`：

1. Nameservers → Change → Custom nameservers。
2. 填入此 zone 实际分配的 `coleman.ns.cloudflare.com` 和 `harleigh.ns.cloudflare.com`。
3. 删除原权威 NS，只保留上述两项，保存设置。

本次已完成保存。后续 DNS 记录由 Cloudflare 管理；Spaceship 原记录即使仍显示为
Inactive，也不会继续决定线上解析。Spaceship 的传播提示可能持续一段时间，官方说明最久可达 48 小时。

### 3. 处理 DNSSEC 与激活

本次在切换 NS 前已关闭原 Spaceship DNSSEC，确认旧 DS 为 0。
旧 DS 不能随新 NS 一起原样保留，否则验证链可能不匹配而导致解析失败。

本次已请求激活检查，随后概览确认 Active，Google / Cloudflare 公共递归均采用新 NS。初期旧 NS TTL 约 1800 秒。
不要把本机 `dig @指定服务器` 的结果单独当作权威证据：本机代理 / TUN 可能截获 DNS；本次
加密公共 DNS 查询与注册局 RDAP 采用独立通道交叉核对。等待 Cloudflare zone 显示 Active，
并按旧 NS TTL 窗口核对代表性递归已采用新 NS。
不要反复修改 NS 来催促传播。后续恢复 DNSSEC 时：

1. 在 Cloudflare 的 DNSSEC 设置中启用签名。
2. 将 Cloudflare 提供的新 DS 参数添加到 Spaceship 的 DNSSEC 配置。
3. 确认注册商保存成功，并验证 DS / DNSKEY 链与实际 DNS 回答一致。

Cloudflare 已启用签名，并在 zone Active、代表递归均采用新 NS 后将其新 DS 保存到 Spaceship。
注册局 RDAP 随后返回 delegationSigned=true；新 DS 与 Cloudflare 当前 DNSKEY 计算出的 SHA-256 完全匹配。
初期公共递归曾缓存“无 DS”回答（TTL 900 秒）；20:41 再次核对，Google 与 Cloudflare
均返回正确新 DS，DS 与 A 的 Status=0、AD=true，签名链验证通过。平台状态若显示旧 Pending，
可刷新核对，不需重填 DS 或反复关闭签名。

### 4. 添加根域 Custom Domain

在 Cloudflare 处理停车记录后：

1. Workers & Pages → `skarma-web`。
2. Settings → Domains & Routes（界面也可能单独提供 Domains 页）。
3. Add → Custom Domain，输入 `skarma.vip`。
4. 确认绑定到原 Worker，等待 DNS 和证书就绪。

本次控制台允许在 zone Pending 时预先完成绑定；已看到 Production 域名列表和 Connected Workers
均包含 `skarma.vip`。这与 DNS / TLS 已生效是两个状态。

Custom Domain 的匹配是精确主机名：根域不会自动包含 `www.skarma.vip`。
本轮先验收根域；若后续需要 `www`，再配置其代理 DNS 与跳转到根域的规则。

Cloudflare 会为 Custom Domain 创建 DNS 记录并签发证书；无需购买独立证书套餐。
若存在同名 CNAME，须先明确其用途并处理冲突，不能直接叠加。

### 5. 同步应用允许来源

现有应用对非只读请求执行精确来源检查，见 `crates/http/src/auth.rs`。
本次保留原 `SKARMA_PUBLIC_URL`，在同一 Worker 添加：

```text
SKARMA_ADDITIONAL_PUBLIC_URLS=https://skarma.vip
```

该项已通过控制台 Add variable and deploy 保存，不需要为此重新上传应用构建。
若原来已有额外来源，应保留所需值，以逗号分隔；当前实现最多接受四个精确 HTTPS 来源。
不要使用 `*`、放宽生产认证或打开示例数据。

后续正式以根域为主入口时，可将 `SKARMA_PUBLIC_URL` 改为 `https://skarma.vip`，
并按需要将旧入口保留在额外来源中。此项不是本次绑定成功的前置要求。
本机忽略的真实 `deploy/cloud-worker/wrangler.toml` 已同步增加额外来源和
`routes = [{ pattern = "skarma.vip", custom_domain = true }]`。云端不包含此私密文件；以后发布前
须在当地配置补入对应条目，保留原绑定与空间，避免清掉控制台设置。真实配置仍不进入 Git。

## 账号、会话、数据与草稿

- 根域和旧 `workers.dev` 绑定同一个 Worker，且 `SKARMA_DB`、`SKARMA_SPACE=default`
  不变时，会继续访问同一份服务器数据。不得为域名绑定重建管理员、清库或切换存储空间。
- 会话 Cookie 为 `__Host-skarma_session`，设置 `Secure`、`HttpOnly`、`SameSite=Strict`
  和 `Path=/`，没有跨域 Domain 属性。旧域的登录态不会自动带到新域，用户须在新域重新登录。
- 浏览器草稿存储按 origin 隔离。旧域未提交的本机草稿不会随域名绑定转到新域；应先回旧域
  将需要的草稿保存为服务器记录，再换入口。不要把未保存草稿丢失误判为数据库迁移失败。
- 新域页面与 API 应同源访问；不要把前端 API 单独指回旧 `workers.dev` 来绕过来源检查。

## 验收记录与判定

以下项目当前尚未全部完成。完成一项再登记结果，不以 DNS 已保存代替实际访问验证。

| 检查 | 通过条件 | 实际结果 |
| --- | --- | --- |
| 权威 NS | 父区委派使用本次两条 Cloudflare NS，Cloudflare zone Active | zone 已 Active；RDAP、Google 与 Cloudflare 递归均采用新 NS |
| 根域绑定 | 原 Worker 显示 `skarma.vip` Custom Domain | Production 列表及 Connected Workers 均已确认 |
| TLS | `https://skarma.vip/` 正常验证证书且无需忽略警告 | 已通过，浏览器和 curl 均未忽略证书检查 |
| 首页与资源 | 首页与对应 JS / 静态资源正常加载 | 登录页完整显示，JS=200；新旧入口使用相同 index-cbfcff231bc6ee67627c8dda03397c3a.js，应用版本保持 |
| API 健康 | GET /api/health 返回正常结果 | 新旧入口均200，内容 status=ok |
| 匿名会话 | `GET /api/auth/me` 返回预期的未登录 401，而非 500 | 新域401，提示请登录后继续 |
| 来源检查 | 新域合法来源请求进入认证流程；非法来源仍返回 403 | 新旧入口均已核对：新域 Origin logout=200，非法 Origin=403 |
| 实际登录 | 用户在新域亲自使用现有账号登录成功 | 用户已明确确认正常登录 |
| 数据保存 | 经用户授权的记录可保存、刷新后仍在，旧服务器记录保持 | 待补 |
| DNSSEC | 新 DS / DNSKEY 链匹配，验证型解析无失败 | 新 DS 已保存；注册局登记与 DNSKEY 摘要匹配，两家公共 DNS 的 DS / A 均 AD=true，已通过 |
| 大陆 Wi-Fi | 家人关闭代理 / VPN 后可打开、登录、读取和保存 | 用户已确认手机直连与登录；网络类型、保存刷新尚未逐项登记 |
| 大陆蜂窝 | 另一种运营商 / 蜂窝网络下同样完成端到端操作 | 待补 |

无需取得用户密码即可先检查来源配置：向 `/api/auth/login` 发送不存在的测试账号，
带 `Origin: https://skarma.vip`、`x-skarma-request: 1` 和正确 JSON 类型，
应得到认证失败的 401，而不是“请求来源不匹配”的 403。
浏览器请求的 `Sec-Fetch-Site` 也不能是 `cross-site`。实际账号密码由用户在页面输入。

**大陆访问判定：**自有域名提供可管理的入口，普通 Cloudflare Anycast 仍不保证
所有大陆网络稳定可达。Mac 上经代理、TUN 或境外网络的成功不能代替家人的实际验证。
Cloudflare China Network 是独立接入方案，不能将普通免费 zone 视为已经使用大陆节点。
若真实验收不理想，再按 [大陆访问方案](mainland-access-plan.md) 与
[自有域名方案核对](cloudflare-custom-domain-options.md) 评估后续入口；本轮不预购其他资源。

## 回滚与后续维护

优先保留旧 `workers.dev` 入口，以便新域故障时仍可使用原入口；它自己的可达性仍取决于网络。
域名绑定失败不需要回滚数据库。回滚只处理本轮新增的域名和来源设置：

1. 若只需撤销新入口，在原 Worker 删除 `skarma.vip` Custom Domain，并移除本轮新增的额外来源；
   保留原 `SKARMA_PUBLIC_URL`、Worker、存储绑定和 `default` 空间。
2. 若已将根域改为主来源，先恢复原主来源及所需额外来源，再撤销域名绑定。
3. 若必须撤销整个 DNS 迁移，先确认 Spaceship 原 DNS 记录能满足需要，再将权威 NS 恢复为
   `launch1.spaceship.net`、`launch2.spaceship.net`。原停车记录并不能提供本应用服务。
4. 若已经启用 Cloudflare DNSSEC，恢复其他 NS 前先正确移除其 DS 并处理缓存传播，
   再按目标 DNS 服务重新建立签名链；不能把不匹配的新旧 DS 混用。

通过控制台加入的域名和变量应写入后续维护依据，防止下一次从本地发布时遗漏。
后续提交文档与配置模板时，不纳入实际凭证、账号标识、真实数据库或私密运维导出。

## 官方参考

- [Spaceship：切换到第三方 nameservers](https://www.spaceship.com/knowledgebase/connect-domain-custom-nameservers/)
- [Cloudflare：完整 DNS 接入与 nameservers 更换](https://developers.cloudflare.com/dns/zone-setups/full-setup/setup/)
- [Cloudflare：DNS 接入故障排查与旧 DS 处理](https://developers.cloudflare.com/dns/zone-setups/full-setup/troubleshooting/)
- [Cloudflare Workers：Custom Domains、自动 DNS / 证书与精确主机名](https://developers.cloudflare.com/workers/configuration/routing/custom-domains/)
- [Cloudflare：DNSSEC](https://developers.cloudflare.com/dns/dnssec/)
- [Cloudflare：China Network](https://developers.cloudflare.com/china-network/)
