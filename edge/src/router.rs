//! 边缘的路由表：根域上有哪几条、作品子域的保留前缀下有哪几条，各允许什么方法。
//!
//! 这是一张**能读的表**，不是 matchit——作品子域上兜底的那条（按清单出文件、混合模式转发）
//! 要先查清单才知道路径意味着什么，塞不进通用路由器；而这一小撮固定路径若散在 `app.rs`
//! 的 `if` 链里，「根域到底有几条路径」这个问题就只能靠读 200 行代码回答。
//! 现在它在这里，测试对着它数数，`app.rs` 只负责每一条落地之后做什么。
//!
//! 路径字面量都从 `playtest_common` 拿：边缘、控制面、CLI 三方认的是同一份契约。

use axum::http::Method;
use playtest_common::api::routes::LLMS_TXT;
use playtest_common::follow::{edge_paths, root_paths};
use playtest_common::{CARD_PATH, CARD_WIDE_PATH, COVER_PATH, RESERVED_PATH_PREFIX, SHARE_PATH};

use crate::card::Shape;
use crate::follow::SW_PATH;

/// 一条路径允许的方法。回 405 时 `Allow` 头也从这里出。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Allow {
    /// GET 与 HEAD。
    Read,
    /// 只有 POST：表单动作。
    Post,
    /// 表单页：GET 看，POST 交。
    ReadOrPost,
    /// 方法由处理函数自己判（它要区分的比这几种多）。
    Any,
}

impl Allow {
    pub fn permits(self, method: &Method) -> bool {
        match self {
            Allow::Read => *method == Method::GET || *method == Method::HEAD,
            Allow::Post => *method == Method::POST,
            Allow::ReadOrPost => {
                *method == Method::GET || *method == Method::HEAD || *method == Method::POST
            }
            Allow::Any => true,
        }
    }

    /// 405 响应上 `Allow` 头的值。
    pub fn header(self) -> &'static str {
        match self {
            Allow::Read => "GET, HEAD",
            Allow::Post => "POST",
            Allow::ReadOrPost => "GET, HEAD, POST",
            Allow::Any => "GET, HEAD, POST",
        }
    }
}

// ---------------------------------------------------------------- 根域

