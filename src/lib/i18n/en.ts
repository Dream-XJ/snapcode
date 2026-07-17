import type { Messages } from "./zh-CN";

/** English dictionary: keys must stay in sync with zh-CN.ts */
export const en: Messages = {
  /* ---------- Common ---------- */
  "app.name": "SnapCode",
  "app.loading": "Loading…",
  "app.initFailed": "Initialization failed: {err}",
  "app.codeCopied": "Code {code} copied",
  "app.codeReceived": "New code received: {code}",
  "app.accessDeniedToast": "Notification access denied. Grant access in Settings.",

  "common.copy": "Copy",
  "common.delete": "Delete",
  "common.remove": "Remove",
  "common.minimize": "Minimize",
  "common.close": "Close",
  "common.retry": "Retry",
  "common.openSystemSettings": "Open system settings",

  /* ---------- Top bar / status ---------- */
  "tabs.history": "History",
  "tabs.settings": "Settings",

  "status.running": "Listening",
  "status.paused": "Paused",
  "status.accessDenied": "Access denied",
  "status.error": "Error",
  "status.starting": "Starting",
  "status.unsupported": "Unsupported",

  "source.phoneLink": "Phone Link",

  /* ---------- History page ---------- */
  "history.searchPlaceholder": "Search code, sender or content…",
  "history.noMatch": "No matching records",
  "history.noMatchHint": "Try a different keyword",
  "history.emptyTitle": "No codes yet",
  "history.emptyDesc":
    "Make sure Phone Link is connected to your phone with SMS notification sync on. On iPhone, keep Bluetooth connected and allow message content in iOS notification settings.",
  "history.unknownSender": "Unknown sender",
  "history.used": "Used",
  "history.copied": "Copied",
  "history.deleted": "Deleted",

  /* ---------- Settings page ---------- */
  "settings.sectionListener": "Notification listener",
  "settings.listenerDefaultDesc":
    "SnapCode reads Windows notifications to detect SMS verification codes.",
  "settings.sectionAppearance": "Appearance",
  "settings.theme": "Theme",
  "settings.language": "Language",
  "theme.light": "Light",
  "theme.dark": "Dark",
  "theme.system": "System",
  "settings.sectionShortcut": "Global shortcut",
  "settings.shortcutLabel": "Paste shortcut",
  "settings.shortcutDesc": "Press in any app to paste the latest code",
  "settings.sectionBehavior": "Behavior",
  "settings.autoCopy": "Auto copy",
  "settings.autoCopyDesc": "Copy to clipboard as soon as a code is detected",
  "settings.autostart": "Launch at startup",
  "settings.autostartDesc": "Start SnapCode automatically after signing in to Windows",
  "settings.sectionSources": "Notification sources",
  "settings.sourcesDesc": "Only listen to notifications from these apps (AUMIDs)",
  "settings.aumidPlaceholder": "Add AUMID…",
  "settings.add": "Add",
  "settings.sourceExists": "This source already exists",
  "settings.restoreDefaultSource": "Restore default source (Phone Link)",
  "settings.sectionHistory": "History",
  "settings.retention": "Retention",
  "settings.retentionDesc": "Expired records are cleaned up automatically",
  "settings.retentionDay1": "1 day",
  "settings.retentionDays": "{n} days",
  "settings.retentionForever": "Keep forever",
  "settings.clearLabel": "Clear history",
  "settings.clearDesc": "Delete all captured verification codes",
  "settings.clearAll": "Clear all",
  "settings.clearConfirm": "Clear all history? This cannot be undone.",
  "settings.cleared": "History cleared",
  "settings.sectionDebug": "Debug",
  "settings.simDesc": "Simulate an incoming SMS notification and run the full detection pipeline",
  "settings.simPlaceholder": "e.g. Your verification code is 482913, valid for 5 minutes.",
  "settings.simulate": "Simulate",
  "settings.simFound": "Code detected: {code}",
  "settings.simNotFound": "No code detected",
  "settings.dumpDesc":
    "If no codes are captured, list current system toast notifications to find the real source AUMID of Phone Link and add it to the listen list with one click",
  "settings.dump": "List system notifications",
  "settings.dumpEmpty": "No system notifications",
  "settings.dumpNoAumid": "(no AUMID)",
  "settings.dumpAdd": "Add as source",
  "settings.sectionAbout": "About",
  "settings.aboutDesc": "v0.1.0 · SMS verification code catcher for Windows",
  "settings.dataLabel": "Data storage",
  "settings.dataDesc": "All data stays on this device; nothing is uploaded",

  /* ---------- Shortcut recorder ---------- */
  "recorder.saving": "Saving…",
  "recorder.press": "Press keys…",
  "recorder.click": "Click to record",

  /* ---------- Onboarding ---------- */
  "onboarding.unsupportedTitle": "Unsupported Windows version",
  "onboarding.unsupportedDesc": "Windows 10 1809 or later is required",
  "onboarding.enterAnyway": "Enter anyway",
  "onboarding.deniedTitle": "Notification access denied",
  "onboarding.deniedDesc":
    "SnapCode needs to read system notifications to detect SMS codes. Grant access in system settings, then retry.",
  "onboarding.continueAnyway": "Continue without access",
  "onboarding.stepListenTitle": "Listen",
  "onboarding.stepListenDesc": "Reads SMS notifications synced to Windows by Phone Link",
  "onboarding.stepDetectTitle": "Detect",
  "onboarding.stepDetectDesc": "Extracts numeric codes from messages into local history",
  "onboarding.stepPasteTitle": "Paste",
  "onboarding.stepPasteDesc1": "Press",
  "onboarding.stepPasteDesc2": "to paste the latest code",
  "onboarding.welcomeTitle": "Welcome to SnapCode",
  "onboarding.welcomeDesc": "Automatically capture SMS verification codes from Windows notifications",
  "onboarding.openSettings": "Open notification settings",
  "onboarding.done": "Get started",
  "onboarding.footer":
    "Before use, enable SMS sync in Phone Link: on iPhone keep Bluetooth connected and allow message content in iOS notification settings; on Android enable SMS sync in Link to Windows.",

  /* ---------- Time ---------- */
  "time.justNow": "just now",
  "time.minutesAgo": "{n} min ago",
  "time.hoursAgo": "{n} hr ago",
  "time.daysAgo": "{n} d ago",
  "time.today": "Today",
  "time.yesterday": "Yesterday",
};
