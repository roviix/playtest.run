//! 邀请卡：把那张 PNG 拿下来，存成一个文件（DESIGN §3.4）。
//!
//! 卡不是 CLI 画的，是边缘按作品当前的样子渲染的——CLI 拿不到玩家那一侧的名额进度，
//! 也不想在三个平台上处理中文字体。所以这里做的只有三件事：按地址取、看清楚取回来的是不是一张
//! PNG、起一个人能看懂的文件名。
//!
//! **拿不到不算失败。** 发布已经成功了，链接已经能玩了；卡是边缘异步渲染出来的，
//! 刚提交完的那一两秒它可能还在读旧的清单。这种时候如实说一句「稍后可以在哪儿拿」，
//! 不把一次成功的发布变成一个红色的错（AGENTS 第 4 条）。

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use playtest_common::api::Site;

use crate::client;
use crate::output;

/// 最多试几次。重试是为了等边缘读到刚提交的那份清单。
const ATTEMPTS: u32 = 5;

/// 连不上时最多试几次。连接被拒、域名解析不了，400 毫秒之后多半还是一样；
/// 试第二次只为排除一次偶然的抖动，试到第五次只是让人白等。
const ATTEMPTS_WHEN_UNREACHABLE: u32 = 2;

/// 两次之间隔多久。
const BETWEEN: Duration = Duration::from_millis(400);

/// 多久算连不上。比控制面那边短得多——发布已经做完了，不该为一张图再等十秒。
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// 一次请求最多等多久（含下载）。一张卡是几百 KB。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(6);

/// 文件名里作品名最多留多少个字。再长的名字在 `ls` 里就是一堵墙，也容易撞上文件系统的上限。
const MAX_NAME_CHARS: usize = 40;

/// PNG 的头八个字节。响应头说自己是 PNG、内容却不是，多半中间挡了一层门户或代理。
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// 为什么这一刻拿不到卡。两种的处置不一样，所以分开。
#[derive(Debug)]
pub enum Unavailable {
    /// 连不上：网络不通，或者这个地址上根本没有东西。
    Unreachable(String),
    /// 连上了，但回来的不是一张卡：还在渲染、作品刚被删、或者中间挡了一层别的。
    NotReady(String),
}

impl std::fmt::Display for Unavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(why) | Self::NotReady(why) => f.write_str(why),
        }
    }
}

/// 一次发布之后那张卡的下场。人类模式靠它决定说哪一句话。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// `--no-card`：不存，也不必说。
    #[default]
    Skipped,
    Saved,
    /// 这一刻拿不到。链接照常能玩，卡稍后自己会好。
    Missing,
}

/// 发布成功之后顺手取的那一张卡。
#[derive(Debug, Default)]
pub struct Taken {
    pub outcome: Outcome,
    /// 存到哪了。打印用的相对路径，不是绝对路径——人要照着它去找这个文件。
    pub path: Option<String>,
    /// PNG 的字节。MCP 要把它贴进对话（DESIGN §3.2），`--json` 不带。
    pub png: Option<Vec<u8>>,
}

/// 发布成功之后：取一张卡存下来。**任何一步不成都只是没有卡，不是失败。**
pub async fn take(
    site_url: &str,
    title: &str,
    slug: &str,
    out: Option<&Path>,
    skip: bool,
) -> Taken {
    if skip {
        return Taken::default();
    }
    let url = playtest_common::card_url(site_url);
    let Ok(png) = fetch(&url).await else {
        return Taken {
            outcome: Outcome::Missing,
            ..Taken::default()
        };
    };
    let path = place_at(title, slug, out);
    match save(&path, &png) {
        Ok(()) => Taken {
            outcome: Outcome::Saved,
            path: Some(shown(&path)),
            png: Some(png),
        },
        // 写不下去（目录不在、只读、盘满了）也一样：卡还在服务器上，说一声去哪儿拿就行。
        Err(e) => {
            crate::ui::say(&format!("邀请卡没能存下来：{e:#}"));
            Taken {
                outcome: Outcome::Missing,
                path: None,
                png: Some(png),
            }
        }
    }
}