/// 根域（`playtest.run/`）上的每一条。**只有这几条**，别的一律 404——根域上不放任何
/// 用户内容，它是玩家路径里唯一我们说了算的一页（DESIGN §3.9、§3.10）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Root<'a> {
    /// `/`：广场。
    Plaza,
    Collections,
    Collection(&'a str),
    /// `POST /follow`：关注页里留邮箱就是关注广场。
    Follow,
    /// `/me`：关注页。
    Me,
    /// `POST /me/action`：取消关注、给自己发链接、关掉浏览器通知。
    MeAction,
    /// `/me/confirm/{token}`：确认信里的链接。
    Confirm(&'a str),
    /// `/me/unsubscribe/{token}`：每封信底部的一键退订。
    Unsubscribe(&'a str),
    /// `/_playtest/sw.js`：根域自己的 Service Worker（只有它能弹通知）。
    ServiceWorker,
    /// `/llms.txt`：给搜到玩家域的助手指路（AGENTS 第 7 条的第二个例外）。
    Llms,
    /// `/p/{slug}`：主域作品邀请函（DESIGN §3.1、§3.3）。
    Project(&'a str),
    /// `/favicon.svg`：高清矢量品牌图标。
    FaviconSvg,
    /// `/favicon.ico`：点阵图标兜底。
    FaviconIco,
    /// `/robots.txt`：搜索引擎与 AI 爬虫抓取规则。
    Robots,
    /// `/sitemap.xml`：公开作品与合集的站点地图。
    Sitemap,
}

/// 根域的表：路径与允许的方法。带令牌与 slug 的按前缀匹配，其余精确匹配。
pub const ROOT: &[(&str, Allow)] = &[
    ("/", Allow::Read),
    (playtest_common::collection::INDEX, Allow::Read),
    (playtest_common::collection::PREFIX, Allow::Read),
    (root_paths::PROJECT_PREFIX, Allow::ReadOrPost),
    (root_paths::FOLLOW, Allow::Post),
    (root_paths::ME, Allow::Read),
    (root_paths::ME_ACTION, Allow::Post),
    (root_paths::ME_CONFIRM, Allow::Read),
    (root_paths::ME_UNSUBSCRIBE, Allow::Read),
    (SW_PATH, Allow::Read),
    (LLMS_TXT, Allow::Read),
    (root_paths::FAVICON_SVG, Allow::Read),
    (root_paths::FAVICON_ICO, Allow::Read),
    (root_paths::ROBOTS_TXT, Allow::Read),
    (root_paths::SITEMAP_XML, Allow::Read),
];

/// 根域上这条路径是哪一间房。`None` 就是 404。
pub fn root(path: &str) -> Option<(Root<'_>, Allow)> {
    let hit = match path {
        "/" => Root::Plaza,
        playtest_common::collection::INDEX => Root::Collections,
        p if p == root_paths::FOLLOW => Root::Follow,
        p if p == root_paths::ME => Root::Me,
        p if p == root_paths::ME_ACTION => Root::MeAction,
        p if p == SW_PATH => Root::ServiceWorker,
        p if p == LLMS_TXT => Root::Llms,
        p if p == root_paths::FAVICON_SVG => Root::FaviconSvg,
        p if p == root_paths::FAVICON_ICO => Root::FaviconIco,
        p if p == root_paths::ROBOTS_TXT => Root::Robots,
        p if p == root_paths::SITEMAP_XML => Root::Sitemap,
        p => {
            if let Some(slug) = p.strip_prefix(playtest_common::collection::PREFIX) {
                playtest_common::slug::validate(slug).ok()?;
                return Some((Root::Collection(slug), Allow::Read));
            }
            if let Some(slug) = p.strip_prefix(root_paths::PROJECT_PREFIX) {
                if slug.is_empty() || slug.contains('/') {
                    return None;
                }
                Root::Project(slug)
            } else if let Some(token) = p.strip_prefix(root_paths::ME_CONFIRM) {
                Root::Confirm(token)
            } else if let Some(token) = p.strip_prefix(root_paths::ME_UNSUBSCRIBE) {
                Root::Unsubscribe(token)
            } else {
                return None;
            }
        }
    };
    let allow = match hit {
        Root::Plaza
        | Root::Collections
        | Root::Collection(_)
        | Root::Me
        | Root::Confirm(_)
        | Root::Unsubscribe(_)
        | Root::ServiceWorker
        | Root::Llms
        | Root::FaviconSvg
        | Root::FaviconIco
        | Root::Robots
        | Root::Sitemap => Allow::Read,
        Root::Project(_) => Allow::ReadOrPost,
        Root::Follow | Root::MeAction => Allow::Post,
    };
    Some((hit, allow))
}

// ---------------------------------------------------------------- 作品子域的保留前缀

/// `/_playtest/...` 后面那一段。作品目录里就算有同名文件也永远取不到这里。
pub fn reserved_tail(path: &str) -> Option<&str> {
    if path == RESERVED_PATH_PREFIX.trim_end_matches('/') {
        return Some("");
    }
    path.strip_prefix(RESERVED_PATH_PREFIX)
}

/// 作品子域保留前缀下的每一条。不在这张表里的尾巴是 404（作品自己的文件取不到这个前缀下面）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reserved {
    /// 任何 Host 都答的健康检查。分流之前就处理：探针带的 Host 通常是 IP。
    Healthz,
    /// 开发者的隧道握手。不看作品状态：这一刻这个 slug 多半还什么都没有。
    Handshake,
    /// `POST /_playtest/start`：门禁页那个「开始」。
    Start,
    /// 举报：GET 表单，POST 提交。
    Report,
    /// `/_playtest/me`：SDK 问「我是谁」。
    Me,
    /// `/_playtest/sdk.js`。
    Sdk,
    /// 封面图。
    Cover,
    /// 邀请卡 PNG，两种形状。
    Card(Shape),
    /// 分享页。
    Share,
    /// `POST /_playtest/follow`：门禁页与玩后落点的「有新版本时告诉我」。
    Follow,
    /// `GET /_playtest/invite`：随时调出作品邀请函。
    Invite,
}

/// 保留前缀下的表。第一列是 `/_playtest/` 之后的尾巴。
pub const RESERVED: &[(&str, Allow)] = &[
    ("healthz", Allow::Read),
    (crate::tunnel::HANDSHAKE_TAIL, Allow::Any),
    ("start", Allow::Post),
    ("report", Allow::ReadOrPost),
    ("me", Allow::Any),
    ("sdk.js", Allow::Any),
    (tail_of(COVER_PATH), Allow::Read),
    (tail_of(CARD_PATH), Allow::Read),
    (tail_of(CARD_WIDE_PATH), Allow::Read),
    (tail_of(SHARE_PATH), Allow::Read),
    (tail_of(edge_paths::FOLLOW), Allow::Post),
    ("invite", Allow::Read),
];

/// 契约里给的是完整路径；这张表按尾巴分。编译期就把前缀剥掉，两者一致不用靠人记。
const fn tail_of(full: &str) -> &str {
    let (prefix, full) = (RESERVED_PATH_PREFIX.as_bytes(), full.as_bytes());
    assert!(full.len() > prefix.len());
    let mut i = 0;
    while i < prefix.len() {
        assert!(full[i] == prefix[i], "契约路径不在保留前缀下");
        i += 1;
    }
    // 上面逐字节比过了前缀，剩下的是合法 UTF-8 的一段。
    match std::str::from_utf8(full.split_at(prefix.len()).1) {
        Ok(s) => s,
        Err(_) => panic!("契约路径不是 UTF-8"),
    }
}

/// 保留前缀下这条尾巴是什么。`None` 就是 404。
pub fn reserved(tail: &str) -> Option<(Reserved, Allow)> {
    let hit = match tail {
        "healthz" => Reserved::Healthz,
        t if t == crate::tunnel::HANDSHAKE_TAIL => Reserved::Handshake,
        "start" => Reserved::Start,
        "report" => Reserved::Report,
        "me" => Reserved::Me,
        "sdk.js" => Reserved::Sdk,
        t if t == tail_of(COVER_PATH) => Reserved::Cover,
        t if t == tail_of(CARD_PATH) => Reserved::Card(Shape::Portrait),
        t if t == tail_of(CARD_WIDE_PATH) => Reserved::Card(Shape::Wide),
        t if t == tail_of(SHARE_PATH) => Reserved::Share,
        t if t == tail_of(edge_paths::FOLLOW) => Reserved::Follow,
        "invite" => Reserved::Invite,
        _ => return None,
    };
    let allow = match hit {
        Reserved::Healthz
        | Reserved::Cover
        | Reserved::Card(_)
        | Reserved::Share
        | Reserved::Invite => Allow::Read,
        Reserved::Start | Reserved::Follow => Allow::Post,
        Reserved::Report => Allow::ReadOrPost,
        Reserved::Handshake | Reserved::Me | Reserved::Sdk => Allow::Any,
    };
    Some((hit, allow))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_root_host_has_exactly_nine_doors() {
        assert_eq!(ROOT.len(), 15, "根域多一条路径要先改 DESIGN §3.9");
        for (path, allow) in ROOT {
            let probe = if path.ends_with('/') && *path != "/" {
                format!("{path}tok")
            } else {
                path.to_string()
            };
            let (_, got) = root(&probe).unwrap_or_else(|| panic!("{path} 不在表里"));
            assert_eq!(got, *allow, "{path}");
        }
        assert!(root("/about").is_none());
        assert!(root("/me/").is_none());
        assert!(root("/index.html").is_none());
        assert_eq!(root("/robots.txt"), Some((Root::Robots, Allow::Read)));
        assert_eq!(root("/sitemap.xml"), Some((Root::Sitemap, Allow::Read)));
        assert_eq!(
            root("/p/scarlet-tiger-35"),
            Some((Root::Project("scarlet-tiger-35"), Allow::ReadOrPost))
        );
        assert!(root("/p/").is_none());
        assert!(root("/p/a/b").is_none());
        assert_eq!(
            root("/me/confirm/abc"),
            Some((Root::Confirm("abc"), Allow::Read))
        );
        assert_eq!(
            root("/me/unsubscribe/xyz"),
            Some((Root::Unsubscribe("xyz"), Allow::Read))
        );
    }

    #[test]
    fn the_reserved_prefix_has_exactly_twelve_doors() {
        assert_eq!(RESERVED.len(), 12);
        for (tail, allow) in RESERVED {
            let (_, got) = reserved(tail).unwrap_or_else(|| panic!("{tail} 不在表里"));
            assert_eq!(got, *allow, "{tail}");
        }
        assert!(reserved("index.html").is_none());
        assert!(reserved("").is_none());
        assert_eq!(
            reserved("card-wide.png"),
            Some((Reserved::Card(Shape::Wide), Allow::Read))
        );
    }

    #[test]
    fn tails_come_from_the_contract() {
        assert_eq!(tail_of(CARD_PATH), "card.png");
        assert_eq!(tail_of(SHARE_PATH), "share");
        assert_eq!(tail_of(edge_paths::FOLLOW), "follow");
        assert!(SW_PATH.starts_with(RESERVED_PATH_PREFIX));
    }

    #[test]
    fn reserved_prefix_is_never_a_site_file() {
        assert_eq!(reserved_tail("/_playtest/start"), Some("start"));
        assert_eq!(reserved_tail("/_playtest/"), Some(""));
        assert_eq!(reserved_tail("/_playtest"), Some(""));
        assert_eq!(reserved_tail("/_playtest/deep/thing"), Some("deep/thing"));
        assert_eq!(reserved_tail("/index.html"), None);
        assert_eq!(reserved_tail("/_playtestx/start"), None);
    }

    #[test]
    fn allow_headers_match_what_is_permitted() {
        assert!(Allow::Read.permits(&Method::HEAD));
        assert!(!Allow::Read.permits(&Method::POST));
        assert!(Allow::Post.permits(&Method::POST));
        assert!(!Allow::Post.permits(&Method::GET));
        assert!(Allow::ReadOrPost.permits(&Method::POST));
        assert!(!Allow::ReadOrPost.permits(&Method::DELETE));
        assert_eq!(Allow::Read.header(), "GET, HEAD");
        assert_eq!(Allow::Post.header(), "POST");
    }
}
