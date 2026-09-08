//! slug：`https://<slug>.playtest.run` 里的那一段。
//!
//! 只能一级（泛域名证书只覆盖一级），DNS 标签规则：小写字母、数字、连字符，
//! 不以连字符开头结尾，1–63 字符。保留名单挡住会被当成官方页面或钓鱼的词。

pub const MAX_LEN: usize = 63;

/// 不允许用户拿到的 slug。品牌词、系统词、支付与社交平台名——钓鱼页最爱用的那些。
pub const RESERVED: &[&str] = &[
    "www", "api", "app", "admin", "login", "signin", "signup", "auth", "oauth", "account",
    "accounts", "mail", "email", "smtp", "imap", "ftp", "ssh", "cdn", "static", "assets",
    "status", "docs", "help", "support", "blog", "dev", "test", "staging", "demo", "console",
    "dashboard", "playtest", "play", "run", "root", "system", "security", "abuse", "report",
    "wechat", "weixin", "wx", "alipay", "taobao", "tmall", "jd", "qq", "tencent", "baidu",
    "douyin", "tiktok", "bilibili", "weibo", "apple", "icloud", "google", "microsoft", "steam",
    "steampowered", "epicgames", "nintendo", "playstation", "xbox", "paypal", "stripe", "visa",
    "mastercard", "unionpay", "bank", "wallet", "verify", "verification", "secure", "update",
    "password", "reset", "recover", "official", "gov", "police",
];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SlugError {
    #[error("名字不能为空")]
    Empty,
    #[error("名字太长：最多 {MAX_LEN} 个字符")]
    TooLong,
    #[error("名字只能用小写字母、数字和连字符，且不能以连字符开头或结尾：{0}")]
    BadChars(String),
    #[error("这个名字保留着，不能用：{0}")]
    Reserved(String),
}

pub fn validate(slug: &str) -> Result<(), SlugError> {
    if slug.is_empty() {
        return Err(SlugError::Empty);
    }
    if slug.len() > MAX_LEN {
        return Err(SlugError::TooLong);
    }
    let bytes = slug.as_bytes();
    let ok_char = |b: u8| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-');
    if !bytes.iter().all(|&b| ok_char(b)) || bytes[0] == b'-' || bytes[bytes.len() - 1] == b'-' {
        return Err(SlugError::BadChars(slug.to_string()));
    }
    if RESERVED.contains(&slug) {
        return Err(SlugError::Reserved(slug.to_string()));
    }
    Ok(())
}

/// 随机 slug 的两份词表：形容词 + 动物，都短、好念、不带歧义。
/// 组合成 `brisk-otter-41`：约 64 × 64 × 90 ≈ 37 万种，够私测阶段用，撞了重抽。
pub const ADJECTIVES: &[&str] = &[
    "brisk", "calm", "bold", "keen", "swift", "quiet", "warm", "cool", "bright", "gentle",
    "lucky", "merry", "nimble", "plucky", "rapid", "sunny", "tidy", "vivid", "witty", "zesty",
    "amber", "azure", "coral", "cyan", "golden", "indigo", "ivory", "jade", "lilac", "maroon",
    "olive", "pearl", "ruby", "sage", "scarlet", "silver", "teal", "violet", "crimson", "cobalt",
    "early", "eager", "fair", "fresh", "glad", "happy", "jolly", "kind", "lively", "mellow",
    "noble", "proud", "quick", "ready", "steady", "sturdy", "tender", "upbeat", "wise", "young",
    "misty", "windy", "snowy", "rainy",
];

pub const ANIMALS: &[&str] = &[
    "otter", "panda", "koala", "lemur", "gecko", "heron", "finch", "robin", "raven", "swan",
    "tiger", "lynx", "puma", "wolf", "fox", "hare", "deer", "moose", "bison", "yak",
    "whale", "dolphin", "seal", "walrus", "manta", "coral", "squid", "crab", "shrimp", "trout",
    "salmon", "tuna", "carp", "eel", "newt", "frog", "toad", "turtle", "iguana", "cobra",
    "eagle", "falcon", "hawk", "owl", "crane", "stork", "pelican", "puffin", "parrot", "macaw",
    "badger", "beaver", "marten", "ferret", "mink", "ermine", "sable", "stoat", "vole", "shrew",
    "camel", "llama", "alpaca", "zebra",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_generated_shape() {
        assert_eq!(validate("brisk-otter-41"), Ok(()));
        assert_eq!(validate("a"), Ok(()));
    }

    #[test]
    fn rejects_bad_shapes() {
        assert_eq!(validate(""), Err(SlugError::Empty));
        assert!(matches!(validate("-abc"), Err(SlugError::BadChars(_))));
        assert!(matches!(validate("abc-"), Err(SlugError::BadChars(_))));
        assert!(matches!(validate("Abc"), Err(SlugError::BadChars(_))));
        assert!(matches!(validate("a.b"), Err(SlugError::BadChars(_))));
        assert_eq!(validate(&"a".repeat(64)), Err(SlugError::TooLong));
    }

    #[test]
    fn rejects_reserved() {
        assert!(matches!(validate("login"), Err(SlugError::Reserved(_))));
        assert!(matches!(validate("wechat"), Err(SlugError::Reserved(_))));
    }

    #[test]
    fn word_lists_are_valid_labels() {
        for w in ADJECTIVES.iter().chain(ANIMALS.iter()) {
            assert_eq!(validate(w), Ok(()), "{w}");
        }
    }
}
