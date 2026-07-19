//! 邮箱验证码监听：监督线程 + 每账户 worker 模型。
//!
//! 监督线程（spawn_email_monitor 启动，CAS 防重入）每 0.5s 热读账户配置并 reconcile，
//! 为每个「启用且完整」的账户维持一个独立 worker OS 线程（POP3 轮询 / IMAP 增量同步），
//! 并统一管理「未启用 / 配置不完整 / 全局暂停」三类状态显示；配置变更或账户删除时
//! 置 stop 标志让旧 worker 自行退出——worker 可能正阻塞在 IDLE 等待或 POP3 连接里，
//! join 会卡住监督线程，故 detach 不 join。
//!
//! 去重策略：
//! - POP3：email_seen 表按账户记录已处理邮件的 UIDL；首次连接只把现存邮件标记为已见
//!   （基线），不导入历史邮件——否则启用瞬间可能灌入大量旧码；
//! - IMAP：imap_state 表按账户记录 (UIDVALIDITY, 已处理最大 UID)；首连或 UIDVALIDITY
//!   变化时同样只建基线（max_uid = UIDNEXT - 1），此后按 UID 单调递增天然增量去重。
//! 账户身份（host/username）变更或账户被删除时 update_settings 会清掉该账户的
//! 去重记录与 IMAP 同步状态，下次连接重建基线。
//!
//! 本模块无 Windows 专用调用，全平台可编译可测。

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::i18n;
use crate::imap_client::ImapSession;
use crate::mail::{self, Pop3Client};
use crate::parser::extract_code;
use crate::settings::{EmailAccount, EmailProtocol};
use crate::state::AppState;
use crate::storage::{now_millis, EMAIL_BASELINE_UIDL};

/// 单次轮询/单轮同步最多拉取的邮件数：异常积压（如长期关机）时逐轮消化，避免一轮拖死连接
const MAX_PER_POLL: usize = 50;

/// IMAP IDLE 一轮的 keepalive。RFC 2177 建议 29 分钟内重发 IDLE，60s 远低于此；
/// 同时把 stop/暂停/配置变更的最坏响应时间控制在可接受范围（worker 阻塞在 IDLE
/// 等待里时无法即时响应）。推送唤醒即处理新邮件，60s 空闲到期仅重新 IDLE。
const IDLE_KEEPALIVE: Duration = Duration::from_secs(60);

/// 监督线程持有的 worker 句柄：配置快照（变更检测用）+ 停止标志
struct WorkerHandle {
    cfg: EmailAccount,
    stop: Arc<AtomicBool>,
}

/// 启动邮箱监听监督线程；CAS 保证不会重复启动。
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

