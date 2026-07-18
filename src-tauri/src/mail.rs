//! POP3 客户端与邮件解析。
//!
//! POP3 协议简单（USER/PASS/STAT/UIDL/RETR/QUIT，+OK/-ERR 状态行，多行响应以
//! 单独的 "." 行终止、行首 "." 需去转义），因此手写实现而非引入 POP3 库；
//! MIME 解析（base64/QP/编码字/多部分）边界情况多，交给 mail-parser。
//!
//! 客户端泛型化于字节流：生产环境是 TLS 或明文 TcpStream，测试是内存 TCP 连接，
//! 因此协议逻辑无需 TLS 即可完整单测。

use std::io::{BufRead, BufReader, Read, Write};
use std::sync::Arc;
use std::time::Duration;

use crate::settings::EmailSettings;

/// 同时满足读写的字节流（TlsStream / TcpStream / 测试流）。
pub trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

/// 连接与读写超时：POP3 服务器挂机时避免监听线程永久阻塞。
/// pub(crate)：IMAP 连接（imap_client.rs）复用同一超时常量。
pub(crate) const IO_TIMEOUT: Duration = Duration::from_secs(15);

/// 一封解析后的邮件，仅保留验证码提取与入库所需字段。
#[derive(Clone, Debug)]
pub struct ParsedMail {
    /// From 头的显示名或地址
    pub sender: Option<String>,
    /// 主题 + 正文纯文本（HTML 已去标签），直接交给 parser::extract_code
    pub text: String,
    /// Date 头换算的 unix 毫秒；解析失败为 None，调用方回退当前时间
    pub received_at: Option<i64>,
}

/// POP3 客户端（RFC 1939 子集）。
pub struct Pop3Client<S: Read + Write> {
    reader: BufReader<S>,
}

impl<S: Read + Write> Pop3Client<S> {
    /// 包装一条已建立的连接，读取服务器 greeting（必须以 +OK 开头）。
    pub fn from_stream(stream: S) -> Result<Self, String> {
        let mut client = Self {
            reader: BufReader::new(stream),
        };
        client.read_status()?;
        Ok(client)
    }

    /// USER + PASS 登录。密码含空格不受支持（POP3 参数按空格分隔）。
    pub fn login(&mut self, user: &str, pass: &str) -> Result<(), String> {
        self.command(&format!("USER {user}"))?;
        self.command(&format!("PASS {pass}"))?;
        Ok(())
    }

    /// STAT：返回（邮件数, 总字节数）。
    pub fn stat(&mut self) -> Result<(u64, u64), String> {
        let line = self.command("STAT")?;
        let mut parts = line.split_whitespace();
        let count = parts
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("STAT 响应无法解析: {line}"))?;
        let size = parts
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("STAT 响应无法解析: {line}"))?;
        Ok((count, size))
    }

    /// UIDL：返回全部邮件的（序号, UIDL）列表。
    pub fn uidl(&mut self) -> Result<Vec<(u32, String)>, String> {
        self.command("UIDL")?;
        let body = self.read_multiline()?;
        let text = String::from_utf8_lossy(&body);
        let mut out = Vec::new();
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let (Some(num), Some(uidl)) = (parts.next(), parts.next()) else {
                continue; // 容忍畸形行，跳过
            };
            if let Ok(num) = num.parse() {
                out.push((num, uidl.to_string()));
            }
        }
        Ok(out)
    }

    /// RETR：取回整封邮件原文（已做 dot 去转义）。
    pub fn retr(&mut self, msg: u32) -> Result<Vec<u8>, String> {
        self.command(&format!("RETR {msg}"))?;
        self.read_multiline()
    }

    /// QUIT：礼貌断开；失败忽略（连接本来就要关了）。
    pub fn quit(&mut self) {
        let _ = self.command("QUIT");
    }

    /// 发送一条命令并读取状态行；-ERR 转为 Err。
    fn command(&mut self, cmd: &str) -> Result<String, String> {
        let stream = self.reader.get_mut();
        stream
            .write_all(cmd.as_bytes())
            .and_then(|_| stream.write_all(b"\r\n"))
            .and_then(|_| stream.flush())
            .map_err(|e| format!("发送命令失败: {e}"))?;
        self.read_status()
    }

    /// 读取一行状态响应。+OK 返回其后的文本，-ERR 返回 Err。
    fn read_status(&mut self) -> Result<String, String> {
        let line = self.read_line()?;
        if let Some(rest) = line.strip_prefix("+OK") {
            Ok(rest.trim().to_string())
        } else if let Some(rest) = line.strip_prefix("-ERR") {
            Err(format!("服务器拒绝: {}", rest.trim()))
        } else {
            Err(format!("无法识别的响应: {line}"))
        }
    }

    /// 读取一行（去除 CRLF），按 lossy UTF-8 处理（状态行应为 ASCII）。
    fn read_line(&mut self) -> Result<String, String> {
        let mut buf = Vec::new();
        self.reader
            .read_until(b'\n', &mut buf)
            .map_err(|e| format!("读取响应失败: {e}"))?;
        if buf.is_empty() {
            return Err("连接被服务器关闭".to_string());
        }
        while matches!(buf.last(), Some(b'\n') | Some(b'\r')) {
            buf.pop();
        }
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    /// 读取多行响应：单独的 "." 行终止，行首 "." 去掉一个转义点。
    /// 保留原始字节（邮件正文可能不是 UTF-8），行间补回 CRLF。
    fn read_multiline(&mut self) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        loop {
            let mut buf = Vec::new();
            let n = self
                .reader
                .read_until(b'\n', &mut buf)
                .map_err(|e| format!("读取响应失败: {e}"))?;
            if n == 0 {
                return Err("连接被服务器关闭".to_string());
            }
            while matches!(buf.last(), Some(b'\n') | Some(b'\r')) {
                buf.pop();
            }
            if buf == b"." {
                return Ok(out);
            }
            if buf.first() == Some(&b'.') {
                buf.remove(0); // dot-stuffing 去转义
            }
            if !out.is_empty() {
                out.extend_from_slice(b"\r\n");
            }
            out.extend_from_slice(&buf);
        }
    }
}