/// `playtest card <slug 或目录>` 与 MCP 的「拿邀请卡」走的那一步：这一次拿不到就是失败，
/// 因为用户要的就是这张卡。
pub async fn fetch_or_explain(site: &Site) -> Result<Vec<u8>> {
    let url = playtest_common::card_url(&site.url);
    fetch(&url).await.map_err(|why| match why {
        Unavailable::Unreachable(reason) => output::network(
            format!("连不上 {url}：{reason}"),
            "检查一下网络；作品链接本身能不能打开也顺便看看。",
        ),
        Unavailable::NotReady(reason) => output::server_error(
            format!("{url} 现在给不了邀请卡：{reason}"),
            "作品刚发出去的话过几秒再试一次；一直这样就是这一版还没准备好。",
        ),
    })
}

/// 按地址取一张卡。重试是为了等边缘读到新清单，所以间隔短、次数少。
async fn fetch(url: &str) -> std::result::Result<Vec<u8>, Unavailable> {
    let http = match client::new_http(CONNECT_TIMEOUT, REQUEST_TIMEOUT) {
        Ok(http) => http,
        Err(e) => return Err(Unavailable::Unreachable(e.to_string())),
    };
    let mut attempts = ATTEMPTS;
    let mut last = Unavailable::NotReady("还没试过".to_string());
    let mut attempt = 0;
    while attempt < attempts {
        if attempt > 0 {
            tokio::time::sleep(BETWEEN).await;
        }
        attempt += 1;
        match once(&http, url).await {
            Ok(png) => return Ok(png),
            Err(why) => {
                if matches!(why, Unavailable::Unreachable(_)) {
                    attempts = attempts.min(ATTEMPTS_WHEN_UNREACHABLE);
                }
                last = why;
            }
        }
    }
    Err(last)
}

async fn once(http: &reqwest::Client, url: &str) -> std::result::Result<Vec<u8>, Unavailable> {
    let response = http
        .get(url)
        .send()
        .await
        .map_err(|e| Unavailable::Unreachable(root_cause(&e)))?;
    let status = response.status();
    if !status.is_success() {
        return Err(Unavailable::NotReady(format!("HTTP {}", status.as_u16())));
    }
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !mime.starts_with("image/png") {
        return Err(Unavailable::NotReady(format!(
            "回来的是 {}，不是一张 PNG",
            if mime.is_empty() {
                "没说类型的东西"
            } else {
                &mime
            }
        )));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| Unavailable::Unreachable(root_cause(&e)))?;
    if !bytes.starts_with(PNG_MAGIC) {
        return Err(Unavailable::NotReady(
            "说是 PNG，字节却不像一张 PNG".to_string(),
        ));
    }
    Ok(bytes.to_vec())
}

/// reqwest 最外层那句只是把地址重复一遍，有用的是链条最里面那条。
fn root_cause(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        return format!("等了 {} 秒还没回应", REQUEST_TIMEOUT.as_secs());
    }
    let mut deepest: &dyn std::error::Error = e;
    while let Some(inner) = deepest.source() {
        deepest = inner;
    }
    deepest.to_string()
}

/// 卡存到哪：`playtest card --out` 说了就听它的。
pub fn place(site: &Site, out: Option<&Path>) -> PathBuf {
    place_at(&site.title, &site.slug, out)
}

/// 没指定就在当前目录按作品名起一个；指定的是一个已经存在的目录（`--card-out ~/Desktop`
/// 是很自然的写法）就放进去，而不是把那个目录当成文件名去写。
fn place_at(title: &str, slug: &str, out: Option<&Path>) -> PathBuf {
    let name = || PathBuf::from(file_name(title, slug));
    match out {
        Some(given) if given.is_dir() => given.join(name()),
        Some(given) => given.to_path_buf(),
        None => name(),
    }
}

