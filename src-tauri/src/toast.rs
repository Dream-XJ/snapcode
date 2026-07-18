//! Windows Toast 通知：用于粘贴结果等轻量提示。

/// 应用的用户模型 ID（AUMID），Toast 通知以此标识应用身份。
pub const APP_AUMID: &str = "com.snapcode.app";

/// 在开始菜单 Programs 下创建带 AUMID 的快捷方式，
/// 使未打包应用也能以自己的身份弹出 Toast。
/// 任何失败返回 Err(描述)，不 panic。
#[cfg(windows)]
pub fn ensure_app_shortcut() -> Result<(), String> {
    use windows::core::{Interface, HSTRING, PROPVARIANT};
    use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
    use windows::Win32::System::Com::StructuredStorage::{PropVariantChangeType, PVCHF_DEFAULT};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::System::Variant::VT_LPWSTR;
    use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    let appdata = std::env::var("APPDATA").map_err(|e| format!("获取 %APPDATA% 失败: {e}"))?;
    let dir = std::path::PathBuf::from(appdata)
        .join(r"Microsoft\Windows\Start Menu\Programs");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建开始菜单目录失败: {e}"))?;
    let lnk = dir.join("SnapCode.lnk");
    let exe = std::env::current_exe().map_err(|e| format!("获取当前可执行文件路径失败: {e}"))?;

    // 旧快捷方式可能指向旧 exe 路径，先删再建，避免覆盖保存时残留旧属性
    let _ = std::fs::remove_file(&lnk);
    // 0.1.x 时代的快捷方式名为「SnapCode 闪码.lnk」，随更名一并清理
    let _ = std::fs::remove_file(dir.join("SnapCode 闪码.lnk"));

    unsafe {
        // COM 可能已被宿主线程初始化；重复调用返回 S_FALSE，忽略即可
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("创建 ShellLink 失败: {e}"))?;
        link.SetPath(&HSTRING::from(exe.as_os_str()))
            .map_err(|e| format!("SetPath 失败: {e}"))?;
        link.SetDescription(&HSTRING::from("SnapCode"))
            .map_err(|e| format!("SetDescription 失败: {e}"))?;

        let store: IPropertyStore = link
            .cast()
            .map_err(|e| format!("获取 IPropertyStore 失败: {e}"))?;
        // AppUserModel.ID 须为 VT_LPWSTR；windows 0.58 无 InitPropVariantFromString，
        // 先构造 VT_BSTR 再转换（PROPVARIANT 带 Drop，无需手动 Clear）
        let src = PROPVARIANT::from(APP_AUMID);
        let mut aumid_pv = PROPVARIANT::new();
        PropVariantChangeType(&mut aumid_pv, &src, PVCHF_DEFAULT, VT_LPWSTR)
            .map_err(|e| format!("构造 PROPVARIANT 失败: {e}"))?;
        store
            .SetValue(&PKEY_AppUserModel_ID, &aumid_pv)
            .map_err(|e| format!("写入 AppUserModel.ID 失败: {e}"))?;
        store.Commit().map_err(|e| format!("提交属性失败: {e}"))?;

        let persist: IPersistFile = link
            .cast()
            .map_err(|e| format!("获取 IPersistFile 失败: {e}"))?;
        persist
            .Save(&HSTRING::from(lnk.as_os_str()), true)
            .map_err(|e| format!("保存快捷方式失败: {e}"))?;
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn ensure_app_shortcut() -> Result<(), String> {
    Ok(())
}

/// 显示一条 ToastGeneric 通知；任何失败仅记录日志。
#[cfg(windows)]
pub fn show_toast(title: &str, body: &str) {
    use windows::core::HSTRING;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    let xml = format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        escape(title),
        escape(body)
    );

    let result = (|| -> windows::core::Result<()> {
        let doc = XmlDocument::new()?;
        doc.LoadXml(&HSTRING::from(xml))?;
        let toast = ToastNotification::CreateToastNotification(&doc)?;
        // 无包身份应用无系统 AUMID：先试系统身份，再试自己的 AUMID（配合 ensure_app_shortcut），
        // 最后才回退借用 PowerShell 的 AUMID
        let notifier = match ToastNotificationManager::CreateToastNotifier() {
            Ok(n) => n,
            Err(_) => match ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
                APP_AUMID,
            )) {
                Ok(n) => n,
                Err(_) => ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
                    "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe",
                ))?,
            },
        };
        notifier.Show(&toast)?;
        Ok(())
    })();

    if let Err(e) = result {
        eprintln!("显示 Toast 通知失败: {e}");
    }
}

#[cfg(not(windows))]
pub fn show_toast(_title: &str, _body: &str) {}