/// 按配置建立 POP3 连接（TCP + 可选 TLS + greeting），尚未登录。
pub fn connect(cfg: &EmailSettings) -> Result<Pop3Client<Box<dyn ReadWrite>>, String> {
    use std::net::TcpStream;

    let host = cfg.host.trim();
    let port = u16::try_from(cfg.port).map_err(|_| format!("端口无效: {}", cfg.port))?;
    let tcp = TcpStream::connect((host, port)).map_err(|e| format!("无法连接服务器: {e}"))?;
    tcp.set_read_timeout(Some(IO_TIMEOUT))
        .and_then(|_| tcp.set_write_timeout(Some(IO_TIMEOUT)))
        .map_err(|e| format!("设置超时失败: {e}"))?;

    let stream: Box<dyn ReadWrite> = if cfg.use_tls {
        Box::new(tls_wrap(tcp, host)?)
    } else {
        Box::new(tcp)
    };
    Pop3Client::from_stream(stream)
}

/// 在 TCP 上建立隐式 TLS（POP3S，995 端口惯例）。
/// pub(crate)：IMAP over SSL（993 端口惯例）经 imap_client.rs 复用同一 rustls 栈。
pub(crate) fn tls_wrap(
    tcp: std::net::TcpStream,
    host: &str,
) -> Result<rustls::StreamOwned<rustls::ClientConnection, std::net::TcpStream>, String> {
    use rustls::pki_types::ServerName;

    // 系统根证书（Windows 经 schannel 证书库）
    let native = rustls_native_certs::load_native_certs();
    let mut roots = rustls::RootCertStore::empty();
    for cert in native.certs {
        let _ = roots.add(cert);
    }
    if roots.is_empty() {
        return Err("系统根证书库为空，无法验证服务器证书".to_string());
    }

    // 显式 ring provider：不依赖进程级默认 provider（安装时机不可控）
    let config = rustls::ClientConfig::builder_with_provider(
        rustls::crypto::ring::default_provider().into(),
    )
    .with_safe_default_protocol_versions()
    .map_err(|e| format!("TLS 配置失败: {e}"))?
    .with_root_certificates(roots)
    .with_no_client_auth();

    let name = ServerName::try_from(host.to_string())
        .map_err(|_| format!("服务器地址无效: {host}"))?;
    let conn = rustls::ClientConnection::new(Arc::new(config), name)
        .map_err(|e| format!("TLS 初始化失败: {e}"))?;
    Ok(rustls::StreamOwned::new(conn, tcp))
}