/// 写下来。同名文件直接覆盖——这张卡就是这个作品此刻的样子，留着上一版的没有意义。
pub fn save(path: &Path, png: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("建不了 {} 这个目录", dir.display()))?;
    }
    std::fs::write(path, png).with_context(|| format!("写不了 {}", path.display()))
}

/// 邀请卡的文件名：`<作品名>-邀请卡.png`。
///
/// 作品名是用户自己起的，可能带斜杠、换行、甚至控制字符——原样当文件名会写到别的目录里去，
/// 或者在终端里显示成一团乱码。这里把这些拿掉；`\` `:` `*` 这些在 Windows 上根本不让用，
/// 一并拿掉，免得同一条命令在另一台机器上写不出文件。清干净之后什么都不剩（比如作品名
/// 整个是标点）就退回用 slug，它一定是安全的。
pub fn file_name(title: &str, slug: &str) -> String {
    let mut stem = clean(title);
    if stem.is_empty() {
        stem = clean(slug);
    }
    if stem.is_empty() {
        stem = "作品".to_string();
    }
    format!("{stem}-邀请卡.png")
}

fn clean(raw: &str) -> String {
    let kept: String = raw
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        .collect();
    kept.trim()
        .chars()
        .take(MAX_NAME_CHARS)
        .collect::<String>()
        .trim()
        .to_string()
}

/// 打印用的路径。就在当前目录下就写成 `./跳一跳-邀请卡.png`——人照着这个去找文件，
/// 一长串绝对路径反而看不清文件名。
pub fn shown(path: &Path) -> String {
    let Ok(here) = std::env::current_dir() else {
        return path.display().to_string();
    };
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        here.join(path)
    };
    match absolute.strip_prefix(&here) {
        Ok(rest) => format!("./{}", rest.display()),
        Err(_) => absolute.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_name_is_the_title_plus_two_words() {
        assert_eq!(file_name("跳一跳", "brisk-otter-41"), "跳一跳-邀请卡.png");
    }

    #[test]
    fn path_separators_and_control_characters_never_reach_the_file_system() {
        assert_eq!(
            file_name("a/b\\c", "brisk-otter-41"),
            "abc-邀请卡.png",
            "斜杠会把文件写到别的目录里去"
        );
        assert_eq!(
            file_name("换\n行\t了\u{7}", "brisk-otter-41"),
            "换行了-邀请卡.png"
        );
        assert_eq!(
            file_name("v1: 最终版?", "brisk-otter-41"),
            "v1 最终版-邀请卡.png",
            "Windows 上不让用的那几个也拿掉"
        );
    }

    #[test]
    fn a_very_long_title_is_cut_not_refused() {
        let long = "很".repeat(80);
        let name = file_name(&long, "brisk-otter-41");
        assert_eq!(name.chars().filter(|c| *c == '很').count(), MAX_NAME_CHARS);
        assert!(name.ends_with("-邀请卡.png"), "{name}");
    }

    /// 名字整个是路径分隔符时不能落到一个叫「-邀请卡.png」的隐形文件上。
    #[test]
    fn a_title_that_cleans_away_to_nothing_falls_back_to_the_slug() {
        assert_eq!(
            file_name("///", "brisk-otter-41"),
            "brisk-otter-41-邀请卡.png"
        );
        assert_eq!(file_name("   ", "  "), "作品-邀请卡.png");
    }

    #[test]
    fn a_file_here_is_shown_with_a_dot_in_front() {
        let here = std::env::current_dir().unwrap();
        assert_eq!(shown(Path::new("跳一跳-邀请卡.png")), "./跳一跳-邀请卡.png");
        assert_eq!(shown(&here.join("a.png")), "./a.png");
    }
}
