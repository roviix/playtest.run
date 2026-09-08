//! 按 `Host` 分流：根域一页介绍，一级标签是一个作品，其余全部当不存在。
//!
//! 只认一级标签是泛域名证书的约束（DESIGN §4.1）：`a-b.playtest.run` 可以，
//! `a.b.playtest.run` 不行。

use playtest_common::slug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKind {
    /// 后缀本身或 `www.<后缀>`：一页介绍，唯一允许出现品牌站链接的地方。
    Root,
    /// 一个作品。
    Site(String),
    /// 多级、保留名、非法形态，或压根不是我们的域名。
    Unknown,
}

/// 去掉端口。IPv6 字面量（`[::1]:8443`）按方括号切，其余按最后一个冒号切。
pub fn strip_port(authority: &str) -> &str {
    if authority.starts_with('[') {
        return match authority.find(']') {
            Some(i) => &authority[..=i],
            None => authority,
        };
    }
    match authority.rsplit_once(':') {
        Some((host, _)) => host,
        None => authority,
    }
}

/// 端口部分，拼本机链接时要带上（本机边缘在 8443，丢了端口链接就点不开）。
pub fn port_of(authority: &str) -> Option<&str> {
    let after_bracket = if authority.starts_with('[') {
        &authority[authority.find(']')? + 1..]
    } else {
        authority
    };
    after_bracket
        .rsplit_once(':')
        .map(|(_, port)| port)
        .filter(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

pub fn classify(authority: &str, suffix: &str) -> HostKind {
    // 末尾的点是合法的 FQDN 写法，浏览器偶尔会带。
    let host = strip_port(authority)
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.is_empty() {
        return HostKind::Unknown;
    }
    if host == suffix || host == format!("www.{suffix}") {
        return HostKind::Root;
    }
    let Some(label) = host
        .strip_suffix(suffix)
        .and_then(|rest| rest.strip_suffix('.'))
    else {
        return HostKind::Unknown;
    };
    if label.contains('.') {
        return HostKind::Unknown;
    }
    match slug::validate(label) {
        Ok(()) => HostKind::Site(label.to_string()),
        Err(_) => HostKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_port_and_ipv6() {
        assert_eq!(strip_port("a.localhost:8443"), "a.localhost");
        assert_eq!(strip_port("a.localhost"), "a.localhost");
        assert_eq!(strip_port("[::1]:8443"), "[::1]");
        assert_eq!(port_of("a.localhost:8443"), Some("8443"));
        assert_eq!(port_of("a.localhost"), None);
        assert_eq!(port_of("[::1]:8443"), Some("8443"));
    }

    #[test]
    fn root_domain_and_www() {
        assert_eq!(classify("localhost:8443", "localhost"), HostKind::Root);
        assert_eq!(classify("www.localhost", "localhost"), HostKind::Root);
        assert_eq!(classify("playtest.run", "playtest.run"), HostKind::Root);
        assert_eq!(classify("PlayTest.Run.", "playtest.run"), HostKind::Root);
    }

    #[test]
    fn single_label_is_a_slug() {
        assert_eq!(
            classify("brisk-otter-41.localhost:8443", "localhost"),
            HostKind::Site("brisk-otter-41".into())
        );
        assert_eq!(
            classify("Brisk-Otter-41.playtest.run", "playtest.run"),
            HostKind::Site("brisk-otter-41".into())
        );
    }

    #[test]
    fn reserved_and_multi_label_are_unknown() {
        assert_eq!(classify("admin.localhost", "localhost"), HostKind::Unknown);
        assert_eq!(
            classify("wechat.playtest.run", "playtest.run"),
            HostKind::Unknown
        );
        assert_eq!(classify("a.b.localhost", "localhost"), HostKind::Unknown);
        assert_eq!(
            classify("under_score.localhost", "localhost"),
            HostKind::Unknown
        );
        assert_eq!(classify("-lead.localhost", "localhost"), HostKind::Unknown);
        assert_eq!(classify("example.com", "localhost"), HostKind::Unknown);
        assert_eq!(classify("127.0.0.1:8443", "localhost"), HostKind::Unknown);
        assert_eq!(classify("", "localhost"), HostKind::Unknown);
        // 后缀是子串但不是标签边界。
        assert_eq!(classify("notlocalhost", "localhost"), HostKind::Unknown);
    }
}
