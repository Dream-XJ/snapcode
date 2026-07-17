//! 短信验证码提取器：从通知/短信文本中识别 4~8 位数字验证码。

use regex::Regex;
use std::sync::OnceLock;

/// 规则 1：关键词后 0~12 个非数字字符内的首个 4~8 位数字。
fn keyword_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)(?:验证码|动态密码|动态码|校验码|短信码|verification\s*code|security\s*code|passcode|code)[^\d]{0,12}(\d{4,8})",
        )
        .unwrap()
    })
}

/// 规则 2 用的独立数字串扫描。
fn digit_run_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\d+").unwrap())
}

/// 账号类语境黑名单：数字前 4 个字符内出现即排除。
const CONTEXT_BLACKLIST: [&str; 8] = [
    "尾号", "卡号", "账号", "账户", "单号", "电话", "热线", "客服",
];

const CURRENCY_SYMBOLS: [char; 4] = ['¥', '￥', '$', '€'];

/// 从文本中提取验证码，识别不到返回 None。
pub fn extract_code(text: &str) -> Option<String> {
    // 规则 1：关键词优先
    for caps in keyword_re().captures_iter(text) {
        let m = match caps.get(1) {
            Some(m) => m,
            None => continue,
        };
        // 捕获组之后仍是数字，说明只是长号码段的前缀，跳过
        if text[m.end()..]
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            continue;
        }
        return Some(m.as_str().to_string());
    }

    // 规则 2：独立 4~6 位数字兜底
    for m in digit_run_re().find_iter(text) {
        let digits = m.as_str();
        let len = digits.len();
        if !(4..=6).contains(&len) {
            continue; // 长号码段或过短
        }
        let start = m.start();
        let end = m.end();

        // 前为货币符号（金额）
        if let Some(c) = text[..start].chars().next_back() {
            if CURRENCY_SYMBOLS.contains(&c) {
                continue;
            }
        }
        // 后为小数点/逗号（金额）
        if let Some(c) = text[end..].chars().next() {
            if c == '.' || c == ',' {
                continue;
            }
        }
        // 4 位且 19xx/20xx（年份）
        if len == 4 && (digits.starts_with("19") || digits.starts_with("20")) {
            continue;
        }
        // 前 4 字内出现账号类语境词
        let window_start = text[..start]
            .char_indices()
            .rev()
            .take(4)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        let before = &text[window_start..start];
        if CONTEXT_BLACKLIST.iter().any(|k| before.contains(k)) {
            continue;
        }
        return Some(digits.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::extract_code;

    #[test]
    fn chinese_keyword_codes() {
        assert_eq!(
            extract_code("【支付宝】您的验证码是 123456，5 分钟内有效"),
            Some("123456".to_string())
        );
        assert_eq!(
            extract_code("【微信】验证码：884422，10 分钟内有效"),
            Some("884422".to_string())
        );
        assert_eq!(
            extract_code("您的动态密码为 778899，有效期 5 分钟"),
            Some("778899".to_string())
        );
    }

    #[test]
    fn english_keyword_codes() {
        assert_eq!(
            extract_code("Your Google verification code is 654321"),
            Some("654321".to_string())
        );
        assert_eq!(
            extract_code("Your security code is 0921"),
            Some("0921".to_string())
        );
    }

    #[test]
    fn rejects_non_codes() {
        assert_eq!(
            extract_code("您尾号 1234 的银行卡支出人民币 500.00 元"),
            None
        );
        assert_eq!(extract_code("客服热线 95588 全天服务"), None);
        assert_eq!(extract_code("2024 年新年快乐"), None);
        assert_eq!(extract_code("这是一条没有任何数字的通知"), None);
    }

    #[test]
    fn fallback_without_keyword() {
        assert_eq!(extract_code("登录验证码 3355 请查收"), Some("3355".to_string()));
        // 超长号码段不应截断冒充验证码
        assert_eq!(extract_code("order 1234567890123 shipped"), None);
        // 年份与金额混合
        assert_eq!(extract_code("2023 年消费 ¥1234"), None);
    }
}
