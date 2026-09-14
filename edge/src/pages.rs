//! 门禁页以外的几页：找不到、已过期、举报、流量用完。根域那一页在 `plaza.rs`。
//!
//! 都是我们自己渲染的完整 HTML，不是裸状态码——玩家拿到的是一条别人发给他的链接，
//! 浏览器默认的错误页只会让他以为是自己的网络坏了。
//!
//! 除根域（广场）外，任何一页都不出现开发者域名（AGENTS 第 7 条）。

use playtest_common::RESERVED_PATH_PREFIX;

use crate::html::{esc, shell};

/// slug 不存在、被删了、或者压根不是一个合法的名字。
/// 不区分「没有」和「已删除」：区分了就等于给人一个扫描哪些名字被占的接口。
pub fn not_found() -> String {
    let body = "<h1>This link does not exist or has expired</h1>\n\
<p class=\"lead\">Please check the URL, or ask the person who shared it for a new link.</p>\n";
    shell("Link not found", "", body)
}

/// 作品在，但请求的文件不在这个版本里。
pub fn file_not_found() -> String {
    let body = "<h1>Page not found in this version</h1>\n\
<p class=\"lead\">The project is online, but this URL does not match any file in this release. Try returning to the home page.</p>\n\
<footer><a href=\"/\">Return to home</a></footer>\n";
    shell("Page not found", "", body)
}

/// 匿名链接到期（DESIGN §3.2：不登录也能拿到一个 24 小时的链接）。
pub fn gone() -> String {
    let body = "<h1>This anonymous link has expired</h1>\n\
<p class=\"lead\">Anonymous links are valid for 24 hours and have now expired. Ask the creator for a new link.</p>\n";
    shell("Link expired", "", body)
}

/// 清单指着一个取不到的 blob——对象存储被删了一半之类。玩家侧只能如实说坏了。
pub fn broken() -> String {
    let body = "<h1>Unable to load this file</h1>\n\
<p class=\"lead\">The project is online, but a file is corrupted on our end. Please try again later or notify the creator.</p>\n";
    shell("File unavailable", "", body)
}

/// 当前无法确认作品是否存在或仍有权访问。503 与 404 分开，恢复后浏览器和 CDN 才会重试。
pub fn unavailable() -> String {
    let body = "<h1>Project temporarily unavailable</h1>\n\
<p class=\"lead\">We cannot reach project storage right now to verify access. Please try again shortly; your device is fine.</p>\n";
    shell("Temporarily unavailable", "", body)
}

pub fn untrusted_action() -> String {
    shell(
        "Please open from invitation page",
        "",
        "<h1>Please open from invitation page</h1><p class=\"lead\">This request did not originate from this site. Please return to the invitation page and try again.</p><footer><a href=\"/\">Return to Plaza</a></footer>",
    )
}

pub const REPORT_REASONS: &[(&str, &str)] = &[
    ("phishing", "Phishing, impersonation, or scam"),
    ("malware", "Malware, malicious scripts, or harmful downloads"),
    ("adult", "Sexually explicit, violent, or inappropriate content"),
    ("infringement", "Copyright infringement or plagiarism"),
    ("harassment", "Targeted harassment or abuse"),
    ("broken", "Broken link or fails to load"),
    ("other", "Other issues"),
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
        "<h1>Report this project</h1>\n\
<p class=\"lead\">Tell us what happened. You can remain anonymous.</p>\n\
<form method=\"post\" action=\"{prefix}report\">\n\
<label>What is the issue?<select name=\"reason\">{options}</select></label>\n\
<label>Additional details (optional)<textarea name=\"detail\" rows=\"5\" maxlength=\"2000\"></textarea></label>\n\
<button type=\"submit\">Submit</button>\n\
</form>\n\
<footer><a href=\"/\">Back to project</a></footer>\n",
        prefix = RESERVED_PATH_PREFIX,
    );
    shell("Report this project", "", &body)
}

/// 举报收下之后。**不承诺处理时限**：服务条款还没有，承诺了就是虚报（AGENTS 第 4 条）。
pub fn report_done() -> String {
    let body = "<h1>Report received.</h1>\n<footer><a href=\"/\">Back to project</a></footer>\n";
    shell("Report received", "", body)
}

/// 这个 slug 这一小时的流量用完了（DESIGN §4.8 的每小时熔断）。
///
/// 说清三件事：不是作品被下架、不是玩家做错了什么、过一会儿真的能好。
/// 不说「被刷了」「触发风控」这种词——玩家既不知道也帮不上忙（AGENTS 第 8 条）。
pub fn over_quota() -> String {
    let body = "<h1>Hourly traffic limit reached for this project</h1>\n\
<p class=\"lead\">Please check back in a little while. The project is still here and has not been taken down — this is an hourly safeguard we place on every project to prevent runaway traffic.</p>\n\
<p class=\"lead\">If this persists after some time, let the creator know.</p>\n";
    shell("Hourly limit reached", "", body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_page_here_may_mention_the_brand_site() {
        // 玩家路径上唯一允许出现开发者域名的是根域的广场（`plaza.rs`），不在这里。
        for page in [
            not_found(),
            file_not_found(),
            gone(),
            broken(),
            report_form(),
            report_done(),
            over_quota(),
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
        assert!(html.contains("check back in a little while"));
        // 不吓唬玩家，也不说他们插不上手的内部词。
        for word in ["熔断", "配额", "风控", "封禁", "DDoS"] {
            assert!(!html.contains(word), "「{word}」不是给玩家看的词");
        }
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
        assert!(html.contains("Report received."));
        for word in ["24 hours", "business day", "as soon as possible", "we will"] {
            assert!(!html.contains(word), "No promised timeline");
        }
    }
}