/// 解析邮件原文为结构化字段；完全无法解析返回 None。
pub fn parse_mail(raw: &[u8]) -> Option<ParsedMail> {
    let msg = mail_parser::MessageParser::default().parse(raw)?;

    let subject = msg.subject().unwrap_or("").to_string();

    let sender = msg
        .from()
        .and_then(|f| f.as_list())
        .and_then(|list| list.first())
        .map(|addr| {
            addr.name
                .as_deref()
                .or(addr.address.as_deref())
                .unwrap_or("")
                .to_string()
        })
        .filter(|s| !s.is_empty());

    // 正文优先真正的 text/plain 部分；注意 mail-parser 会把 text/html 件同时列入
    // text_body 与 html_body（body_text 会对 HTML 做自动纯文本转换，块边界不插空格，
    // 会把 </h1><p> 两侧数字粘住），因此首个 text 件同时也是 HTML 件时改用本模块的
    // html_to_text（块级标签转空格）；multipart/alternative 时 text/plain 件优先
    let body = match msg.text_body.first() {
        Some(id) if !msg.html_body.contains(id) => {
            msg.body_text(0).map(|t| t.into_owned()).unwrap_or_default()
        }
        _ => msg.body_html(0).map(|h| html_to_text(&h)).unwrap_or_default(),
    };

    let received_at = msg.date().map(|d| d.to_timestamp() * 1000);

    let text = format!("{subject}\n{body}");
    Some(ParsedMail {
        sender,
        text,
        received_at,
    })
}

