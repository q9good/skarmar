# Garden Skills：当前云端工作区安装

2026-11-03已完成项目级文件安装，来源为作者官方仓库
[ConardLi/garden-skills](https://github.com/ConardLi/garden-skills)，固定提交
`2b9a89340b70f632d31c0f1c69fce94a7912423e`。

| 技能 | 用途 |
| --- | --- |
| `web-design-engineer` | 网页设计、界面评审与前端视觉实现 |
| `kb-retriever` | 本地知识库检索 |
| `beautiful-article` | 将素材整理为单文件HTML文章 |
| `gpt-image-2` | 图像生成／编辑的提示词与工具使用指引 |
| `motion-showreel` | 代码动画与视频制作 |
| `web-video-presentation` | 网页演示与可录屏讲解 |

规范目录：`/workspace/worktrees/skarmar-web/.agents/skills/<技能名>/SKILL.md`。
项目内 `.codex/skills/<技能名>` 也已链接到对应目录，兼容作者提供的Codex目录约定。
调用示例：“用web-design-engineer评审当前界面”。执行任务前读取对应SKILL.md及所需引用；
用户已确认的需求、RN／Expo技术栈和正式数据保留要求优先。

安装采用作者支持的手动复制方式：六份技能元数据解析通过，417个上游文件逐项SHA-256一致，
每份技能附带MIT许可，六个兼容链接均能解析。详细安装清单在本环境忽略目录
`.local/garden-skills-installation.json`；原始源码在 `.local/skill-sources/garden-skills/`。

这属于当前云端工作区的文件安装，当前会话的executor技能列表仍为空，不代表已注册为平台插件，
也不声称技能菜单已刷新。当前agent可直接读取文件使用；重新打开项目会话时是否自动发现由宿主决定。
技能文件通过本机Git exclude排除，不随应用提交或普通pull同步，其他设备／环境需分别安装。

本次只安装技能和资源文件，没有安装生成脚本的运行依赖或执行出图／TTS等外部调用；
需要这些能力时再按实际任务准备。没有改动应用代码、生产服务、环境草案或凭据。
