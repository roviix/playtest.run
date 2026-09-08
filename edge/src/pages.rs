//! 门禁页以外的几页：根域介绍、找不到、已过期、举报。
//!
//! 都是我们自己渲染的完整 HTML，不是裸状态码——玩家拿到的是一条别人发给他的链接，
//! 浏览器默认的错误页只会让他以为是自己的网络坏了。
//!
//! 除根域介绍页外，任何一页都不出现开发者域名（AGENTS 第 7 条）。

use playtest_common::{DEVELOPER_API_URL, RESERVED_PATH_PREFIX};

use crate::html::{esc, shell};

/// 根域：一页极简介绍。这里是唯一允许把开发者引去品牌站的地方。
pub fn root(host_suffix: &str) -> String {
    let body = format!(
        "<h1>playtest</h1>\n\
<p class=\"lead\">把你手上这个能玩的版本，用一条命令放到别人面前，然后知道他们玩成了什么样。</p>\n\
<pre><code>playtest ./dist</code></pre>\n\
<p class=\"lead\">几秒钟拿到一个链接和一张二维码。拿到链接的人点开就能玩——不注册、不装东西、手机上也一样。\
每个作品一个 <code>xxx.{suffix}</code>。</p>\n\
<footer><a href=\"{dev}\">开发者从这里开始</a></footer>\n",
        suffix = esc(host_suffix),
        dev = DEVELOPER_API_URL,
    );
    shell("playtest · 一条命令，把这个版本放到别人面前", "", &body)
}

/// slug 不存在、被删了、或者压根不是一个合法的名字。
/// 不区分「没有」和「已删除」：区分了就等于给人一个扫描哪些名字被占的接口。
pub fn not_found() -> String {
    let body = "<h1>这个链接不存在，或者已经失效</h1>\n\
<p class=\"lead\">检查一下地址有没有抄漏，或者找发链接给你的人再要一次。</p>\n";
    shell("这个链接不存在", "", body)
}

/// 作品在，但请求的文件不在这个版本里。
pub fn file_not_found() -> String {
    let body = "<h1>这个页面不在这个版本里</h1>\n\
<p class=\"lead\">作品还在，只是这条地址没有对应的文件。回到首页试试。</p>\n\
<footer><a href=\"/\">回到首页</a></footer>\n";
    shell("页面不存在", "", body)
}

/// 匿名链接到期（DESIGN §3.2：不登录也能拿到一个 24 小时的链接）。
pub fn gone() -> String {
    let body = "<h1>这个匿名链接已过期</h1>\n\
<p class=\"lead\">免登录的链接有效期 24 小时，现在已经过了。找发链接给你的人再发一个新的。</p>\n";
    shell("链接已过期", "", body)
}

/// 清单指着一个取不到的 blob——对象存储被删了一半之类。玩家侧只能如实说坏了。
pub fn broken() -> String {
    let body = "<h1>这个文件取不出来</h1>\n\
<p class=\"lead\">作品在，但它的一个文件在我们这边坏了。稍后再试，或者告诉发链接给你的人。</p>\n";
    shell("文件取不出来", "", body)
}

pub const REPORT_REASONS: &[(&str, &str)] = &[
    ("phishing", "假冒别人、骗账号或骗钱"),
    ("malware", "病毒、恶意脚本或有害下载"),
    ("adult", "色情、暴力或其它不适宜的内容"),
    ("infringement", "抄袭或侵犯版权"),
    ("harassment", "针对具体的人的攻击或骚扰"),
    ("broken", "打不开、加载不出来"),
    ("other", "其它"),
];

pub fn report_form() -> String {
    let mut options = String::new();
    for (value, label) in REPORT_REASONS {
        options.push_str(&format!(
            "<option value=\"{}\">{}</option>",
            esc(value),
            esc(label)
        ));
    }
    let body = format!(
        "<h1>举报这个作品</h1>\n\
<p class=\"lead\">告诉我们这里出了什么事。不用留下你是谁。</p>\n\
<form method=\"post\" action=\"{prefix}report\">\n\
<label>是什么问题<select name=\"reason\">{options}</select></label>\n\
<label>还想补充点什么（可以不填）<textarea name=\"detail\" rows=\"5\" maxlength=\"2000\"></textarea></label>\n\
<button type=\"submit\">提交</button>\n\
</form>\n\
<footer><a href=\"/\">返回作品</a></footer>\n",
        prefix = RESERVED_PATH_PREFIX,
    );
    shell("举报这个作品", "", &body)
}

/// 举报收下之后。**不承诺处理时限**：服务条款还没有，承诺了就是虚报（AGENTS 第 4 条）。
pub fn report_done() -> String {
    let body = "<h1>已收到。</h1>\n<footer><a href=\"/\">返回作品</a></footer>\n";
    shell("已收到", "", body)
}

/// 这个 slug 这一小时的流量用完了（DESIGN §4.8 的每小时熔断）。
///
/// 说清三件事：不是作品被下架、不是玩家做错了什么、过一会儿真的能好。
/// 不说「被刷了」「触发风控」这种词——玩家既不知道也帮不上忙（AGENTS 第 8 条）。
pub fn over_quota() -> String {
    let body = "<h1>这个作品这一小时的流量用完了</h1>\n\
<p class=\"lead\">过一会儿再点一次就好。作品还在，也没有被下架——这是我们给每个作品设的每小时上限，\
免得一个人的作品被刷爆之后连累别人。</p>\n\
<p class=\"lead\">要是等了很久还这样，告诉发链接给你的人。</p>\n";
    shell("这一小时的流量用完了", "", body)
}

/// 第二周才实现的分支落在这里。v0.1 走不到，写出来是为了不让它悄悄变成 500。
pub fn not_implemented() -> String {
    let body = "<h1>这条链接现在打不开</h1>\n\
<p class=\"lead\">这种连接方式边缘还没实现。找发链接给你的人确认一下。</p>\n";
    shell("暂时打不开", "", body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_root_page_may_mention_the_brand_site() {
        assert!(root("playtest.run").contains(DEVELOPER_API_URL));
        for page in [
            not_found(),
            file_not_found(),
            gone(),
            broken(),
            report_form(),
            report_done(),
            over_quota(),
            not_implemented(),
        ] {
            assert!(
                !page.contains(playtest_common::DEVELOPER_HOST),
                "玩家页面上不能出现开发者域名"
            );
        }
    }

    #[test]
    fn over_quota_says_it_comes_back() {
        let html = over_quota();
        assert!(html.contains("过一会儿再点一次"));
        // 不吓唬玩家，也不说他们插不上手的内部词。
        for word in ["熔断", "配额", "风控", "封禁", "DDoS"] {
            assert!(!html.contains(word), "「{word}」不是给玩家看的词");
        }
    }

    #[test]
    fn root_page_shows_the_real_suffix() {
        assert!(root("localhost").contains("xxx.localhost"));
        assert!(root("playtest.run").contains("xxx.playtest.run"));
    }

    #[test]
    fn report_form_has_a_reason_and_a_textarea() {
        let html = report_form();
        assert!(html.contains("<select name=\"reason\">"));
        assert!(html.contains("<textarea name=\"detail\""));
        assert!(html.contains("value=\"phishing\""));
    }

    #[test]
    fn report_done_makes_no_promise() {
        let html = report_done();
        assert!(html.contains("已收到。"));
        for word in ["24 小时", "工作日", "尽快处理", "我们会在"] {
            assert!(!html.contains(word), "不承诺处理时限");
        }
    }
}
