# 取消密码最小长度与线上发布准备

日期：2026-10-20。用户要求取消注册的12位密码限制，并将最新功能部署到现有线上应用。

## 已完成的代码

密码不再设置最小长度，仍需非空且不能全部为空白，保留原128字节上限。注册、管理员创建／重置、自行修改密码及本机管理员命令共用此规则。页面和终端提示同步更新，已有账号密码哈希及数据不修改，审批、限流、会话撤销和来源检查继续保留。

本轮没有增加数据库迁移。待发布的完整版本包含此前T27六页流程、T30管理员删除／成员训练修订，以及这次密码调整；不是在旧正式版本上只替换一个页面。

## 本轮验收

- 72项Rust测试通过：包含6位密码注册审批后登录、1个汉字密码注册审批后登录、空白／超长密码拒绝，以及原有认证、导入、关联与事务测试。
- 原生及Wasm Clippy、TypeScript、Web构建、Rust Wasm构建通过。
- `npm run test:runtimes`通过：原生SQLite与真实Rust Worker/DO验证1位密码注册、短密码创建账号／重置／自行改密、空白与超长拒绝、审批及Cookie／权限／会话撤销；既有事务、删除修订、导入和Excel合约通过。
- `npm run test:host`通过：手机尺寸页面填写1位密码申请注册，管理员审批后实际登录；原页面、多人冲突、本机草稿、Excel及备份恢复回归通过。
- `npm run cf:check`通过，只执行根目录开发配置的dry-run，未上传。输出代码约4496.18 KiB、gzip约1500.70 KiB；它不验证缺失的实际生产配置，也不等于已发布。

以上业务测试均使用隔离的临时合成数据库，未创建线上测试账号或更改线上记录。

## 发布尚未执行：凭据与生产配置缺失

运行环境状态没有Cloudflare凭据；默认和既有`XDG_CONFIG_HOME`缓存两处Wrangler `whoami`均显示未登录。实际`deploy/cloud-worker/wrangler.toml`未在云端，只有不含实际账号和来源的模板。GitHub可访问不能代替Cloudflare授权，不能使用临时预览账号替代现有线上实例。

本次尝试仅添加`CLOUDFLARE_API_TOKEN`需求，配置工具返回`CONFLICT / stale_base`，没有确认保存或应用。没有重交旧草案或修改安装、启动、仓库及网络配置。完整新增需求保存在：

- [需求JSON](environment/cloudflare-publish-requirements.json)
- [新setup会话接续说明](environment/cloudflare-publish-requirements.md)
- [可转移ZIP包](artifacts/deployment/cloudflare-publish-requirements.zip)，仅含上述两份无密钥值的文件

按配置工具要求，从当前环境设置打开新的setup会话，读取新草案并合并该需求，在安全字段填写Token并应用；不要在聊天或Git填写密钥。随后先验证实际Cloudflare授权，取得并核对现有生产配置与备份／恢复点，再发布。若继续使用Mac已有登录，则在Mac安全拉取共享分支并由当地发布环境接续，不能在云端声称Mac已同步或已部署。

用户的上线授权已明确，取得所需凭据后可继续，不再重复请求发布授权。发布必须保留`skarma-web`、`SkarmaSpace`、`SKARMA_DB`、`SKARMA_SPACE=default`、现有公开来源和`skarma.vip` Custom Domain；按[域名配置](custom-domain-deployment.md)确认私密配置已包含新域及允许来源。禁止用带占位符模板直接发布、重建数据库或管理员。

本次从云环境访问`https://skarma.vip/`及健康／匿名身份接口得到HTTP403、Cloudflare错误1010，不能据此判断用户设备上的站点状态，也未完成线上健康验收。未绕过代理、未更改站点防护；匿名结果保留于忽略目录`.local/deployment-20261020/`。

待凭据就绪后，在受控备份和配置核对之后发布，再核对首页／JS版本、健康、匿名401、精确来源及既有账号和记录保留。当前没有生产发布版本ID，不能将本轮构建或dry-run当作上线结果。
