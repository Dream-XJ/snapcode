/** 中文（简体）词典：i18n 键的单一事实来源，新增文案先在这里加键 */
export const zhCN = {
  /* ---------- 通用 ---------- */
  "app.name": "SnapCode 闪码",
  "app.loading": "加载中…",
  "app.initFailed": "初始化失败：{err}",
  "app.codeCopied": "验证码 {code} 已复制",
  "app.codeReceived": "收到新验证码 {code}",
  "app.accessDeniedToast": "通知访问权限被拒绝，请在设置中授权",

  "common.copy": "复制",
  "common.delete": "删除",
  "common.remove": "移除",
  "common.minimize": "最小化",
  "common.close": "关闭",
  "common.retry": "重新检测",
  "common.openSystemSettings": "打开系统设置",

  /* ---------- 顶栏 / 状态 ---------- */
  "tabs.history": "历史",
  "tabs.settings": "设置",

  "status.running": "监听中",
  "status.paused": "已暂停",
  "status.accessDenied": "权限被拒",
  "status.error": "监听出错",
  "status.starting": "启动中",
  "status.unsupported": "系统不支持",

  "source.phoneLink": "手机连接",

  /* ---------- 历史页 ---------- */
  "history.searchPlaceholder": "搜索验证码、号码或内容…",
  "history.noMatch": "没有匹配的记录",
  "history.noMatchHint": "换个关键字试试",
  "history.emptyTitle": "暂无验证码",
  "history.emptyDesc":
    "请确认「手机连接」已连接手机并开启短信通知同步；iPhone 需保持蓝牙连接，并在 iOS 通知设置中允许短信显示内容。",
  "history.unknownSender": "未知号码",
  "history.used": "已使用",
  "history.copied": "已复制",
  "history.deleted": "已删除",

  /* ---------- 设置页 ---------- */
  "settings.sectionListener": "通知监听",
  "settings.listenerDefaultDesc": "SnapCode 通过读取 Windows 通知识别短信验证码。",
  "settings.sectionAppearance": "外观",
  "settings.theme": "主题",
  "settings.language": "语言",
  "theme.light": "浅色",
  "theme.dark": "深色",
  "theme.system": "跟随系统",
  "settings.sectionShortcut": "全局快捷键",
  "settings.shortcutLabel": "粘贴快捷键",
  "settings.shortcutDesc": "在任意应用中按下，即可粘贴最新验证码",
  "settings.sectionBehavior": "行为",
  "settings.autoCopy": "自动复制",
  "settings.autoCopyDesc": "识别到验证码后立即写入剪贴板",
  "settings.autostart": "开机自启",
  "settings.autostartDesc": "登录 Windows 后自动启动 SnapCode",
  "settings.sectionSources": "通知来源",
  "settings.sourcesDesc": "仅监听以下应用（AUMID）的通知",
  "settings.aumidPlaceholder": "添加 AUMID…",
  "settings.add": "添加",
  "settings.sourceExists": "该来源已存在",
  "settings.restoreDefaultSource": "恢复默认来源（手机连接）",
  "settings.sectionHistory": "历史记录",
  "settings.retention": "保留策略",
  "settings.retentionDesc": "过期记录将自动清理",
  "settings.retentionDay1": "1 天",
  "settings.retentionDays": "{n} 天",
  "settings.retentionForever": "永久保留",
  "settings.clearLabel": "清空历史",
  "settings.clearDesc": "删除全部已捕获的验证码记录",
  "settings.clearAll": "清空全部",
  "settings.clearConfirm": "确定要清空全部历史记录吗？此操作不可撤销。",
  "settings.cleared": "历史记录已清空",
  "settings.sectionDebug": "调试",
  "settings.simDesc": "模拟收到一条短信通知，走完整的识别与入库流程",
  "settings.simPlaceholder": "例如：【微信】您的验证码是 482913，5 分钟内有效。",
  "settings.simulate": "模拟通知",
  "settings.simFound": "识别到验证码 {code}",
  "settings.simNotFound": "未识别到验证码",
  "settings.dumpDesc":
    "抓取不到验证码时，列出当前系统中的 Toast 通知，确认「手机连接」的真实来源 AUMID 并一键加入监听列表",
  "settings.dump": "列出系统通知",
  "settings.dumpEmpty": "当前没有系统通知",
  "settings.dumpNoAumid": "(无 AUMID)",
  "settings.dumpAdd": "加为来源",
  "settings.sectionAbout": "关于",
  "settings.aboutDesc": "v0.1.0 · Windows 短信验证码捕获工具",
  "settings.dataLabel": "数据存储",
  "settings.dataDesc": "全部数据仅保存在本机，不会上传",

  /* ---------- 快捷键录制 ---------- */
  "recorder.saving": "保存中…",
  "recorder.press": "按下组合键…",
  "recorder.click": "点击录入",

  /* ---------- 首次引导 ---------- */
  "onboarding.unsupportedTitle": "系统版本不受支持",
  "onboarding.unsupportedDesc": "需要 Windows 10 1809 或更高版本",
  "onboarding.enterAnyway": "仍要进入应用",
  "onboarding.deniedTitle": "通知访问权限被拒绝",
  "onboarding.deniedDesc":
    "SnapCode 需要读取系统通知才能识别短信验证码，请在系统设置中授权后重新检测。",
  "onboarding.continueAnyway": "暂不授权，继续使用",
  "onboarding.stepListenTitle": "监听通知",
  "onboarding.stepListenDesc": "读取「手机连接」同步到 Windows 的短信通知",
  "onboarding.stepDetectTitle": "识别验证码",
  "onboarding.stepDetectDesc": "自动提取短信中的数字验证码，存入本地历史",
  "onboarding.stepPasteTitle": "快捷粘贴",
  "onboarding.stepPasteDesc1": "按下",
  "onboarding.stepPasteDesc2": "即可粘贴最新验证码",
  "onboarding.welcomeTitle": "欢迎使用 SnapCode 闪码",
  "onboarding.welcomeDesc": "自动捕获 Windows 通知里的短信验证码",
  "onboarding.openSettings": "打开通知设置",
  "onboarding.done": "开始使用",
  "onboarding.footer":
    "使用前请在「手机连接」中开启短信同步：iPhone 需保持蓝牙连接，并在 iOS 通知设置中允许短信显示内容；Android 请在「连接至 Windows」中开启短信同步。",

  /* ---------- 时间 ---------- */
  "time.justNow": "刚刚",
  "time.minutesAgo": "{n} 分钟前",
  "time.hoursAgo": "{n} 小时前",
  "time.daysAgo": "{n} 天前",
  "time.today": "今天",
  "time.yesterday": "昨天",
};

/** 词典类型：en.ts 必须与之同构（同键、字符串值） */
export type Messages = typeof zhCN;
