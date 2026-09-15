//! 分享页（DESIGN §3.4）：`/_playtest/share`。
//!
//! 一页只做一件事——把那张竖版邀请卡交到玩家手上：图、保存、复制链接，支持系统分享的
//! 浏览器上多一个「分享」。**只有公开的作品有这一页**：私测的邀请不该被转发；公开求测的
//! 作品，玩家帮开发者招玩家正是开发者想要的——这是邀请环的病毒那一半。
//!
//! 不公开的作品这里回 404 而不是「你没有权限」：一页解释等于告诉别人这个作品存在。

use playtest_common::live::SiteLive;
use playtest_common::manifest::Manifest;
use playtest_common::{CARD_PATH, RESERVED_PATH_PREFIX};

use crate::html::{copy_row, esc, shell_hero, COPY_JS};

pub struct SharePage<'a> {
    pub manifest: &'a Manifest,
    pub live: &'a SiteLive,
    /// 作品的源 `scheme://<slug>.<后缀>[:端口]`，玩家要复制的就是它。
    pub origin: &'a str,
}

impl SharePage<'_> {
    /// 不在广场上的作品没有这一页；调用方拿到 `None` 就出 404。
    pub fn render(&self) -> Option<String> {
        if !self.live.listed {
            return None;
        }
        let m = self.manifest;
        let title = esc(&m.title);
        let link = esc(&playtest_common::door_url(self.origin, &m.slug));
        // 卡是 1080×1350。先用 aspect-ratio 占好位，图慢一点到也不会把页面顶下去。
        // `height:auto` 不能省：标签上的 `height="1350"` 是有效的样式声明，
        // 不覆盖它，浏览器就真的按 1350 像素高排，卡会被拉长。
        let head = "<meta name=\"robots\" content=\"noindex\">\n\
<style>.shot{display:block;width:100%;height:auto;aspect-ratio:1080/1350;background:#0e1014;\
border-bottom:1px solid var(--line)}</style>\n";
        let hero = format!(
            "<img class=\"shot\" src=\"{CARD_PATH}\" alt=\"Invite card for {title}\" width=\"1080\" height=\"1350\">\n"
        );
        let body = format!(
            "<h1>Share this card</h1>\n\
<p class=\"lead\">Long-press or click to save image, or share directly. The QR code on the card links here.</p>\n\
{row}<button type=\"button\" id=\"pt-share\" data-title=\"{title}\" hidden>Share</button>\n\
<div class=\"more\"><a href=\"{CARD_PATH}\" download=\"{slug}-invite.png\">Save image</a>\
<a href=\"/\">Back to project</a></div>\n\
<footer><a href=\"{RESERVED_PATH_PREFIX}report\">Report an issue</a></footer>\n\
<script>{COPY_JS}{SCRIPT}</script>\n",
            slug = esc(&m.slug),
            row = copy_row(&link, Some("Project link")),
        );
        Some(shell_hero(
            &format!("{} · Invite card", m.title),
            head,
            &hero,
            &body,
        ))
    }
}

/// 两件事：复制链接、有系统分享就用系统分享（连图一起给它，微信在分享面板里就是一张图）。
/// 写成 ES5、每一步都能失败：没有 JS 时那个只读输入框自己就是「复制链接」的办法。
const SCRIPT: &str = "\
(function(){var i=document.getElementById('pt-url'),s=document.getElementById('pt-share');\
ptCopy(i,document.getElementById('pt-copy'));\
if(!navigator.share){return}\
s.hidden=false;\
s.onclick=function(){var d={title:s.getAttribute('data-title'),url:i.value};\
if(!navigator.canShare||!window.File){navigator.share(d);return}\
fetch('/_playtest/card.png').then(function(r){return r.blob()}).then(function(b){\
var f=new File([b],'invite.png',{type:'image/png'});\
if(navigator.canShare({files:[f]})){d.files=[f]}return navigator.share(d)})\
.catch(function(){navigator.share(d).catch(function(){})})};})();";

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::manifest::{GateMode, SCHEMA};

    fn manifest() -> Manifest {
        Manifest {
            schema: SCHEMA,
            slug: "brisk-otter-41".into(),
            version: 7,
            title: "小球大冒险".into(),
            developer: "某某".into(),
            note: None,
            summary: None,
            cover: None,
            created_at: "2026-09-07T00:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: Some("godot".into()),
            kind: playtest_common::manifest::WorkKind::Web,
            entry: None,
            article: None,
            chapters: vec![],
            files: vec![],
        }
    }

    fn page<'a>(m: &'a Manifest, live: &'a SiteLive) -> SharePage<'a> {
        SharePage {
            manifest: m,
            live,
            origin: "http://brisk-otter-41.localhost:8443",
        }
    }

    #[test]
    fn only_a_public_work_has_this_page() {
        let m = manifest();
        let mut live = SiteLive::empty("brisk-otter-41");
        assert!(page(&m, &live).render().is_none());
        live.listed = true;
        assert!(page(&m, &live).render().is_some());
    }

    #[test]
    fn the_page_is_the_card_and_two_ways_to_pass_it_on() {
        let m = manifest();
        let mut live = SiteLive::empty("brisk-otter-41");
        live.listed = true;
        let html = page(&m, &live).render().unwrap();
        assert!(html.contains("<img class=\"shot\" src=\"/_playtest/card.png\""));
        assert!(html.contains("download=\"brisk-otter-41-invite.png\""));
        assert!(html.contains("value=\"http://localhost:8443/p/brisk-otter-41\""));
        assert!(html.contains(">Copy link</button>"));
        // 系统分享默认藏着：没有 navigator.share 的浏览器上它不该占位。
        assert!(html.contains("id=\"pt-share\" data-title=\"小球大冒险\" hidden"));
        assert!(html.contains("<title>小球大冒险 · Invite card</title>"));
        // 除了那张卡，一个外部资源都不加载。
        for forbidden in [
            "<script src",
            "<link rel=\"stylesheet",
            "@import",
            "//fonts.",
        ] {
            assert!(!html.contains(forbidden), "{forbidden}");
        }
        assert!(!html.contains(playtest_common::DEVELOPER_HOST));
        assert!(html.len() < 20 * 1024, "分享页 {} 字节", html.len());
    }

    #[test]
    fn a_title_with_html_in_it_stays_text() {
        let mut m = manifest();
        m.title = "\"><script>alert(1)</script>".into();
        let mut live = SiteLive::empty("brisk-otter-41");
        live.listed = true;
        let html = page(&m, &live).render().unwrap();
        assert!(!html.contains("<script>alert"));
        assert!(html.contains("&lt;script&gt;"));
    }
}
