//! IMAP 客户端模块：基于 imap crate（同步阻塞式），TLS 复用 mail.rs 的 rustls 栈。
//!
//! 与 POP3（mail.rs）只能「全量列表 + UIDL 去重」不同，IMAP 以 UID 做增量同步
//! （UIDVALIDITY 判定基线是否失效、UID 单调递增），并支持 IDLE 推送（RFC 2177）：
//! 一轮 IDLE 阻塞至新邮件到达或 keepalive 到期，取代高频轮询。
//!
//! 两个关键实现细节：
//! - `UID SEARCH UID n:*` 的 `*` 使区间在邮箱非空时至少匹配最后一封邮件
//!   （max_uid 已是最新时仍会返回它），因此 search_newer_than 必须在客户端
//!   再过滤一次 `uid <= max_uid`，否则同一封邮件会被反复当成新邮件；
//! - IDLE 长等待与 15s IO_TIMEOUT 冲突：等待期间 socket 读超时经 imap crate 的
//!   SetReadTimeout 精确设为 keepalive（到期即醒、正常结束本轮），等待结束后
//!   恢复 IO_TIMEOUT；socket 断开等 IO 错误原样上报，由调用方走重连。

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use imap::extensions::idle::SetReadTimeout;

use crate::mail::{tls_wrap, IO_TIMEOUT};

/// SELECT INBOX 返回的邮箱元信息；UID 增量去重基线依赖 uid_validity / uid_next。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MailboxInfo {
    pub uid_validity: u32,
    pub uid_next: u32,
    pub exists: u32,
}

/// 会话字节流：明文 TcpStream 或 rustls TLS 流。
///
/// 为什么按形态分枚举而不是「Box<dyn ReadWrite> + try_clone 句柄」：imap crate
/// 带超时的 IDLE 等待（Handle::wait_with_timeout）要求流类型实现其
/// SetReadTimeout trait；而 Windows 上 SO_RCVTIMEO 是句柄级状态——在
/// try_clone 出的句柄上 set_read_timeout 不影响原句柄的阻塞读（实测仍会
/// 一直阻塞）。因此必须直接对「真正执行读的那个句柄」调超时：明文是
/// TcpStream 本身，TLS 是 rustls StreamOwned 内部的 sock（pub 字段）。
enum IdleStream {
    Plain(TcpStream),
    // StreamOwned 体积大，装箱保持枚举紧凑
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>),
}

impl IdleStream {
    /// 真正执行读操作的 socket 句柄（TLS 时取 rustls 内部的 TcpStream）
    fn socket_mut(&mut self) -> &mut TcpStream {
        match self {
            Self::Plain(s) => s,
            Self::Tls(s) => &mut s.sock,
        }
    }
}

impl Read for IdleStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            Self::Tls(s) => s.read(buf),
        }
    }
}

impl Write for IdleStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(buf),
            Self::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }
}

impl SetReadTimeout for IdleStream {
    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> imap::Result<()> {
        // wait_with_timeout 等待期间传 Some(keepalive)——必须精确生效，否则
        // keepalive 到期检测会被拖晚；等待结束时传 None——映射回 IO_TIMEOUT
        // 而非「永不超时」，避免 DONE 收尾阶段在半开连接上无限阻塞
        let effective = timeout.unwrap_or(IO_TIMEOUT);
        // 必须点名固有方法：接收者是 &mut TcpStream 时，方法解析会先命中本 trait
        // （SetReadTimeout，返回 imap::Result）而非 TcpStream 固有的同名方法
        TcpStream::set_read_timeout(self.socket_mut(), Some(effective)).map_err(imap::Error::Io)
    }
}

/// 一条已登录的 IMAP 会话（INBOX 尚未选中）。
pub struct ImapSession {
    session: imap::Session<IdleStream>,
}

