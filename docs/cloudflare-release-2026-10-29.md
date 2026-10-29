# 试用反馈修复发布（2026-10-29）

状态：**已上线并通过公开检查**。入口：**https://skarma.vip**。

用户明确要求部署刚完成复验的版本。本次为原应用的普通代码发布，保留全部账号、会话和业务记录。
没有新schema、清库、重复初始化、演示恢复或自动种入参考内容。

## 本次范围

应用实现基线为 `23215ce9dc1a6b7668d8e339e2d8ea67c1b6ddeb`，包括：

- T55：修复课记上传扫描延迟、基础草稿与错误串页，支持保留／放弃本机填写；长期关注分隔，帮助集中到账号，搜索折叠。
- 课时核对可按老师／课程汇总节数，默认老师和上个自然月；不把节数推算为分钟。
- 跟进目标／方法按层级多选，共用方法同步勾选，新建经验／困难仅在库分段显示。
- T56：移除干预更多操作中遗漏的全局帮助入口，并补充当前Web账号全流程检查。
- [图文报告](functional-review-2026-10-29.md)中的五项体验优化仍待用户评审，本次没有顺带实施。

## 发布前核对

应用文件摘要 `28effa0107e3455216e499bc90b8367b2b0c371f418dd404fc8b659bd897ffab` 与T56受测字节一致。
工作区与远端均为23215ce且干净；[普通验证run40859567395](https://github.com/q9good/skarmar/actions/runs/40859567395)成功。
本轮重跑发布保护29项、演示工具保护9项及既有96项产物校验，均通过；公开health200。

T56已实际完成162项Rust、61项客户端、26组Web、两套存储完整合约及重启、类型和Web构建等检查。
57条AC在本机范围通过、2条部分验证；Opus、真机和完整读屏仍待补，不将本次发布称为这些验收通过。
原Cloud环境T12密钥发布故障未变，本次使用既有GitHub Actions Secrets和已验收的云端触发链路，未修改环境草案或取出密钥。

## 实际发布结果

| 项目 | 结果 |
| --- | --- |
| 触发／部署源码 | `87f94d66a30853d3d71fc9ed315046c6cfecd4e7`（含应用23215ce） |
| Actions | [run40885836043](https://github.com/q9good/skarmar/actions/runs/40885836043)，validate与release均success |
| 原版本 | `09d57936-2071-45e8-a4c2-69a46e9645fb` |
| 上传并实际激活 | `266022a9-e23d-4916-bda1-52e646badb41` |
| 模型 | `prd_v1` |
| 回执 | [deployed_and_verified](artifacts/deployment/cloudflare-release-20261029-receipt.json) |

CI实际执行类型检查、Web构建、Rust测试和Wasm Clippy后，核对原应用、上传唯一候选、比较原绑定及配置，再激活100%流量。
没有使用基线初始化或演示恢复标记。发布期间远端分支没有前进；仅在run成功后才提交收尾记录。

## 上线检查

runner和[云端独立公开检查](artifacts/deployment/cloudflare-release-20261029-independent-checks.json)均通过：

- 首页仅排除已知的一条Cloudflare Analytics注入后，与本次构建字节一致；JavaScript逐字节匹配。
- health200/status ok；匿名me/state401，未知API404。
- 无账号Cookie的合法Origin logout200；非法Origin或缺请求标记403。
- 使用从未签发的随机会话检查数据库启动成功，没有登录真实账号或读取业务数据。

原Worker `skarma-web`、`SkarmaSpace/SKARMA_DB/default`、PRD v7结构、skarma.vip和精确来源均保留。
不重建管理员、DO或空间，不清除本机／服务器草稿，不进行线上合成业务写入测试。
用户刷新页面即可查看本版；已有账号继续使用。实际账号和手机体验由用户确认。

本文件、回执和任务快照的收尾提交不带发布标记，不再次部署。
