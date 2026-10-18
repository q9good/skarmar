# 提供默认 HTTPS 地址的完整应用托管候选

核对日期：2026-10-19。主要用户在中国大陆；本轮只核对能力，不购买资源、不发布或迁移生产数据。

## 结论

自有域名不是运行完整应用的必需条件。平台提供的子域名可以作为 HTTPS 入口；
“平台支持托管”与“大陆真实用户能稳定访问”仍须分别验收。

本项目已有 `deploy/host/Dockerfile`，可评估原生 Rust / Axum + SQLite + 构建后的网页
一并运行。以下“可承载”是基于容器与持久卷能力的技术判断，尚未在这些平台实际部署验证。
Cloudflare Durable Object 中的新账号和数据不会随代码部署自动搬到其他平台。

## 候选对比

| 平台 | 平台地址与 HTTPS | 完整应用及持久化 | 本项目需确认的条件 |
| --- | --- | --- | --- |
| Sealos 应用管理 | 平台应用域名；官方模板使用分配的 `app_host` 与平台域名，并配置 TLS；Docker 部署文档说明 HTTPS 自动签发证书 | Docker 长期运行服务、环境配置、持久存储卷 | 优先检查目标可用区及默认域名对正式应用的开放条件、实名或审核要求，再用大陆网络测试；当前公开文档未充分确定所有区域的默认域名长期使用限制 |
| Railway | Railway 提供的 `.railway.app` 域名及自动 SSL | 服务运行与持久卷，可给 SQLite 配置固定数据目录 | 需要部署并核对账户配额、持续在线费用及实际大陆访问；未确认大陆可达 |
| Render | 每个 Web Service 提供 `*.onrender.com`，自动 HTTPS / TLS | Docker 或原生 Web Service；付费服务可挂持久磁盘 | 本项目保存 SQLite 需付费服务与持久磁盘，不适合用临时文件系统存放正式数据；可选新加坡，仍须测试大陆网络 |
| Zeabur | 可生成 `*.zeabur.app` 子域名；提供 HTTPS 接入 | Git / Docker 服务与持久存储卷 | 官方已说明共享集群在 2026 年停止服务，新项目需购买或接入服务器；不能按旧的免费共享托管印象估算成本；默认域名大陆可达性未验证 |

以上平台均未发现能替代真实网络验收的“所有大陆网络稳定可用”承诺。默认地址免费
不代表运行资源、磁盘、流量或持续在线免费。具体费用在确定区域、资源和账户条件后再核算。

## 建议的下一步

先检查 Sealos 控制台实际可选区域及应用默认域名规则。满足正式使用条件后，
以不含业务数据的最小应用测试首页、HTTPS 和健康接口，随后再演练登录、保存及重启数据保留。
Railway、Render 作为其他候选。此顺序是本项目的筛选建议，不是大陆可用性保证。

持久 SQLite 的初期部署采用单实例，并把数据库、WAL 和关联文件放在同一持久目录。
切换平台前须另行完成 Cloudflare 数据导出、迁移和恢复演练；当前没有自动整库迁移能力。
无代理验收步骤沿用 [大陆访问评估](mainland-access-plan.md)。

## 官方依据

- Sealos：[应用管理](https://sealos.run/docs/guides/app-management)、[Docker 部署与自动证书](https://sealos.run/zh/docs/getting-started/deploy-docker-image)、[分配域名及 TLS 的模板结构](https://sealos.run/docs/guides/appstore/code-templets)、[可用区与工作空间](https://sealos.run/docs/guides/account-workspace)。
- Railway：[平台域名与自动 SSL](https://docs.railway.com/networking/public-networking)、[持久卷](https://docs.railway.com/volumes/reference)。
- Render：[Web Service、默认域名及 TLS](https://render.com/docs/web-services)、[付费服务持久磁盘](https://render.com/docs/disks)、[区域](https://render.com/docs/regions)。
- Zeabur：[平台域名](https://zeabur.com/docs/zh-CN/deploy/networking/public-networking)、[快速开始](https://zeabur.com/docs/zh-CN/get-started/quick-start)、[持久存储](https://zeabur.com/docs/zh-CN/operations/data/volumes)、[共享集群停止服务](https://zeabur.com/docs/zh-CN/server/shared-cluster)。
