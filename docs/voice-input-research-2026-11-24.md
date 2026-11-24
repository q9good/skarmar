# 手机中文语音录入调研

| 编号 | 本轮调研要回答的问题 |
|---|---|
| T88-1 | 现有网页中，iPhone 和国内安卓用户如何把语音输入指定字段？ |
| T88-2 | 网页自带话筒、服务端转写和后续 Expo App 各需要什么，成本和数据流向有何区别？ |
| T88-3 | 哪些结论有官方文档支持，哪些必须用家庭实际手机验收？ |

## 建议先从家人已经在用的键盘开始

外婆在课后说一句“换成两张图后，他自己找到了小猫”，文字应当直接出现在她点的那一条课堂观察里。她能改字、再说一句，最后自己决定是否保存。父母填写家庭总结的实际表现、困难或经验时，沿用同一种操作。老师、课程和关联目标仍用原有选择器，以免把语音里的同音词关联到错误的记录。

对第一期手机网页，建议先在每份长文本表单的第一处录入位置给一次提示：“点输入框，用键盘上的话筒说，说完可以改字。”在“我的账号 → 使用说明”放苹果和安卓的图示。先请家里常记课的人用现有输入法试录，若找不到话筒、切换步骤多，或经常要改关键字，再投入开发站内话筒。

### T88-1 现在如何用

