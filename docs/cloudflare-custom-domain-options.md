# 自有域名访问 Cloudflare：新增资料核对与接入建议

日期：2026-10-19，Asia/Singapore。用户本轮明确倾向自有域名，取代此前不购买域名的偏好；
尚未提供实际域名，未购买、改 DNS、开通计费服务或部署。生产发布继续暂停。

**后续进展：**用户随后购买 `skarma.vip` 并授权接入。实际操作与尚未完成的 DNS / HTTPS /
DNSSEC / 大陆验收见 [域名部署记录](custom-domain-deployment.md)（T26）；下文保留本轮调研时的事实边界。

## 建议

第一步选择 **一个可长期管理的域名 + Cloudflare Free zone + 现有 Worker 的 Custom Domain**。
使用 `app.example.com` 同时承载页面和 `/api/*`，保留原 Rust、DO 和账号数据。
先完成家人真实网络的验证，再决定是否需要额外措施。域名直绑仍经过普通全球网络，尚不等于
大陆访问已解决；没有证据支持先为本项目购买 Pro、Argo 或增加一台 VPS。

libp2p / 家庭持久节点仍是未实施的客户端同步候选。本轮确定的是访问入口偏好，没有自动
取消同步研究或改动业务架构。此前的 [亚洲 VPS 中转方案](cloudflare-mainland-link-review.md)
保留为直绑效果不足时的候选。

## 五个链接对应四篇独立文章

