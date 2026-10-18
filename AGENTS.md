# 跨环境协作

开始工作时先查看 `git status`、当前分支及本地修改，然后阅读 `docs/handoff.md` 和
`docs/workspace-state.json`。状态文件是最后一次交接快照，不表示其他设备已拉取，也不表示
后续代码自动通过了旧验收。设计和统计口径以其中链接的文档为准。

用户已授权本项目使用 `feat/tencent-informed-web` 分支实现功能并推送 GitHub；沿用已有
checkout / worktree，新增 worktree 需用户明确要求。不要自动切换分支、stash、reset、
强制推送或覆盖另一环境的未提交内容。

完成或交接工作时，更新 `docs/workspace-state.json` 中相关任务、阻塞原因与验证证据；
必要时补充 `docs/handoff.md`。新任务使用稳定且不重复的 ID。记录实际完成的检查，区分
历史通过、当前通过、未执行与受阻。`implementationCommit` 指最后一次应用验收对应提交，
交接文档自身的提交不需要改写它。

将已确认仅含合成数据的必要截图放入 `docs/artifacts/validation/`，更新清单中的 SHA-256
后与代码一起提交。未经检查的截图、日志、真实训练数据、SQLite、凭证和环境配置私密
导出不得自动纳入 Git；`.local/`、数据库卷和本机缓存按现有规则留在各环境。

`npm run sync:status` 查看本地交接状态并校验产物；`npm run sync:pull` 在工作区干净且
分支匹配时获取远端并快进。按用户任务处理和提交本地工作，再在另一环境拉取；不要
把“修改状态文件”当成完成实现，也不要声称聊天记录或运行中任务已双向同步。
