//! 邮箱验证码监听：定时轮询新邮件（当前支持 POP3，IMAP 接入中），提取验证码后
//! 走与通知监听相同的入库 → 广播 → 自动复制管线。
//!
//! 去重策略：SQLite email_seen 表按账户记录已处理邮件的 UIDL；每个账户首次成功
//! 连接时只把现存邮件标记为已见（基线），不导入历史邮件——否则启用瞬间可能灌入
//! 大量旧码。账户身份（host/username）变更或账户被删除时 update_settings 会清掉
//! 该账户的去重记录与 IMAP 同步状态，下次轮询重建基线。
//!
//! 多账户并行轮询完整接入前，线程只跟踪第一个「启用且配置完整」的账户。
//!
//! 本模块无 Windows 专用调用，全平台可编译可测。

use std::collections::HashSet;
use std::io::{Read, Write};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::i18n;
use crate::mail::{self, Pop3Client};
use crate::parser::extract_code;
use crate::settings::{EmailAccount, EmailProtocol};
use crate::state::AppState;
use crate::storage::{now_millis, EMAIL_BASELINE_UIDL};

/// 单次轮询最多拉取的邮件数：异常积压（如长期关机）时逐轮消化，避免一轮拖死连接
const MAX_PER_POLL: usize = 50;

/// 启动邮箱轮询线程；CAS 保证不会重复启动。
pub fn spawn_email_monitor(app: AppHandle, state: Arc<AppState>) {
    if state
        .email_alive
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    std::thread::spawn(move || {
        run(&app, &state);
        state.email_alive.store(false, Ordering::SeqCst);
    });
}

/// 主循环：每 0.5s 检查一次配置与暂停标志，到间隔才发起一轮轮询。
/// 配置热生效：设置变更在下一轮轮询前被读到，无需重启线程。
/// 多账户完整支持前，只取第一个「启用且配置完整」的账户作为工作对象。
fn run(app: &AppHandle, state: &Arc<AppState>) {
    let mut next_poll = Instant::now();
    let mut active_id: Option<String> = None;
    loop {
        std::thread::sleep(Duration::from_millis(500));
        let cfg = {
            let settings = state.settings.read().unwrap();
            settings
                .email
                .accounts
                .iter()
                .find(|a| a.enabled && a.is_complete())
                .cloned()
        };

        // 工作账户切换（含变为无可用账户）时，移除旧账户的状态条目
        let new_id = cfg.as_ref().map(|a| a.id.clone());
        if active_id != new_id {
            if let Some(old_id) = active_id.take() {
                state.remove_email_status(app, &old_id);
            }
            active_id = new_id;
        }

        let Some(cfg) = cfg else {
            // 无启用且完整的账户：没有可上报的对象（旧状态已在上面移除）
            next_poll = Instant::now(); // 恢复启用时立即轮询
            continue;
        };

        // IMAP 尚未接入：明确报 error 而不是假装在轮询。
        // 这是配置层面的永久状态，优先级高于全局暂停
        if cfg.protocol == EmailProtocol::Imap {
            state.set_email_status(
                app,
                &cfg.id,
                "error",
                Some(i18n::email_protocol_unsupported(&state.lang()).to_string()),
            );
            next_poll = Instant::now();
            continue;
        }

        if state.paused.load(Ordering::SeqCst) {
            state.set_email_status(app, &cfg.id, "paused", None);
            next_poll = Instant::now();
            continue;
        }
        if Instant::now() < next_poll {
            continue;
        }
        next_poll = Instant::now() + Duration::from_secs(cfg.interval());

        match poll_once(app, state, &cfg) {
            Ok(()) => state.set_email_status(app, &cfg.id, "running", None),
            Err(e) => {
                eprintln!("邮箱轮询失败: {e}");
                state.set_email_status(app, &cfg.id, "error", Some(e));
            }
        }
    }
}

/// 一轮轮询：连接 → 登录 → UIDL 比对 → 拉取新邮件 → 逐封处理。连接总会以 QUIT 收尾。
fn poll_once(app: &AppHandle, state: &Arc<AppState>, cfg: &EmailAccount) -> Result<(), String> {
    let mut client = mail::connect(cfg).map_err(|e| i18n::email_connect_failed(&state.lang(), &e))?;
    let result = poll_with_client(app, state, cfg, &mut client);
    client.quit();
    result
}

fn poll_with_client<S: Read + Write>(
    app: &AppHandle,
    state: &Arc<AppState>,
    cfg: &EmailAccount,
    client: &mut Pop3Client<S>,
) -> Result<(), String> {
    client
        .login(&cfg.username, &cfg.password)
        .map_err(|e| i18n::email_auth_failed(&state.lang(), &e))?;
    let uidls = client
        .uidl()
        .map_err(|e| i18n::email_poll_failed(&state.lang(), &e))?;
    let seen = state.db.email_seen_set(&cfg.id)?;

    match plan_poll(&seen, &uidls) {
        PollPlan::Baseline(marks) => {
            state.db.email_seen_mark(&cfg.id, &marks)?;
            eprintln!(
                "邮箱基线已建立：{} 封现存邮件标记为已见，不导入历史",
                uidls.len()
            );
        }
        PollPlan::Fetch(list) => {
            for (num, uidl) in list {
                let raw = match client.retr(num) {
                    Ok(raw) => raw,
                    // 拉取失败多半是连接问题；该邮件未标记已见，下轮重试
                    Err(e) => return Err(i18n::email_poll_failed(&state.lang(), &e)),
                };
                state.db.email_seen_mark(&cfg.id, &[uidl])?;
                handle_mail(app, state, cfg, &raw);
            }
        }
    }
    Ok(())
}