/// 极简 HTML → 纯文本：去 script/style 块、块级标签转空格、解常见实体、折叠空白。
/// 只为验证码提取服务，不追求排版还原。
fn html_to_text(html: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;

    static BLOCK_RE: OnceLock<Regex> = OnceLock::new();
    static BLOCKTAG_RE: OnceLock<Regex> = OnceLock::new();
    static TAG_RE: OnceLock<Regex> = OnceLock::new();
    static WS_RE: OnceLock<Regex> = OnceLock::new();

    // regex crate 不支持反向引用（\1），script/style 用交替写成一条
    let block_re = BLOCK_RE
        .get_or_init(|| Regex::new(r"(?is)<script[^>]*>.*?</script>|<style[^>]*>.*?</style>").unwrap());
    // 块级标签是天然的词边界（否则 <h1>886655</h1><p>10 分钟</p> 会粘出假数字串）；
    // 行内标签直接删除，还原被样式拆开的验证码（<b>482</b><b>913</b>）
    let blocktag_re = BLOCKTAG_RE.get_or_init(|| {
        Regex::new(r"(?is)</?(?:p|div|br|hr|tr|td|th|li|ul|ol|table|h[1-6]|blockquote|section|article|header|footer)[^>]*>")
            .unwrap()
    });
    let tag_re = TAG_RE.get_or_init(|| Regex::new(r"(?s)<[^>]+>").unwrap());
    let ws_re = WS_RE.get_or_init(|| Regex::new(r"\s+").unwrap());

    let no_script = block_re.replace_all(html, " ");
    let blocked = blocktag_re.replace_all(&no_script, " ");
    let no_tags = tag_re.replace_all(&blocked, "");
    let unescaped = no_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    ws_re.replace_all(&unescaped, " ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};

    fn client_to(port: u16) -> Pop3Client<TcpStream> {
        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        Pop3Client::from_stream(stream).unwrap()
    }

    #[test]
    fn rejects_bad_greeting() {
        // greeting 阶段就回 -ERR：连接应立即失败并带服务器原因
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(b"-ERR service not available\r\n")
                .unwrap();
        });
        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let result = Pop3Client::from_stream(stream);
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("greeting 为 -ERR 时不应成功"),
        };
        assert!(err.contains("service not available"));
        handle.join().unwrap();
    }

    #[test]
    fn full_session_flow() {
        // 覆盖完整会话：greeting + USER/PASS/STAT/UIDL/RETR(含 dot 转义)/QUIT
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || -> Result<(), String> {
            let (stream, _) = listener.accept().map_err(|e| e.to_string())?;
            let mut reader = BufReader::new(stream);
            reader
                .get_mut()
                .write_all(b"+OK mock POP3 ready\r\n")
                .map_err(|e| e.to_string())?;

            let script: Vec<(&str, Vec<&str>)> = vec![
                ("USER alice@example.com", vec!["+OK user accepted"]),
                ("PASS secret", vec!["+OK mailbox locked and ready"]),
                ("STAT", vec!["+OK 2 1024"]),
                (
                    "UIDL",
                    vec!["+OK", "1 uidl-aaa", "2 uidl-bbb", "."],
                ),
                (
                    "RETR 2",
                    vec![
                        "+OK message follows",
                        "Subject: test",
                        "",
                        "..dot-prefixed line",
                        ".",
                    ],
                ),
                ("QUIT", vec!["+OK bye"]),
            ];
            for (expect, responses) in script {
                let mut line = String::new();
                if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
                    return Err(format!("客户端提前断开（期望 {expect}）"));
                }
                let got = line.trim_end().to_string();
                if got != expect {
                    return Err(format!("期望 [{expect}]，实际 [{got}]"));
                }
                for resp in responses {
                    let stream = reader.get_mut();
                    stream
                        .write_all(resp.as_bytes())
                        .and_then(|_| stream.write_all(b"\r\n"))
                        .map_err(|e| e.to_string())?;
                }
                reader.get_mut().flush().map_err(|e| e.to_string())?;
            }
            Ok(())
        });

        let mut client = client_to(port);
        client.login("alice@example.com", "secret").unwrap();
        assert_eq!(client.stat().unwrap(), (2, 1024));
        assert_eq!(
            client.uidl().unwrap(),
            vec![(1, "uidl-aaa".to_string()), (2, "uidl-bbb".to_string())]
        );
        let raw = client.retr(2).unwrap();
        let text = String::from_utf8(raw).unwrap();
        assert_eq!(text, "Subject: test\r\n\r\n.dot-prefixed line");
        client.quit();

        server.join().unwrap().unwrap();
    }

    #[test]
    fn login_rejected_by_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            reader
                .get_mut()
                .write_all(b"+OK mock POP3 ready\r\n")
                .unwrap();
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            reader.get_mut().write_all(b"+OK\r\n").unwrap();
            line.clear();
            reader.read_line(&mut line).unwrap();
            reader
                .get_mut()
                .write_all(b"-ERR authentication failed\r\n")
                .unwrap();
        });

        let mut client = client_to(port);
        let err = client.login("alice@example.com", "wrong").unwrap_err();
        assert!(err.contains("authentication failed"));
        server.join().unwrap();
    }

    /* ---------- 邮件解析 ---------- */

    #[test]
    fn parses_plain_text_mail() {
        let raw = "From: 阿里云 <noreply@aliyun.com>\r\n\
                   Subject: 登录验证\r\n\
                   Date: Mon, 01 Jan 2024 12:00:00 +0800\r\n\
                   Content-Type: text/plain; charset=utf-8\r\n\
                   Content-Transfer-Encoding: 8bit\r\n\
                   \r\n\
                   您的验证码是 482913，5 分钟内有效。\r\n";
        let mail = parse_mail(raw.as_bytes()).unwrap();
        assert!(mail.text.starts_with("登录验证"));
        assert!(mail.sender.as_deref().unwrap().contains("阿里云"));
        assert!(mail.text.contains("482913"));
        // 2024-01-01 12:00:00 +0800 = 1704081600 秒
        assert_eq!(mail.received_at, Some(1_704_081_600_000));
    }

    #[test]
    fn parses_base64_body_and_encoded_subject() {
        // Subject 为 UTF-8 编码字「验证码通知」；正文为 base64 的英文验证码邮件
        let raw = concat!(
            "From: Google <no-reply@accounts.google.com>\r\n",
            "Subject: =?UTF-8?B?6aqM6K+B56CB6YCa55+l?=\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n",
            "Content-Transfer-Encoding: base64\r\n",
            "\r\n",
            "WW91ciB2ZXJpZmljYXRpb24gY29kZSBpcyA2NTQzMjEsIHZhbGlkIGZvciAxMCBtaW51dGVzLg==\r\n",
        );
        let mail = parse_mail(raw.as_bytes()).unwrap();
        assert!(mail.text.starts_with("验证码通知"));
        assert!(mail.text.contains("654321"));
    }

    #[test]
    fn falls_back_to_html_body() {
        let raw = concat!(
            "From: <service@example.com>\r\n",
            "Subject: =?UTF-8?B?6aqM6K+B56CB?=\r\n",
            "Content-Type: text/html; charset=utf-8\r\n",
            "\r\n",
            "<html><head><style>.x{color:red}</style></head>",
            "<body><p>您的验证码是 <b>886655</b>&nbsp;，10 分钟内有效</p>",
            "<script>alert(1)</script></body></html>\r\n",
        );
        let mail = parse_mail(raw.as_bytes()).unwrap();
        assert!(mail.text.contains("886655"));
        assert!(!mail.text.contains("alert"));
        assert!(!mail.text.contains("color:red"));
    }

    #[test]
    fn rejects_garbage() {
        // 极端输入不 panic：要么解析失败，要么得不到任何可用文本
        let result = parse_mail(b"\x00\x01\x02\x03 not a mail");
        assert!(result.map(|m| m.text.trim().is_empty()).unwrap_or(true));
    }

    #[test]
    fn html_to_text_basics() {
        // 行内标签删除（还原被拆开的数字），块级标签转空格（隔开上下文数字）
        assert_eq!(html_to_text("<p>a<b>b</b>c</p>"), "abc");
        assert_eq!(html_to_text("<b>482</b><b>913</b>"), "482913");
        assert_eq!(html_to_text("<h1>886655</h1><p>10 分钟</p>"), "886655 10 分钟");
        assert_eq!(html_to_text("a&nbsp;b &amp; c"), "a b & c");
    }

    /// 端到端：原始邮件字节 → 解析 → 提取验证码（中英 + HTML 三种形态）
    #[test]
    fn end_to_end_code_extraction() {
        use crate::parser::extract_code;

        let zh = concat!(
            "From: 淘宝网 <service@taobao.com>\r\n",
            "Subject: =?UTF-8?B?55m75b2V6aqM6K+B56CB?=\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n",
            "Content-Transfer-Encoding: 8bit\r\n",
            "\r\n",
            "尊敬的用户，您的验证码为 730251，5 分钟内有效。工作人员不会索取，请勿泄露。\r\n",
        );
        let mail = parse_mail(zh.as_bytes()).unwrap();
        assert_eq!(extract_code(&mail.text), Some("730251".to_string()));

        let en = concat!(
            "From: Google <no-reply@accounts.google.com>\r\n",
            "Subject: Security code\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n",
            "Content-Transfer-Encoding: base64\r\n",
            "\r\n",
            "WW91ciB2ZXJpZmljYXRpb24gY29kZSBpcyA2NTQzMjEsIHZhbGlkIGZvciAxMCBtaW51dGVzLg==\r\n",
        );
        let mail = parse_mail(en.as_bytes()).unwrap();
        assert_eq!(extract_code(&mail.text), Some("654321".to_string()));

        let html = concat!(
            "From: <service@example.com>\r\n",
            "Subject: =?UTF-8?B?6aqM6K+B56CB?=\r\n",
            "Content-Type: text/html; charset=utf-8\r\n",
            "\r\n",
            "<html><body><p>您的验证码是</p><h1 style=\"color:red\">886655</h1>",
            "<p>10 分钟内有效</p></body></html>\r\n",
        );
        let mail = parse_mail(html.as_bytes()).unwrap();
        assert_eq!(extract_code(&mail.text), Some("886655".to_string()));

        // 无验证码的普通邮件不应误识别（含金额、年份）
        let normal = concat!(
            "From: shop@example.com\r\n",
            "Subject: Order shipped\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n",
            "\r\n",
            "Your order #1234567890123 from 2024 has shipped. Total: $59.99.\r\n",
        );
        let mail = parse_mail(normal.as_bytes()).unwrap();
        assert_eq!(extract_code(&mail.text), None);
    }

    /// multipart/alternative：同时含 text/plain 与 text/html 时优先 text/plain
    #[test]
    fn prefers_plain_part_in_alternative() {
        let raw = concat!(
            "Content-Type: multipart/alternative; boundary=bb\r\n",
            "\r\n",
            "--bb\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n",
            "\r\n",
            "Your code is 112233.\r\n",
            "--bb\r\n",
            "Content-Type: text/html; charset=utf-8\r\n",
            "\r\n",
            "<html><body><h1>999999</h1></body></html>\r\n",
            "--bb--\r\n",
        );
        let mail = parse_mail(raw.as_bytes()).unwrap();
        assert!(mail.text.contains("112233"));
        assert!(!mail.text.contains("999999"));
    }
}