impl ImapSession {
    /// TCP 建连 → 设 IO_TIMEOUT 读写超时 → use_tls 时 TLS 握手 → 读 greeting → LOGIN。
    /// 失败返回中文错误串（调用方会再经 i18n 包装）。
    pub fn connect(
        host: &str,
        port: i64,
        use_tls: bool,
        username: &str,
        password: &str,
    ) -> Result<Self, String> {
        let host = host.trim();
        let port = u16::try_from(port).map_err(|_| format!("端口无效: {port}"))?;
        let tcp = TcpStream::connect((host, port)).map_err(|e| format!("无法连接服务器: {e}"))?;
        tcp.set_read_timeout(Some(IO_TIMEOUT))
            .and_then(|_| tcp.set_write_timeout(Some(IO_TIMEOUT)))
            .map_err(|e| format!("设置超时失败: {e}"))?;

        let stream = if use_tls {
            IdleStream::Tls(Box::new(tls_wrap(tcp, host)?))
        } else {
            IdleStream::Plain(tcp)
        };
        let mut client = imap::Client::new(stream);
        client
            .read_greeting()
            .map_err(|e| format!("读取服务器问候失败: {e}"))?;
        let session = client
            .login(username, password)
            .map_err(|(e, _)| format!("登录失败: {e}"))?;
        Ok(Self { session })
    }

    /// CAPABILITY 探测（登录后），返回是否支持 IDLE。
    pub fn has_idle(&mut self) -> Result<bool, String> {
        let caps = self
            .session
            .capabilities()
            .map_err(|e| format!("读取服务器能力失败: {e}"))?;
        Ok(caps.has_str("IDLE"))
    }

    /// SELECT INBOX；服务器缺失 UIDVALIDITY/UIDNEXT 时报错（增量去重基线依赖它们）。
    pub fn select_inbox(&mut self) -> Result<MailboxInfo, String> {
        let mailbox = self
            .session
            .select("INBOX")
            .map_err(|e| format!("选择收件箱失败: {e}"))?;
        let uid_validity = mailbox
            .uid_validity
            .ok_or_else(|| "服务器未返回 UIDVALIDITY，无法建立同步基线".to_string())?;
        let uid_next = mailbox
            .uid_next
            .ok_or_else(|| "服务器未返回 UIDNEXT，无法建立同步基线".to_string())?;
        Ok(MailboxInfo {
            uid_validity,
            uid_next,
            exists: mailbox.exists,
        })
    }

    /// UID SEARCH UID {max_uid+1}:*，返回升序 UID 列表。
    ///
    /// RFC 3501 的 UID 区间陷阱：`n:*` 中的 `*` 使区间在邮箱非空时至少匹配最后
    /// 一封邮件，max_uid 已是最大 UID 时搜索仍会返回它——必须客户端过滤，否则
    /// 同一封邮件被反复当成新邮件。max_uid 为 u32::MAX 时 +1 会溢出，直接判无新邮件。
    pub fn search_newer_than(&mut self, max_uid: u32) -> Result<Vec<u32>, String> {
        let Some(from) = max_uid.checked_add(1) else {
            return Ok(Vec::new());
        };
        let found = self
            .session
            .uid_search(format!("UID {from}:*"))
            .map_err(|e| format!("搜索新邮件失败: {e}"))?;
        let mut uids: Vec<u32> = found.into_iter().filter(|&uid| uid > max_uid).collect();
        uids.sort_unstable();
        Ok(uids)
    }

    /// UID FETCH {uid} (BODY.PEEK[])，返回邮件原文字节；PEEK 避免置 \Seen。
    pub fn fetch_body(&mut self, uid: u32) -> Result<Vec<u8>, String> {
        let fetches = self
            .session
            .uid_fetch(uid.to_string(), "BODY.PEEK[]")
            .map_err(|e| format!("拉取邮件失败 (UID {uid}): {e}"))?;
        let body = fetches
            .iter()
            .find_map(|f| f.body())
            .ok_or_else(|| format!("服务器未返回邮件内容 (UID {uid})"))?;
        Ok(body.to_vec())
    }