/// 轮询决策（纯函数）：建基线 or 拉取哪些新邮件。
enum PollPlan {
    /// 首次连接：把全部现存 UIDL + 基线哨兵标记为已见
    Baseline(Vec<String>),
    /// 增量：需要 RETR 的（序号, UIDL），最多 MAX_PER_POLL 封
    Fetch(Vec<(u32, String)>),
}

fn plan_poll(seen: &HashSet<String>, uidls: &[(u32, String)]) -> PollPlan {
    if !seen.contains(EMAIL_BASELINE_UIDL) {
        let mut marks: Vec<String> = uidls.iter().map(|(_, u)| u.clone()).collect();
        marks.push(EMAIL_BASELINE_UIDL.to_string());
        return PollPlan::Baseline(marks);
    }
    let new: Vec<(u32, String)> = uidls
        .iter()
        .filter(|(_, u)| !seen.contains(u))
        .take(MAX_PER_POLL)
        .map(|(n, u)| (*n, u.clone()))
        .collect();
    PollPlan::Fetch(new)
}

/// 处理单封邮件：解析 → 提取验证码 → 入库 → 广播 → 自动复制。
/// 与 notifications.rs 的通知处理是同一条管线（两处各自调用，保持最小侵入）。
fn handle_mail(app: &AppHandle, state: &Arc<AppState>, cfg: &EmailAccount, raw: &[u8]) {
    let parsed = match mail::parse_mail(raw) {
        Some(m) => m,
        None => return,
    };
    let code = match extract_code(&parsed.text) {
        Some(c) => c,
        None => return,
    };

    // 来源标识取传入的账户快照：轮询期间设置可能已变更，重读全局配置会张冠李戴
    let source = format!("email:{}", cfg.display_name());
    let received_at = parsed.received_at.unwrap_or_else(now_millis);
    let body = truncate_chars(&parsed.text, 500);

    let record = match state
        .db
        .insert(&source, parsed.sender.as_deref(), &body, &code, received_at)
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("邮件验证码入库失败: {e}");
            return;
        }
    };

    if let Err(e) = app.emit("code-added", &record) {
        eprintln!("广播 code-added 事件失败: {e}");
    }

    if state.settings.read().unwrap().auto_copy {
        if let Err(e) = app.clipboard().write_text(&code) {
            eprintln!("自动复制到剪贴板失败: {e}");
        }
    }
}

/// 按字符数截断（入库正文上限，防止超长邮件撑爆历史记录）。
fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uidl_set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn first_poll_builds_baseline() {
        let seen = HashSet::new();
        let uidls = vec![
            (1, "a".to_string()),
            (2, "b".to_string()),
        ];
        match plan_poll(&seen, &uidls) {
            PollPlan::Baseline(marks) => {
                assert_eq!(marks, vec!["a".to_string(), "b".to_string(), EMAIL_BASELINE_UIDL.to_string()]);
            }
            PollPlan::Fetch(_) => panic!("首次轮询应建基线而非拉取"),
        }
    }

    #[test]
    fn baseline_with_empty_mailbox() {
        // 空邮箱也要写入哨兵，否则后续邮件会被当作「首次」反复基线
        let seen = HashSet::new();
        match plan_poll(&seen, &[]) {
            PollPlan::Baseline(marks) => {
                assert_eq!(marks, vec![EMAIL_BASELINE_UIDL.to_string()]);
            }
            PollPlan::Fetch(_) => panic!("空邮箱首次轮询仍应建基线"),
        }
    }

    #[test]
    fn incremental_poll_fetches_only_unseen() {
        let seen = uidl_set(&[EMAIL_BASELINE_UIDL, "a", "b"]);
        let uidls = vec![
            (1, "a".to_string()),
            (2, "b".to_string()),
            (3, "c".to_string()),
        ];
        match plan_poll(&seen, &uidls) {
            PollPlan::Fetch(list) => assert_eq!(list, vec![(3, "c".to_string())]),
            PollPlan::Baseline(_) => panic!("已有基线应增量拉取"),
        }
    }

    #[test]
    fn poll_is_capped() {
        let seen = uidl_set(&[EMAIL_BASELINE_UIDL]);
        let uidls: Vec<(u32, String)> = (1..=80).map(|i| (i, format!("u{i}"))).collect();
        match plan_poll(&seen, &uidls) {
            PollPlan::Fetch(list) => {
                assert_eq!(list.len(), MAX_PER_POLL);
                assert_eq!(list[0].0, 1); // 从最旧开始逐轮消化
            }
            PollPlan::Baseline(_) => panic!("已有基线应增量拉取"),
        }
    }

    #[test]
    fn truncate_bounds_body() {
        assert_eq!(truncate_chars("abc", 500), "abc");
        let long = "x".repeat(600);
        assert_eq!(truncate_chars(&long, 500).chars().count(), 500);
        // 按字符而非字节截断，中文不会截出乱码
        let zh = "验".repeat(600);
        assert_eq!(truncate_chars(&zh, 500), "验".repeat(500));
    }
}
