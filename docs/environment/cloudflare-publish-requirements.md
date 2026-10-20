# Cloudflare发布凭据需求交接

这里只保存凭据需求，不含Token或任何真实生产配置。2026-10-20已只读检查默认和既有XDG缓存目录，Wrangler均未登录；运行环境元数据也未绑定Cloudflare凭据。

当前配置草案的添加请求返回CONFLICT / stale_base，未确认保存，更没有应用。不要反复读取、重交同一个旧草案。配置工具明确要求从当前环境设置打开新的setup会话，读取新草案后再合并本JSON中的secret需求；其余仓库、脚本、网络和已有凭据保持原样。JSON只表达完整的新增需求，不能当作完整配置覆盖原草案。

在新环境设置安全填写CLOUDFLARE_API_TOKEN，目标仅api.cloudflare.com，并按界面保存、应用／发布。Token须能读取和更新现有账号的skarma-web Worker及相关绑定／路由。不要在聊天、Git或此文件填写Token值；不需要原管理员密码，不初始化新管理员，不创建临时预览账号。

继续任务时先核对q9good/skarmar的feat/tencent-informed-web分支、AGENTS.md、docs/handoff.md及workspace-state；保留工作区改动。用Wrangler whoami和环境状态确认实际凭据就绪，不能把保存需求当作已经获得凭据。

取得访问后先读取现有生产配置并核对备份／恢复点，保留skarma-web、SkarmaSpace、SKARMA_DB、default空间、原公开来源及skarma.vip Custom Domain。实际私密Wrangler配置仍在Mac，不随Git；不能用占位模板直接发布，也不能重建数据库或管理员。最终发布授权已由用户给出，无需再次询问是否允许部署。

同包内JSON、此说明及Git中的应用代码可转移到新setup会话；包不依赖本工作区的运行进程。此次没有更改环境设置或生产版本。