    /// IDLE 一轮：阻塞至服务器推送或 keepalive 到期后自动 DONE 返回 Ok；
    /// 断线等 IO 错误返回 Err（调用方走重连）。
    ///
    /// 读超时处理：wait_with_timeout 经 SetReadTimeout 把读超时精确设为
    /// keepalive——到期即醒（TimedOut）、推送提前唤醒（MailboxChanged），两者
    /// 对本模块都是 Ok；等待结束时 crate 会传 None，IdleStream 的 SetReadTimeout
    /// 实现把它恢复为 IO_TIMEOUT，因此无论本轮成败，后续普通命令（以及 DONE
    /// 收尾）都不受 IDLE 阶段超时设置的影响。
    pub fn idle_wait(&mut self, keepalive: Duration) -> Result<(), String> {
        let handle = self
            .session
            .idle()
            .map_err(|e| format!("进入 IDLE 失败: {e}"))?;
        handle
            .wait_with_timeout(keepalive)
            .map_err(|e| format!("IDLE 等待中断: {e}"))?;
        Ok(())
    }

    /// LOGOUT，忽略结果（连接收尾用）。
    pub fn logout(&mut self) {
        let _ = self.session.logout();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::thread::JoinHandle;
    use std::time::Instant;

    /// mock IMAP 服务器：真实 TcpListener + 脚本线程，按标签协议应答
    struct MockServer {
        port: u16,
        thread: JoinHandle<Result<(), String>>,
    }

    impl MockServer {
        /// 起监听、发 greeting，然后把连接交给脚本闭包交互。
        /// 服务器侧读超时 5s：脚本异常时以 Err 收场而非挂死整个测试。
        fn start(
            script: impl FnOnce(&mut BufReader<TcpStream>) -> Result<(), String> + Send + 'static,
        ) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let thread = std::thread::spawn(move || -> Result<(), String> {
                let (stream, _) = listener.accept().map_err(|e| e.to_string())?;
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .map_err(|e| e.to_string())?;
                let mut reader = BufReader::new(stream);
                write_raw(reader.get_mut(), "* OK mock IMAP4rev1 ready")?;
                script(&mut reader)
            });
            MockServer { port, thread }
        }

        /// 客户端断言结束后收尾：脚本失败（含收到意外命令）在此暴露
        fn join(self) {
            self.thread.join().unwrap().unwrap();
        }
    }

    /// 写一行 CRLF 结尾的响应并 flush
    fn write_raw(stream: &mut TcpStream, line: &str) -> Result<(), String> {
        stream
            .write_all(line.as_bytes())
            .and_then(|_| stream.write_all(b"\r\n"))
            .and_then(|_| stream.flush())
            .map_err(|e| format!("服务器写入失败: {e}"))
    }