| 链接 | 本次读取与实际主题 | 可以用于本项目的部分 |
| --- | --- | --- |
| [零度博客](https://www.freedidi.com/23618.html) | 通过已有系统 Chromium 的临时未登录会话读到全文，2026-04-02。主题是基于 Pages / KV 的 VLESS、Trojan 等客户端代理节点 | 域名及平台入口可作背景；节点订阅、ProxyIP 与代理客户端并非本应用的浏览器直连 CDN 方案，不部署文章程序 |
| [阿里浏览器链接](https://browser.alibaba-inc.com/?Url=https%3A%2F%2Fplumephp.com%2Fcloudflare-china-cdn%2F) | 链接目标是下方 PlumePHP；本轮外层返回 SSO 页面，未登录，直接阅读原站 | 不作为第二份独立证据 |
| [腾讯云开发者社区](https://cloud.tencent.com/developer/article/2654949) | 通过系统 Chromium 读到全文；平台发布时间 2026-04-14，标注原始发表 2025-09-26。主题是 GitHub Pages 静态博客接入 Cloudflare | 自有域名、NS 与 HTTPS 的思路可用；GitHub CNAME 和四个 Pages IP 不用于当前 Worker 应用 |
| [Cloudflare 实战手册](https://chendahuang.com/playbook/cloudflare/chapters/06-china-access) | 成功读取，正文与上一轮已读取版本的 HTML SHA-256 相同 | 自有域名、减少外部依赖、真实网络拨测；其中 DNS 现象的概括不能当成本项目诊断 |
| [PlumePHP 原站](https://plumephp.com/cloudflare-china-cdn/) | 成功读取全文，标注 2025-11-21。讨论套餐、Argo、缓存、DNS 和国内 CDN 混用 | 静态资源体积与缓存、国内外入口比较可参考；若干功能和套餐说法需按官方现状修正 |

公开页面缓存留在忽略目录 `.local/research/cf-custom-domain-20261019/`，未将全文、网页会话
或缓存提交 Git。没有安装新浏览器 / 手机依赖，没有执行文章中的部署、订阅或代理客户端步骤。

## 影响选择的官方核对

1. **自有域名不需要先升级 Pro。** 现有 Worker 可通过官方
   [Custom Domains](https://developers.cloudflare.com/workers/configuration/routing/custom-domains/)
   接入自己管理的激活 zone，由平台建立相应 DNS 与证书。普通付费计划不是中国网络订阅，
   也不能据此承诺一定调度到香港 / 日本或达到文章里的固定时延。
2. **Auto Minify 已弃用。** [官方弃用表](https://developers.cloudflare.com/fundamentals/api/reference/deprecations/#2024-08-05)
   列明 2024-08-05 弃用；不能照抄新开启该功能的菜单操作。当前 Expo 构建与资源优化应按项目
   构建链处理，不靠恢复一个旧开关。
3. **CNAME flattening 不是大陆线路开关。** [官方说明](https://developers.cloudflare.com/dns/cname-flattening/)
   是将 CNAME 最终地址解析后返回 IP；[配置页](https://developers.cloudflare.com/dns/cname-flattening/set-up-cname-flattening/)
   区分根域名默认 flatten、已代理记录默认 flatten，以及付费 zone 的「flatten 所有 CNAME」。
   不能把所有菜单选项都算作 Free 权益，也不能认为减少查询就获得大陆节点。
4. **外部 DNS 的原生 Partial CNAME setup 有套餐门槛。**
   [官方说明](https://developers.cloudflare.com/dns/zone-setups/partial-setup/)限定 Business / Enterprise。
   PlumePHP 示例中的 `你的域名.cdn.cloudflare.net` 不能未经正式接入就在免费外部 DNS 上照填。
5. **Argo 主要优化回源段。** [官方入门](https://developers.cloudflare.com/argo-smart-routing/get-started/)
   明确优化 Cloudflare 数据中心与源站间路径，需要计费信息；现文档将其列入 Smart Shield。
   这不等于解决手机到边缘的 DNS / TLS / 路由失败。本项目用 Worker + DO，并非文章中的
   海外博客源站；不能直接套用「每月 5 美元即可解决」或性能百分比结论。
6. **静态博客的可达性和缓存不能直接外推到登录应用。** 文章中的全站 Cache Everything、
   Always Online 等不能替代真实 API 连通。保留登录、业务、Excel 的 `no-store`，不因某网络
   首页返回 200 就认定可以正常保存记录。

文章的 Pro / Business / China Network 报价和测速表不是本项目的当前报价或验收数据；
本次不采用「升级 Pro 就足够」「某 DNS 在大陆一律可用 / 不可用」这类统一结论。

## 自有域名实施顺序

以下是后续接入计划，本轮没有执行设置变更。

1. **选定域名及 DNS 管理方式。** 若已有域名先复用；新注册时核对正常续费价格与管理权限，
   不以首年促销或第三方「永久免费」宣传估算长期成本。第一期按普通 full zone 接入 Free，
   保留原有必要的 MX / TXT 等记录；不为外部智能 DNS 先购买 Business。
2. **固定一个应用子域名。** 例如 `app.example.com`。在原 Worker 的
   `Settings → Domains & Routes → Add → Custom Domain` 绑定，等待 zone 与证书生效。
   不指向 GitHub Pages 的 IP，也不手动拼一个跨账号 CNAME 代替平台的域名绑定。
3. **保留数据标识并允许新来源。** 原 Worker、`SKARMA_DB` namespace、`SKARMA_SPACE`
   都保持一致。按 [登录实现](../crates/http/src/auth.rs)先将新 HTTPS 入口加入
   `SKARMA_ADDITIONAL_PUBLIC_URLS`，核对最多四个精确来源的限制；不关闭来源校验。
   [前端 API](../apps/client/src/api.ts)使用同源 `/api`，检查构建没有写死旧 API 域名。
4. **处理 Cookie 和浏览器草稿。** 新域名重新登录；服务端记录仍在原库，旧域名的 Host Cookie
   和浏览器未提交草稿不会自动转移。切换前先处理旧入口草稿。
5. **先用隔离测试实例与合成数据做流程验证。** 核对页面、JS、健康、登录、读取、保存、
   重新打开、注销、Excel。再在家人常用大陆 Wi-Fi / 蜂窝无代理网络对比默认地址与新域名，
   记录 DNS / TLS 错误、首字节 / 总时延和成功率；公共拨测作辅助，不携带账号 Cookie 或业务记录。
6. **验收后再接入生产。** 将已验证的入口配置用于原 Worker，保留回退入口与原数据标识。
   确认后再决定正式主入口和后续界面发布；本文不会自动解除生产暂停要求。

新增基础预算是域名年费；Cloudflare 计算、DO 与资源服务仍按各自额度计量，不因绑定域名
而变成无限免费。第一轮不预购 Pro、Argo、Business 或中转服务器。

## 优选 IP、CNAME 与 SaaS 的边界

本轮提供的文章没有给出已验证适用于本项目的 SaaS 优选接入配置。作为附加核对：

- Cloudflare 使用 [Anycast](https://www.cloudflare.com/learning/cdn/glossary/anycast-network/)。
  一个所谓「香港 IP」不代表各运营商请求都到同一香港节点。优选需要从家人实际网络测试
  域名、SNI、证书与 HTTP，不仅测 Ping；未做此测试，不固定任何 IP。
- 公共优选 CNAME 增加第三方 DNS 依赖，不代表自动获得中国网络，也不等于亚洲 VPS 中转。
  不把它作为未经验证的唯一生产入口，不改变平台管理的 Custom Domain 记录来试运气。
- [Cloudflare for SaaS 当前套餐](https://developers.cloudflare.com/cloudflare-for-platforms/cloudflare-for-saas/plans/)
  在 Free / Pro / Business 包含 100 个 custom hostnames，额外主机名表价 0.10 美元；
  [开通页](https://developers.cloudflare.com/cloudflare-for-platforms/cloudflare-for-saas/start/enable/)
  要求非企业用户提供付款信息。[官方也支持 Worker 作 fallback origin](https://developers.cloudflare.com/cloudflare-for-platforms/cloudflare-for-saas/start/advanced-settings/worker-as-origin/)，
  但须完成 hostname、证书与 Worker route 配置；不是只添加一条 CNAME，也不是大陆加速保证。
  有明确外部 DNS / 域名接入需求时再验证，不作为只有一个家庭应用时的首轮默认配置。

若直绑域名仍有问题，先分清失败发生于解析、连接、TLS 还是应用响应。确有证据后再比较
DNS / IPv6 调整、受控 IP 测试、SaaS 接入或已评估的亚洲 VPS 中转。域名没有把握实际可用之前，
不依据这些文章直接开付费计划或移走现有数据库。

当前已完成资料与项目适配评估；实际域名、证书、DNS 设置、目标网络和端到端业务均未验收。