/// 监督主循环：每 0.5s reconcile worker 集合，并统一管理非 worker 职责的状态显示。
fn run(app: &AppHandle, state: &Arc<AppState>) {
    let mut workers: HashMap<String, WorkerHandle> = HashMap::new();
    loop {
        std::thread::sleep(Duration::from_millis(500));
        let accounts = state.settings.read().unwrap().email.accounts.clone();

        // 1) reconcile：worker 集合向「启用且完整」的账户集合收敛
        let running: HashMap<String, EmailAccount> = workers
            .iter()
            .map(|(id, h)| (id.clone(), h.cfg.clone()))
            .collect();
        for action in reconcile(&accounts, &running) {
            match action {
                AccountAction::Spawn(cfg) => {
                    // 同 id 旧 worker（配置已变更）先置 stop 让其自行退出
                    if let Some(old) = workers.remove(&cfg.id) {
                        old.stop.store(true, Ordering::SeqCst);
                    }
                    let stop = Arc::new(AtomicBool::new(false));
                    workers.insert(
                        cfg.id.clone(),
                        WorkerHandle {
                            cfg: cfg.clone(),
                            stop: stop.clone(),
                        },
                    );
                    spawn_worker(app.clone(), state.clone(), cfg, stop);
                }
                AccountAction::Stop(id) => {
                    if let Some(h) = workers.remove(&id) {
                        h.stop.store(true, Ordering::SeqCst);
                    }
                }
            }
        }

        // 2) 状态显示：未启用/配置不完整/全局暂停由 supervisor 负责（worker 暂停期间
        //    不发状态，0.5s tick 保证暂停显示即时）；running/error 归 worker。
        //    set_email_status 对无变化账户不重复广播，逐 tick 调用是廉价的。
        let paused = state.paused.load(Ordering::SeqCst);
        for acc in &accounts {
            if !acc.enabled {
                state.set_email_status(app, &acc.id, "disabled", None);
            } else if !acc.is_complete() {
                state.set_email_status(
                    app,
                    &acc.id,
                    "disabled",
                    Some(i18n::email_config_incomplete(&state.lang()).to_string()),
                );
            } else if paused {
                state.set_email_status(app, &acc.id, "paused", None);
            }
        }

        // 3) 已从配置中删除的账户：清掉残留状态条目（worker 已在第 1 步停止）
        let stale: Vec<String> = {
            let guard = state.email_status.read().unwrap();
            guard
                .keys()
                .filter(|id| !accounts.iter().any(|a| &a.id == *id))
                .cloned()
                .collect()
        };
        for id in stale {
            state.remove_email_status(app, &id);
        }
    }
}

/// 监督决策结果。
#[derive(Debug, PartialEq)]
enum AccountAction {
    /// 新增或配置变更：启动（或重启）该账户的 worker
    Spawn(EmailAccount),
    /// 账户被删除或不再可运行：停止对应 worker
    Stop(String),
}

/// reconcile（纯函数）：worker 集合与「启用且完整」账户集合的差异 → 动作序列。
/// 遍历顺序确定：先按 desired 顺序产出 Spawn，再按 running 的 id 排序产出 Stop。
fn reconcile(
    desired: &[EmailAccount],
    running: &HashMap<String, EmailAccount>,
) -> Vec<AccountAction> {
    let runnable = |a: &&EmailAccount| a.enabled && a.is_complete();
    let mut actions = Vec::new();
    for acc in desired.iter().filter(runnable) {
        // 配置完全一致则保持现状；缺失或任何字段变化都重启（停旧 worker 由执行方负责）
        if running.get(&acc.id) != Some(acc) {
            actions.push(AccountAction::Spawn(acc.clone()));
        }
    }
    // HashMap 遍历顺序不定，排序保证输出确定（可测、行为可复现）
    let mut running_ids: Vec<&String> = running.keys().collect();
    running_ids.sort();
    for id in running_ids {
        if !desired.iter().any(|a| &a.id == id && runnable(&a)) {
            actions.push(AccountAction::Stop(id.clone()));
        }
    }
    actions
}

/// IMAP 重连退避秒数（纯函数）：5,10,20,40,80,160 后封顶 300。
fn backoff_secs(attempt: u32) -> u64 {
    // attempt 随失败次数无限增长，先钳位移位数防溢出；钳到 10 已远超封顶值
    (5u64 << attempt.min(10)).min(300)
}

/// IMAP 增量同步起点决策（纯函数）：返回 (起始 max_uid, 是否需要写库)。
/// stored 的 UIDVALIDITY 与当前一致 → 续用 stored max_uid，不写库；
/// 否则（首连或 UIDVALIDITY 变化）只建基线：max_uid = UIDNEXT - 1，现存邮件不导入
/// （与 POP3 基线语义一致）。uid_next 为 0 时饱和到 0。
fn plan_uid_state(stored: Option<(u32, u32)>, uid_validity: u32, uid_next: u32) -> (u32, bool) {
    match stored {
        Some((v, max_uid)) if v == uid_validity => (max_uid, false),
        _ => (uid_next.saturating_sub(1), true),
    }
}

