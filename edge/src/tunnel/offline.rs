//! 隧道这一侧要出的几页：开发者的电脑不在线、人太多、那边没响应、那边超时。
//!
//! 都是完整 HTML，和 `pages.rs` 同一套外壳与样式——玩家拿到的是别人发给他的一条链接，
//! 一个裸状态码只会让他以为是自己的网络坏了。
//!
//! 这几页上都不出现开发者域名（AGENTS 第 7 条），也不说「隧道」「yamux」「上游」
//! 这些玩家插不上手的词：他能做的只有等一会儿，或者去问发链接给他的人。

use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use crate::html::{esc, shell};
use crate::tunnel::registry::LastSeen;

/// 没有 JS 时按这个时区显示，和门禁页同一个约定。
const FALLBACK_OFFSET_HOURS: i8 = 8;

/// 「开发者的电脑暂时不在线，上次在线 12:40」（DESIGN §3.3），而不是让玩家等超时。
pub fn offline(seen: &LastSeen) -> String {
    let developer = esc(&seen.developer);
    let title = esc(&seen.title);
    let when = match readable(&seen.at) {
        Some((machine, human)) => format!(
            "<p class=\"meta\">Last online <time datetime=\"{}\">{}</time></p>\n{LOCAL_TIME_SCRIPT}",
            esc(&machine),
            esc(&human)
        ),
        // 时间读不懂就只说「上次在线过」，不把一串机器码摆给玩家看。
        None => "<p class=\"meta\">Last online time not recorded</p>\n".to_string(),
    };
    let body = format!(
        "<p class=\"by\">{developer} is currently offline</p>\n<h1>{title}</h1>\n\
<p class=\"lead\">This project is hosted directly from the developer's machine. You can play when they are online, but they are currently offline.</p>\n\
{when}\
<p class=\"lead\">Please try again later, or let the creator know.</p>\n"
    );
    shell(&format!("{} is currently offline", seen.developer), "", &body)
}

/// 同时在场的人超过了这个档位的上限（[`playtest_common::tunnel::Claims::max_players`]）。
pub fn busy() -> String {
    let body = "<h1>Too many players right now, please try again soon</h1>\n\
<p class=\"lead\">Concurrent players have reached this project's limit. Please wait a moment and try again.</p>\n";
    shell("Too many players", "", body)
}

/// 隧道在，但开发者机器上那个进程没接。多半是他把 dev server 关了、或者端口填错了。
pub fn unreachable() -> String {
    let body = "<h1>No response from developer machine</h1>\n\
<p class=\"lead\">The machine is connected, but the development server is not responding. Try again later or notify the creator.</p>\n";
    shell("No response", "", body)
}

/// 请求送进去了，但迟迟没有响应头。
pub fn timed_out() -> String {
    let body = "<h1>Request timed out</h1>\n\
<p class=\"lead\">The request reached the developer machine, but took too long to respond. Try refreshing the page.</p>\n";
    shell("Request timed out", "", body)
}

/// 把 RFC 3339 换成人能读的。第一个值给 `<time datetime>`，第二个是没有 JS 时显示的那串。
/// 和门禁页的到期时间同一个手法：服务端出一个绝对时间，有 JS 就换成访客自己的时区。
fn readable(raw: &str) -> Option<(String, String)> {
    let at = OffsetDateTime::parse(raw, &Rfc3339).ok()?;
    let local = at.to_offset(UtcOffset::from_hms(FALLBACK_OFFSET_HOURS, 0, 0).ok()?);
    let human = format!(
        "{} {}, {:02}:{:02} (UTC+{})",
        crate::when::month_short(local.month()),
        local.day(),
        local.hour(),
        local.minute(),
        FALLBACK_OFFSET_HOURS,
    );
    Some((at.format(&Rfc3339).ok()?, human))
}

/// 内联、可有可无：把上面那个绝对时间换成访客自己时区的写法。
/// 关掉 JS 只是看到 UTC+8 的时间，这一页本来也没有要点的东西。
const LOCAL_TIME_SCRIPT: &str = "<script>\
for(const t of document.querySelectorAll('time[datetime]')){\
const d=new Date(t.getAttribute('datetime'));\
if(!isNaN(d))t.textContent=d.toLocaleString(undefined,{dateStyle:'long',timeStyle:'short'});}\
</script>\n";

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::manifest::GateMode;

    fn seen(at: &str) -> LastSeen {
        LastSeen {
            at: at.into(),
            title: "小球大冒险".into(),
            developer: "某某".into(),
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            hybrid: false,
        }
    }

    #[test]
    fn says_who_what_and_when() {
        let html = offline(&seen("2026-09-07T04:40:00Z"));
        assert!(html.contains("某某 is currently offline"));
        assert!(html.contains("小球大冒险"));
        assert!(html.contains("Last online <time datetime=\"2026-09-07T04:40:00Z\">"));
        assert!(html.contains("Sep 7, 12:40 (UTC+8)"));
        assert!(html.contains("<title>某某 is currently offline</title>"));
        // 玩家页面上不出现品牌域名，也不出现我们内部的说法。
        assert!(!html.contains(playtest_common::DEVELOPER_HOST));
        for word in ["隧道", "yamux", "上游", "WebSocket"] {
            assert!(!html.contains(word), "「{word}」不是给玩家看的词");
        }
    }

    #[test]
    fn a_time_we_cannot_read_never_reaches_the_player() {
        let html = offline(&seen("刚才"));
        assert!(!html.contains("刚才"));
        assert!(html.contains("Last online"));
        assert!(!html.contains("<time"));
    }

    #[test]
    fn everything_from_the_token_is_escaped() {
        let mut s = seen("2026-09-07T04:40:00Z");
        s.title = "<img src=x onerror=alert(1)>".into();
        s.developer = "\"><script>alert(2)</script>".into();
        let html = offline(&s);
        assert!(!html.contains("<img src=x"));
        assert!(!html.contains("<script>alert"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    }

    #[test]
    fn the_other_three_pages_are_complete_html() {
        for html in [busy(), unreachable(), timed_out()] {
            assert!(html.starts_with("<!doctype html>\n<html lang=\"en\">"));
            assert!(html.contains("<h1>"));
            assert!(!html.contains(playtest_common::DEVELOPER_HOST));
        }
        assert!(busy().contains("Too many players"));
        assert!(unreachable().contains("No response"));
        assert!(timed_out().contains("Request timed out"));
    }
}