| 手机 | 操作 | 条件 |
|---|---|---|
| iPhone | “设置 → 通用 → 键盘 → 启用听写”。回到 skarma.vip，点“实际表现”或某一条课堂观察，切到中文键盘，点键盘上的听写话筒。说完检查文字，再按原流程保存。 | 苹果的设备端听写清单列有“中国大陆普通话”。机型、系统、键盘语言与地区会影响可用性，是否在本机处理可查看这台手机的键盘隐私说明。[iPhone 听写指南](https://support.apple.com/zh-cn/guide/iphone/iph2c0651d2/ios) · [支持语言](https://www.apple.com.cn/ios/feature-availability/) · [听写隐私](https://www.apple.com/legal/privacy/data/en/ask-siri-dictation/) |
| 安卓 | 在网页点同一个输入框，打开当前中文输入法的语音入口。Gboard 的入口在键盘顶部；国内手机可先看已有输入法的工具栏和语音设置，麦克风权限由具体输入法申请。 | 百度、讯飞等均提供中文语音输入，手机预装的厂商版本可能有不同入口。家人现有输入法能用即可，无须为 skarmar 安装 Gboard。部分输入法还提供“润色”，记老师原话和实际观察时宜选逐字转写，并核对否定词、数字和人名。[Gboard 操作](https://support.google.com/gboard/answer/2781851?hl=zh-Hans&co=GENIE.Platform%3DAndroid) · [百度输入法](https://srf.baidu.com/default/) · [讯飞安卓版产品说明](https://app.mi.com/details?id=com.iflytek.inputmethod) |

这种路径里，键盘把文字送入当前字段，skarmar 沿用已有的本机草稿和正式提交。网站不申请麦克风权限、不接收原始音频，也没有新增转写服务费。输入法是否上传语音、断网后能否听写，由输入法、语言包和设置决定。网页不能用自定义按钮可靠地替用户启动系统键盘的听写。

**现有实现有一个明确缺口。** 课堂观察、家庭总结和跟进的叙述字段使用 `GuidedField`，它将 RN `TextInput` 的 `onChangeText` 接回原表单。字段矩阵已有“输入法语音提示常显”这条设计备注，`guidanceFor` 和 `GuidedField` 并没有把它显示给家人。应给这些表单加面向用户的短提示，无需展示整份内部设计备注。PRD 已允许输入法话筒，自建录音和识别在原实施批次之外。[PRD](prd-family-intervention.md) · [字段指引](../apps/client/src/field-guidance.ts) · [输入控件](../apps/client/src/guided-field.tsx)

### T88-2 如果网页必须有自己的话筒

| 方案 | 适用条件与取舍 |
|---|---|
| 浏览器直接识别 | Web Speech 的 `SpeechRecognition` 可做可选增强。MDN 仍标为支持有限，部分浏览器默认把音频交给远程引擎。WebKit 引入时说明 Safari 使用 Siri 的识别引擎并依赖 Siri 设置。检测到接口也不能证明中文服务可用；国内安卓浏览器、微信内置网页和主屏幕网页要分开试。`processLocally` 与语言包安装能力需单独探测。家庭两端的统一入口不宜押在这个接口上。[MDN 接口](https://developer.mozilla.org/en-US/docs/Web/API/SpeechRecognition) · [WebKit 说明](https://webkit.org/blog/11648/new-webkit-features-in-safari-14-1/) |
| 网页录短句，服务端识别 | 如需在同一位置提供“说一句”，建议先做这个原型。用户点开后才通过 HTTPS 请求麦克风，显示正在录音及停止、取消。每次建议上限 30 秒，只把本段送去识别；返回文字后由用户插入当前字段，再按原流程保存。先检查设备支持的录音格式与服务商接受格式的交集。安卓可能产出 WebM/Opus，iOS 可能产出 MP4/AAC；腾讯列出 Ogg/Opus、M4A、WAV 等，不能靠改后缀兼容 WebM。技术原型要实际解码验证，必要时采用服务商支持的单声道 PCM/WAV，或受控转码。拒绝授权、接到电话和网络失败时，原有文字要保留，随时可打字。[网页麦克风授权](https://developer.mozilla.org/en-US/docs/Web/API/MediaDevices/getUserMedia) · [录音格式检测](https://developer.mozilla.org/en-US/docs/Web/API/MediaRecorder/isTypeSupported_static) · [腾讯接口及格式](https://cloud.tencent.com/document/product/1093/35646) |
| 后续原生 Expo App | 系统键盘听写仍可沿用。要做 App 内话筒，可用 Apple Speech 和 Android SpeechRecognizer。iOS 按中文识别器查询 `supportsOnDeviceRecognition`，需要仅本机处理时设 `requiresOnDeviceRecognition`。Android 12 起可查询并显式创建端侧识别器，Android 13 起可进一步查询指定请求的支持情况；旧版的 `EXTRA_PREFER_OFFLINE` 可能被引擎忽略。即使安卓键盘能听写，手机也未必有可供第三方 App 调用的普通话识别服务。设备端不可用时保留键盘，云端识别由用户另选。[Apple Speech](https://developer.apple.com/documentation/speech/sfspeechrecognizer/supportsondevicerecognition) · [Android SpeechRecognizer](https://developer.android.com/reference/android/speech/SpeechRecognizer) · [Android 离线参数](https://developer.android.com/reference/android/speech/RecognizerIntent#EXTRA_PREFER_OFFLINE) |

项目锁定 Expo SDK 57。社区库 `expo-speech-recognition` 的维护者给出了 SDK 57 对应版本；若走原生路线，应固定兼容版本，配置 iOS 的麦克风及语音识别用途说明、Android 的录音权限及可用识别服务，在自定义开发构建里验证。官方 `expo-speech` 用来把文字读出来，`expo-audio` 能录音，两者都不能直接完成听写。本轮未安装任何库。[项目依赖](../apps/client/package.json) · [社区库](https://github.com/jamsch/expo-speech-recognition) · [Expo 开发构建](https://docs.expo.dev/develop/development-builds/introduction/) · [Expo Speech](https://docs.expo.dev/versions/latest/sdk/speech/) · [Expo Audio](https://docs.expo.dev/versions/latest/sdk/audio/)

### 短句识别预算和数据流向

按 **3 位家人、每人每天 5 段、每段 20 秒**估算，每天 5 分钟，每 30 天约 450 次调用。以下是 2026-11-24 核对的公开配额，只计识别，未计请求、传输、失败重试与其他业务共用的额度。

| 选项 | 这份用量大致意味着什么 | 何时值得选 |
|---|---|---|
| 腾讯云一句话识别 | 官方列出开通后每月 5,000 次免费成功识别，单段最长 60 秒、最大 3 MB，可直接传音频数据。450 次约占月额度 9%。可关闭后付费，核对账号的免费包已生效。 | 若家人需要明显的站内按钮，值得用实际普通话、长辈口音和人名优先试录。仍经现有 Cloudflare Worker 中转时，整个请求仍要先到 skarma.vip；选择国内服务商本身不能证明端到端延迟或音频全程在境内。[计费与免费额度](https://cloud.tencent.com/document/product/1093/35686) · [一句话接口](https://cloud.tencent.com/document/product/1093/35646) |
| Cloudflare Workers AI，Whisper Large V3 Turbo | 官方计价 46.63 Neurons/音频分钟，Workers AI 免费配额为每日 10,000 Neurons。例中约 233 Neurons/天，使用同一账号的其他 AI 调用也会占用额度。 | 沿用现有供应商便于运维，可将模型语言设为中文并做对照。实际速度取决于家庭网络和推理耗时。项目生产发布器目前只接受现有类型的绑定；若新增 AI binding，需审查发布配置和校验逻辑，同时保留原数据库绑定。[官方计价](https://developers.cloudflare.com/workers-ai/platform/pricing/) · [模型参数](https://developers.cloudflare.com/workers-ai/models/whisper-large-v3-turbo/) · [项目发布校验](../tools/cloudflare_release.py) |

**建议的短句数据路径**是浏览器采音 → 当前登录账号的转写接口 → 明示的识别服务商 → 一段候选文字。密钥保留在服务端；接口限制时长、大小、并发和账号用量。优先直接传本段音频，业务库和日志均不留音频或转写正文；供应商自身的留存与处理地区需按选用服务另行核对。弹窗展示转写内容，用户可插入、修改或取消。录音期间切到另一条课堂观察、退出表单或请求重复返回，都不能把文字误插到其他字段。选择目标、关联方法、正式提交仍由家人操作。

### T88-3 真机验收与选型

第一步只需一台家里常用的 iPhone 和一台日常记课的安卓手机。用合成短句，先在 Safari、安卓默认浏览器各做一次；家人平时从微信或主屏幕图标进，再按原入口复验。每次记下机型、系统、浏览器、输入法、语言和网络。请实际记课的人亲手操作，以便区分找不到键盘话筒和识别不准。

1. 在两条不同的课堂观察中各说一句。试“一开始没有回应，第二次才转头”和“彭彭老师带着轮流拍了三次”，检查否定、次数和姓名，在光标处改一个字，确认旧文字和另一条观察均保留。
2. 在家庭总结的“实际表现”中先打半句再听写，关闭重开草稿，确认文字只出现一次。正式提交并刷新，确认保留家人校正后的版本。若现场有背景对话，观察麦克风收进去的内容，再决定是否课后找安静处口述。
3. 在已打开的草稿里断网试录、拒绝权限、手动停止；分别观察能否得到本地文字以及恢复网络后的正式保存结果。以后测试站内话筒时，再加切字段、系统来电中断及取消后不插字。
4. 比较键盘与候选站内原型时，用同一组十条短句记录首次找到入口的用时、说完到出字的用时、需修改的关键字和失败次数。若键盘路径已能独立完成，保留轻量提示即可；若困难主要是找入口，优先原型验证“说一句”；若主要是方言或专名错识，先比较引擎，再投资新界面。

本轮完成了项目源码、PRD 和上列厂商文档的核对。未使用家庭手机、录制家庭语音、测量大陆网络或开通服务。以上额度是预算，手机端可用性和识别效果仍待真机试录。