/// 为账户启动 worker 线程（detach 不 join，理由见模块文档）。
fn spawn_worker(app: AppHandle, state: Arc<AppState>, cfg: EmailAccount, stop: Arc<AtomicBool>) {
    std::thread::spawn(move || match cfg.protocol {
        EmailProtocol::Pop3 => pop3_worker(&app, &state, &cfg, &stop),
        EmailProtocol::Imap => imap_worker(&app, &state, &cfg, &stop),
    });
}

/// worker 侧统一的状态上报入口：先查 stop——已下线的 worker 不得复活
/// 已被 supervisor 移除的状态条目（如账户被删后旧 worker 的迟到上报）。
fn set_worker_status(
    app: &AppHandle,
    state: &Arc<AppState>,
    cfg: &EmailAccount,
    stop: &AtomicBool,
    status: &str,
    message: Option<String>,
) {
    if stop.load(Ordering::SeqCst) {
        return;
    }
    state.set_email_status(app, &cfg.id, status, message);
}

/// 分片睡眠（每片 0.5s）：任一片检测到 stop 或全局暂停立即返回 true（被打断），
/// 睡满返回 false。让长等待（重连退避/轮询间隔）能及时响应停止与暂停。
fn sleep_interrupted(state: &Arc<AppState>, stop: &AtomicBool, secs: u64) -> bool {
    for _ in 0..secs.saturating_mul(2) {
        if stop.load(Ordering::SeqCst) || state.paused.load(Ordering::SeqCst) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
}

/* ---------- POP3 worker ---------- */

/// POP3 worker：0.5s tick，到期发起一轮轮询。轮询内的阻塞调用最长约 IO_TIMEOUT（15s），
/// stop/配置变更的响应延迟以此为上限。
fn pop3_worker(app: &AppHandle, state: &Arc<AppState>, cfg: &EmailAccount, stop: &AtomicBool) {
    let mut next_poll = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(500));
        if stop.load(Ordering::SeqCst) {
            return;
        }
        if state.paused.load(Ordering::SeqCst) {
            // 暂停期间状态显示归 supervisor；重置计时使恢复后立即轮询
            next_poll = Instant::now();
            continue;
        }
        if Instant::now() < next_poll {
            continue;
        }
        next_poll = Instant::now() + Duration::from_secs(cfg.interval());

        match poll_once(app, state, cfg) {
            Ok(()) => set_worker_status(app, state, cfg, stop, "running", None),
            Err(e) => {
                eprintln!("邮箱轮询失败 ({}): {e}", cfg.display_name());
                set_worker_status(app, state, cfg, stop, "error", Some(e));
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

/* ---------- IMAP worker ---------- */

/// IMAP worker：维持一条长连接做增量同步；连接级失败按指数退避重连。
fn imap_worker(app: &AppHandle, state: &Arc<AppState>, cfg: &EmailAccount, stop: &AtomicBool) {
    let mut attempt: u32 = 0;
    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        if state.paused.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        match imap_sync_loop(app, state, cfg, stop) {
            // 被 stop/暂停正常打断：清零退避直接继续（循环顶部再判定走向）
            Ok(()) => attempt = 0,
            Err(e) => {
                if stop.load(Ordering::SeqCst) {
                    return;
                }
                if state.paused.load(Ordering::SeqCst) {
                    continue;
                }
                eprintln!("IMAP 同步失败 ({}): {e}", cfg.display_name());
                set_worker_status(
                    app,
                    state,
                    cfg,
                    stop,
                    "error",
                    Some(i18n::email_poll_failed(&state.lang(), &e)),
                );
                let secs = backoff_secs(attempt);
                attempt = attempt.saturating_add(1);
                // 0.5s 切片睡眠：stop/暂停在退避期间也能即时生效（由循环顶部处置）
                sleep_interrupted(state, stop, secs);
            }
        }
    }
}

/// 一次 IMAP 连接的生命周期：建连 → 会话内同步 → LOGOUT 收尾（忽略结果：
/// 失败路径上连接可能已断）。返回 Ok 表示被 stop/暂停正常打断，Err 为连接级失败。
fn imap_sync_loop(
    app: &AppHandle,
    state: &Arc<AppState>,
    cfg: &EmailAccount,
    stop: &AtomicBool,
) -> Result<(), String> {
    let mut session = ImapSession::connect(
        &cfg.host,
        cfg.port,
        cfg.use_tls,
        &cfg.username,
        &cfg.password,
    )?;
    let result = imap_run_session(app, state, cfg, stop, &mut session);
    session.logout();
    result
}

/// 会话内同步：能力探测 → SELECT INBOX → 建/校同步基线 → 增量拉取 → IDLE/定时等待。
fn imap_run_session(
    app: &AppHandle,
    state: &Arc<AppState>,
    cfg: &EmailAccount,
    stop: &AtomicBool,
    session: &mut ImapSession,
) -> Result<(), String> {
    // CAPABILITY 探测失败按不支持 IDLE 降级（退化为按 interval 轮询），不致命
    let idle_capable = session.has_idle().unwrap_or(false);
    let mbox = session.select_inbox()?;

    // 同步基线：UIDVALIDITY 一致才续用已存 max_uid，否则只建基线不导入历史
    let stored = state.db.imap_state_get(&cfg.id)?;
    let (mut max_uid, need_write) = plan_uid_state(stored, mbox.uid_validity, mbox.uid_next);
    if need_write {
        state.db.imap_state_set(&cfg.id, mbox.uid_validity, max_uid)?;
    }

    set_worker_status(app, state, cfg, stop, "running", None);

    loop {
        // 每轮先查 stop/暂停：保证暂停期间绝不拉取/入库；退出时的 LOGOUT 由 imap_sync_loop 负责
        if stop.load(Ordering::SeqCst) || state.paused.load(Ordering::SeqCst) {
            return Ok(());
        }
        for uid in session
            .search_newer_than(max_uid)?
            .into_iter()
            .take(MAX_PER_POLL)
        {
            if stop.load(Ordering::SeqCst) || state.paused.load(Ordering::SeqCst) {
                return Ok(());
            }
            // 拉取失败：max_uid 不前进也不落库，重连后从同一封重试
            let raw = session.fetch_body(uid)?;
            handle_mail(app, state, cfg, &raw);
            max_uid = uid;
            state.db.imap_state_set(&cfg.id, mbox.uid_validity, max_uid)?;
        }
        if idle_capable {
            // 推送唤醒或 keepalive 到期均返回 Ok，随即进入下一轮查新邮件；
            // 断线等 IO 错误冒泡，由外层退避重连
            session.idle_wait(IDLE_KEEPALIVE)?;
        } else if sleep_interrupted(state, stop, cfg.interval()) {
            return Ok(());
        }
    }
}

/* ---------- 共用邮件处理管线 ---------- */

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

    /* ---------- reconcile ---------- */

    /// 最小可运行账户（enabled + 关键字段齐全）
    fn acc(id: &str) -> EmailAccount {
        EmailAccount {
            id: id.to_string(),
            enabled: true,
            host: "imap.example.com".to_string(),
            username: "u@example.com".to_string(),
            password: "x".to_string(),
            ..EmailAccount::default()
        }
    }

    fn running_map(cfgs: &[EmailAccount]) -> HashMap<String, EmailAccount> {
        cfgs.iter().map(|c| (c.id.clone(), c.clone())).collect()
    }

    #[test]
    fn reconcile_spawns_new_account() {
        let a = acc("a1");
        let actions = reconcile(&[a.clone()], &HashMap::new());
        assert_eq!(actions, vec![AccountAction::Spawn(a)]);
    }

    #[test]
    fn reconcile_stops_removed_account() {
        let actions = reconcile(&[], &running_map(&[acc("a1")]));
        assert_eq!(actions, vec![AccountAction::Stop("a1".to_string())]);
    }

    #[test]
    fn reconcile_restarts_on_config_change() {
        let old = acc("a1");
        let mut new = old.clone();
        new.port = 993; // 任意字段差异都算配置变更
        let actions = reconcile(&[new.clone()], &running_map(&[old]));
        assert_eq!(actions, vec![AccountAction::Spawn(new)]);
    }

    #[test]
    fn reconcile_keeps_unchanged() {
        let a = acc("a1");
        assert!(reconcile(&[a.clone()], &running_map(&[a])).is_empty());
    }

    #[test]
    fn reconcile_ignores_disabled_and_incomplete() {
        let mut disabled = acc("a1");
        disabled.enabled = false;
        let mut incomplete = acc("a2");
        incomplete.password = String::new();
        // 禁用与不完整账户不 spawn
        assert!(reconcile(&[disabled.clone(), incomplete.clone()], &HashMap::new()).is_empty());
        // 正在运行的账户被禁用或变得不完整后要停掉（按 id 排序输出）
        let actions = reconcile(
            &[disabled, incomplete],
            &running_map(&[acc("a1"), acc("a2")]),
        );
        assert_eq!(
            actions,
            vec![
                AccountAction::Stop("a1".to_string()),
                AccountAction::Stop("a2".to_string()),
            ]
        );
    }

    #[test]
    fn reconcile_mixed_accounts() {
        // a1 不变、a2 配置变更重启、a3 新增、a4 被删除、a5 禁用（运行中 → 停）
        let a1 = acc("a1");
        let mut a2_new = acc("a2");
        a2_new.host = "pop.example.com".to_string();
        let a3 = acc("a3");
        let mut a5 = acc("a5");
        a5.enabled = false;
        let desired = vec![a1.clone(), a2_new.clone(), a3.clone(), a5];
        let running = running_map(&[a1, acc("a2"), acc("a4"), acc("a5")]);
        let actions = reconcile(&desired, &running);
        assert_eq!(
            actions,
            vec![
                AccountAction::Spawn(a2_new),
                AccountAction::Spawn(a3),
                AccountAction::Stop("a4".to_string()),
                AccountAction::Stop("a5".to_string()),
            ]
        );
    }

    /* ---------- backoff ---------- */

    #[test]
    fn backoff_grows_exponentially_then_caps() {
        assert_eq!(backoff_secs(0), 5);
        assert_eq!(backoff_secs(1), 10);
        assert_eq!(backoff_secs(2), 20);
        assert_eq!(backoff_secs(3), 40);
        assert_eq!(backoff_secs(4), 80);
        assert_eq!(backoff_secs(5), 160);
        assert_eq!(backoff_secs(6), 300); // 320 封顶
        assert_eq!(backoff_secs(7), 300);
        assert_eq!(backoff_secs(u32::MAX), 300); // 大 attempt 不溢出
    }

    /* ---------- plan_uid_state ---------- */

    #[test]
    fn uid_state_first_connect_builds_baseline() {
        // 首连：从 UIDNEXT - 1 起步（现存邮件不导入），需写库
        assert_eq!(plan_uid_state(None, 100, 42), (41, true));
    }

    #[test]
    fn uid_state_same_validity_resumes() {
        assert_eq!(plan_uid_state(Some((100, 55)), 100, 60), (55, false));
    }

    #[test]
    fn uid_state_validity_change_rebaselines() {
        // UIDVALIDITY 变化：旧 UID 命名空间作废，重新只建基线
        assert_eq!(plan_uid_state(Some((100, 55)), 200, 60), (59, true));
    }

    #[test]
    fn uid_state_zero_uid_next_saturates() {
        assert_eq!(plan_uid_state(None, 100, 0), (0, true));
    }
}
