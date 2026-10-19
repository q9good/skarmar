# skarma.vip 接入现有 Cloudflare 应用

日期：2026-10-19。最近核对时间：20:22（Asia/Singapore）。本文记录本次实际操作与尚待验证的项目。

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
| Cloudflare zone | 已建立免费 zone，当前为 Pending | 等待权威 NS 委派生效并显示 Active |
| 原权威 NS | `launch1.spaceship.net`、`launch2.spaceship.net` | 作为变更记录保留，不与新 NS 混用 |
| 新权威 NS | Spaceship 已确认改为 `coleman.ns.cloudflare.com`、`harleigh.ns.cloudflare.com`；注册局 RDAP 也返回这两条 NS | Spaceship 显示 IN PROPAGATION，Google / Cloudflare 公共递归仍返回旧 NS，尚不能认定全网生效 |
| DNSSEC | 旧 Spaceship DNSSEC 已关闭，注册局 RDAP 为 delegationSigned=false；Google 公共 DS 查询无旧 DS | Cloudflare 签名已开启，等待向注册商添加新 DS；待 zone Active 与旧 NS TTL 窗口结束后提交，尚未恢复验证链 |
| 停车记录 | 扫描导入两条根域 A：`54.149.79.189`、`34.216.117.25` | 用户确认后已删除；列表核对为 0 条，再由 Worker 创建接管记录 |
| 应用允许来源 | 已在原 Worker 添加 `SKARMA_ADDITIONAL_PUBLIC_URLS=https://skarma.vip` 并发布配置 | 经旧入口带新域 Origin 的匿名 logout=200；非法来源仍403。新域实际登录待 DNS 生效 |
| 现有存储 | 保留 `SKARMA_DB` 绑定和 `SKARMA_SPACE=default` | 不新建数据库、不重新初始化管理员 |
| 根域 Custom Domain | 原 Worker Domains 页已显示 `skarma.vip` Production，zone 概览显示 Connected Workers → skarma-web | 平台绑定已保存；等激活与证书后核验实际访问 |
| TLS | 尚未确认新域证书签发和实际 HTTPS 握手 | 不用忽略证书检查来代替验收 |
| 大陆访问 | 尚未执行真实大陆无代理验收 | 家人 Wi-Fi 与蜂窝网络分别验证 |

此表是实操进行中的快照，不能将 Pending、配置已保存或代理下的 HTTP 200 写成全面上线成功。

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

本次已请求一次激活检查；注册局 RDAP 已显示新 NS，但公共递归仍缓存旧 NS，原 NS TTL 约 1800 秒。
不要把本机 `dig @指定服务器` 的结果单独当作权威证据：本机代理 / TUN 可能截获 DNS；本次
加密公共 DNS 查询与注册局 RDAP 采用独立通道交叉核对。等待 Cloudflare zone 显示 Active，
并按旧 NS TTL 窗口核对代表性递归已采用新 NS。
不要反复修改 NS 来催促传播。后续恢复 DNSSEC 时：

1. 在 Cloudflare 的 DNSSEC 设置中启用签名。
2. 将 Cloudflare 提供的新 DS 参数添加到 Spaceship 的 DNSSEC 配置。
3. 确认注册商保存成功，并验证 DS / DNSKEY 链与实际 DNS 回答一致。

Cloudflare 已启用签名并提供新 DS，尚未向 Spaceship 提交。不要恢复旧 Spaceship DS。
根据官方迁移顺序，先等待旧委派缓存退场，再提交新 DS；否则可能出现新 DS 与旧权威密钥不匹配。
最终应记录新链是否已验证；目前尚未完成。

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
| 权威 NS | 父区委派使用本次两条 Cloudflare NS，Cloudflare zone Active | RDAP 已为新 NS；zone 仍 Pending，代表递归仍为旧 NS |
| 根域绑定 | 原 Worker 显示 `skarma.vip` Custom Domain | Production 列表及 Connected Workers 均已确认 |
| TLS | `https://skarma.vip/` 正常验证证书且无需忽略警告 | 待补 |
| 首页与资源 | 首页与对应 JS / 静态资源正常加载 | 待补 |
| API 健康 | 使用项目现有健康检查入口返回正常结果 | 待补 |
| 匿名会话 | `GET /api/auth/me` 返回预期的未登录 401，而非 500 | 待补 |
| 来源检查 | 新域合法来源请求进入认证流程；非法来源仍返回 403 | 经旧入口的无会话 logout：新域 Origin=200，非法 Origin=403；原入口 health=200 |
| 实际登录 | 用户在新域亲自使用现有账号登录成功 | 待补 |
| 数据保存 | 经用户授权的记录可保存、刷新后仍在，旧服务器记录保持 | 待补 |
| DNSSEC | 新 DS / DNSKEY 链匹配，验证型解析无失败 | 旧 DS 已撤；新 Cloudflare DS 尚未发布，等待激活与缓存窗口 |
| 大陆 Wi-Fi | 家人关闭代理 / VPN 后可打开、登录、读取和保存 | 待补 |
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