    /// 读一条客户端命令，拆出标签与命令体（DONE 无标签，整体进命令体）
    fn read_command(reader: &mut BufReader<TcpStream>) -> Result<(String, String), String> {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Err("客户端提前断开".to_string());
        }
        let line = line.trim_end().to_string();
        match line.split_once(' ') {
            Some((tag, rest)) => Ok((tag.to_string(), rest.to_string())),
            None => Ok((String::new(), line)),
        }
    }

    /// 期望一条命令并回 tagged OK（可带 untagged 行）
    fn expect_command(
        reader: &mut BufReader<TcpStream>,
        expect: &str,
        untagged: &[&str],
    ) -> Result<(), String> {
        let (tag, cmd) = read_command(reader)?;
        if cmd != expect {
            return Err(format!("期望 [{expect}]，实际 [{cmd}]"));
        }
        for line in untagged {
            write_raw(reader.get_mut(), line)?;
        }
        write_raw(reader.get_mut(), &format!("{tag} OK done"))
    }

    /// 应答一次成功的 LOGIN（imap crate 会把登录参数按 IMAP quoted-string 加引号）
    fn expect_login(reader: &mut BufReader<TcpStream>) -> Result<(), String> {
        expect_command(reader, "LOGIN \"alice@example.com\" \"secret\"", &[])
    }

    /// 应答一次 SELECT INBOX（含 FLAGS/EXISTS/RECENT/UIDVALIDITY/UIDNEXT；
    /// imap crate 会把邮箱名按 quoted-string 加引号）
    fn expect_select(reader: &mut BufReader<TcpStream>) -> Result<(), String> {
        expect_command(
            reader,
            "SELECT \"INBOX\"",
            &[
                "* FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft)",
                "* 3 EXISTS",
                "* 1 RECENT",
                "* OK [UIDVALIDITY 1680000000] UIDs valid",
                "* OK [UIDNEXT 42] Predicted next UID",
            ],
        )
    }

    /// 期望 IDLE：回 "+ idling" 后交给闭包决定推送/沉默，最后读 DONE 并收尾
    fn expect_idle(
        reader: &mut BufReader<TcpStream>,
        during: impl FnOnce(&mut BufReader<TcpStream>) -> Result<(), String>,
    ) -> Result<(), String> {
        let (tag, cmd) = read_command(reader)?;
        if cmd != "IDLE" {
            return Err(format!("期望 [IDLE]，实际 [{cmd}]"));
        }
        write_raw(reader.get_mut(), "+ idling")?;
        during(reader)?;
        let (_, done) = read_command(reader)?;
        if done != "DONE" {
            return Err(format!("期望 [DONE]，实际 [{done}]"));
        }
        write_raw(reader.get_mut(), &format!("{tag} OK IDLE terminated"))
    }

    fn connect_to(port: u16) -> ImapSession {
        ImapSession::connect("127.0.0.1", port as i64, false, "alice@example.com", "secret")
            .unwrap()
    }

    #[test]
    fn login_and_idle_capability() {
        let server = MockServer::start(|reader| {
            expect_login(reader)?;
            expect_command(
                reader,
                "CAPABILITY",
                &["* CAPABILITY IMAP4rev1 IDLE UIDPLUS"],
            )
        });
        let mut session = connect_to(server.port);
        assert!(session.has_idle().unwrap());
        server.join();
    }

    #[test]
    fn login_rejected_returns_err() {
        let server = MockServer::start(|reader| {
            let (tag, _cmd) = read_command(reader)?;
            write_raw(reader.get_mut(), &format!("{tag} NO invalid credentials"))
        });
        let result =
            ImapSession::connect("127.0.0.1", server.port as i64, false, "alice@example.com", "wrong");
        let err = match result {
            Ok(_) => panic!("LOGIN 被拒时不应连接成功"),
            Err(e) => e,
        };
        assert!(err.contains("登录失败"), "错误缺少上下文: {err}");
        server.join();
    }

    #[test]
    fn capability_without_idle_returns_false() {
        let server = MockServer::start(|reader| {
            expect_login(reader)?;
            expect_command(reader, "CAPABILITY", &["* CAPABILITY IMAP4rev1 STARTTLS"])
        });
        let mut session = connect_to(server.port);
        assert!(!session.has_idle().unwrap());
        server.join();
    }

    #[test]
    fn select_inbox_returns_mailbox_info() {
        let server = MockServer::start(|reader| {
            expect_login(reader)?;
            expect_select(reader)
        });
        let mut session = connect_to(server.port);
        let info = session.select_inbox().unwrap();
        assert_eq!(
            info,
            MailboxInfo {
                uid_validity: 1680000000,
                uid_next: 42,
                exists: 3,
            }
        );
        server.join();
    }

    #[test]
    fn search_newer_than_filters_boundary_uid() {
        // 服务器对 UID 6:* 返回乱序的 9 5 7：5 == max_uid 正是 n:* 区间陷阱的
        // 必然命中（* 至少匹配最后一封），客户端必须过滤它并升序输出
        let server = MockServer::start(|reader| {
            expect_login(reader)?;
            expect_select(reader)?;
            expect_command(reader, "UID SEARCH UID 6:*", &["* SEARCH 9 5 7"])
        });
        let mut session = connect_to(server.port);
        session.select_inbox().unwrap();
        assert_eq!(session.search_newer_than(5).unwrap(), vec![7, 9]);
        // u32::MAX 时 +1 溢出：直接判无新邮件，不得向服务器发命令
        // （脚本已结束、socket 已关，一旦真的发命令必然报错）
        assert_eq!(
            session.search_newer_than(u32::MAX).unwrap(),
            Vec::<u32>::new()
        );
        server.join();
    }

    #[test]
    fn fetch_body_returns_raw_bytes() {
        const RAW_MAIL: &str =
            "From: <service@example.com>\r\nSubject: code\r\n\r\n您的验证码是 482913\r\n";
        let server = MockServer::start(|reader| {
            expect_login(reader)?;
            expect_select(reader)?;
            let (tag, cmd) = read_command(reader)?;
            if cmd != "UID FETCH 7 BODY.PEEK[]" {
                return Err(format!("期望 [UID FETCH 7 BODY.PEEK[]]，实际 [{cmd}]"));
            }
            // literal 语法：{n} 行的 CRLF 后紧跟 n 字节原文，再以 ")" 收尾
            write_raw(
                reader.get_mut(),
                &format!("* 1 FETCH (UID 7 BODY[] {{{}}}", RAW_MAIL.len()),
            )?;
            reader
                .get_mut()
                .write_all(RAW_MAIL.as_bytes())
                .map_err(|e| e.to_string())?;
            write_raw(reader.get_mut(), ")")?;
            write_raw(reader.get_mut(), &format!("{tag} OK FETCH completed"))
        });
        let mut session = connect_to(server.port);
        session.select_inbox().unwrap();
        let body = session.fetch_body(7).unwrap();
        assert_eq!(body, RAW_MAIL.as_bytes());
        server.join();
    }

    #[test]
    fn idle_wakeup_and_second_round() {
        // 服务器推送 * EXISTS 唤醒客户端；DONE 后连接可复用，能再发起第二轮 IDLE
        let server = MockServer::start(|reader| {
            expect_login(reader)?;
            expect_select(reader)?;
            for round in 0..2 {
                expect_idle(reader, |reader| {
                    std::thread::sleep(Duration::from_millis(100));
                    write_raw(reader.get_mut(), &format!("* {} EXISTS", 2 + round))
                })?;
            }
            Ok(())
        });
        let mut session = connect_to(server.port);
        session.select_inbox().unwrap();
        let start = Instant::now();
        // keepalive 远大于推送延迟：返回即推送唤醒而非超时
        session.idle_wait(Duration::from_secs(10)).unwrap();
        session.idle_wait(Duration::from_secs(10)).unwrap();
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "两轮 IDLE 应由推送唤醒（~200ms），实际耗时 {:?}",
            start.elapsed()
        );
        server.join();
    }

    #[test]
    fn idle_broken_connection_returns_err() {
        // 服务器在 IDLE 中途直接断开 socket
        let server = MockServer::start(|reader| {
            expect_login(reader)?;
            expect_select(reader)?;
            let (_tag, cmd) = read_command(reader)?;
            if cmd != "IDLE" {
                return Err(format!("期望 [IDLE]，实际 [{cmd}]"));
            }
            write_raw(reader.get_mut(), "+ idling")?;
            Ok(()) // 闭包返回即 drop socket，客户端读到 EOF
        });
        let mut session = connect_to(server.port);
        session.select_inbox().unwrap();
        let err = session.idle_wait(Duration::from_secs(10)).unwrap_err();
        assert!(err.contains("IDLE 等待中断"), "错误缺少上下文: {err}");
        server.join();
    }

    #[test]
    fn idle_keepalive_expiry_returns_ok() {
        // 服务器不推送：keepalive 到期后客户端自动 DONE，一轮正常结束
        let server = MockServer::start(|reader| {
            expect_login(reader)?;
            expect_select(reader)?;
            expect_idle(reader, |_reader| Ok(())) // 沉默，等客户端超时 DONE
        });
        let mut session = connect_to(server.port);
        session.select_inbox().unwrap();
        let start = Instant::now();
        session.idle_wait(Duration::from_millis(300)).unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(250) && elapsed < Duration::from_secs(3),
            "keepalive 300ms 到期应约 300ms 返回，实际 {elapsed:?}"
        );
        server.join();
    }
}
