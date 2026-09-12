//! 一条命令的**结果**，和怎么把它说出来分开（REWRITE §3.1）。
//!
//! 在这之前，`ls / rm / open / card / followers` 在 `sites.rs`（人话）和 `output.rs`
//! （`--json`）里各实现了一遍，而 `versions / rollback / unlist` 只有人话那一半——
//! 于是 `playtest rollback --json` 会往 stdout 写一条裸链接，破坏「stdout 只有一个
//! JSON 对象」的契约，而 `versions --json` 的 stdout 是空的。
//!
//! 现在每条命令只产出一个 [`Report`]，由 [`Report::human`] 或 [`Report::json`] 说出来。
//! 加一条命令就是加一个变体加两段渲染，不可能只加一半。

use serde::Serialize;
use serde_json::{json, Value};

use playtest_common::api::{Me, Site, VersionFiles, VersionList};

use crate::clock;
use crate::ui;

/// 一条命令做完之后有什么可说的。
pub enum Report {
    LoggedOut,
    /// `ls`
    Sites(Vec<Site>),
    /// `open`：`opened` 说的是「弹了浏览器没有」——`--json` 下不弹，跑在 agent
    /// 或 CI 里多半没有浏览器，也不该抢焦点。
    Opened {
        site: Site,
        opened: bool,
    },
    /// `rm`
    Removed {
        slug: String,
    },
    /// `rm` 时用户说了不删。
    KeptAfterAsking {
        slug: String,
    },
    /// `versions`
    Versions {
        slug: String,
        list: VersionList,
    },
    /// `rollback`
    RolledBack {
        site: Site,
        version: u32,
    },
    /// `unlist`
    Unlisted {
        slug: String,
    },
    /// `card`
    Card {
        site: Site,
        path: String,
    },
    /// `whoami`
    Whoami {
        me: Me,
        projects: Option<u32>,
    },
    /// `whoami`，但这台机器上还没有任何身份。
    Nobody,
    /// `files`
    Files(VersionFiles),
}

impl Report {
    /// JSON 里的 `action`。脚本按它分流，所以它是契约的一部分，不跟着命令改名走。
    pub fn action(&self) -> &'static str {
        match self {
            Self::LoggedOut => "logout",
            Self::Sites(_) => "list",
            Self::Opened { .. } => "open",
            Self::Removed { .. } | Self::KeptAfterAsking { .. } => "remove",
            Self::Versions { .. } => "versions",
            Self::RolledBack { .. } => "rollback",
            Self::Unlisted { .. } => "unlist",
            Self::Card { .. } => "card",
            Self::Whoami { .. } | Self::Nobody => "whoami",
            Self::Files(_) => "files",
        }
    }

    /// `--json` 模式：stdout 上那一个对象的主体。`ok` / `action` / `elapsed_ms`
    /// 由 [`crate::output::emit_report`] 统一盖上，这里只管这条命令自己的字段。
    pub fn json(&self) -> Value {
        match self {
            Self::LoggedOut => json!({ "local_credentials_cleared": true }),
            Self::Sites(sites) => json!({
                "sites": sites.iter().map(site_json).collect::<Vec<_>>(),
            }),
            Self::Opened { site, opened } => json!({
                "slug": site.slug,
                "url": playtest_common::door_url(&site.url, &site.slug),
                "opened": opened,
            }),
            Self::Removed { slug } => json!({ "slug": slug, "removed": true }),
            Self::KeptAfterAsking { slug } => json!({ "slug": slug, "removed": false }),
            Self::Versions { slug, list } => json!({
                "slug": slug,
                "current_version": list.current_version,
                "versions": list.versions.iter().map(|v| json!({
                    "version": v.version,
                    "created_at": v.created_at,
                    "note": v.note,
                    "file_count": v.file_count,
                    "total_bytes": v.total_bytes,
                    "current": v.current,
                })).collect::<Vec<_>>(),
            }),
            Self::RolledBack { site, version } => json!({
                "slug": site.slug,
                "url": playtest_common::door_url(&site.url, &site.slug),
                "title": site.title,
                "version": version,
            }),
            Self::Unlisted { slug } => json!({ "slug": slug, "listed": false }),
            Self::Whoami { me, projects } => json!({
                "kind": me.kind,
                "display_name": me.display_name,
                "login": me.login,
                "avatar_url": me.avatar_url,
                "expires_at": me.expires_at,
                "projects": projects,
            }),
            Self::Nobody => json!({ "kind": null, "projects": 0 }),
            Self::Files(files) => json!({
                "slug": files.slug,
                "version": files.version,
                "current": files.current,
                "total_bytes": files.total_bytes,
                "files": files.files.iter().map(|f| json!({
                    "path": f.path,
                    "size": f.size,
                    "hash": f.hash,
                    "url": f.url,
                })).collect::<Vec<_>>(),
            }),
            Self::Card { site, path } => json!({
                "slug": site.slug,
                "title": site.title,
                // 图不进 JSON：几百 KB 的 base64 塞进一行 stdout，对着管道读的脚本会很难受。
                // 这里给路径，图在文件里。
                "card_url": playtest_common::card_url(&site.url),
                "card_path": path,
            }),
        }
    }

    /// 人话模式。stdout 只放用户要拿走的东西（链接），叙述走 stderr（[`ui`] 管这件事）。
    pub fn human(&self) {
        match self {
            Self::LoggedOut => ui::say("当前令牌已撤销或失效，本机凭据已清除。作品没有删除。"),
            Self::Sites(sites) if sites.is_empty() => {
                ui::say("你还没有作品。运行 playtest ./dist 发一个。");
            }
            Self::Sites(sites) => {
                for site in sites {
                    say_site(site);
                }
            }
            Self::Opened { site, .. } => {
                ui::out(&playtest_common::door_url(&site.url, &site.slug));
            }
            Self::Removed { slug } => ui::say(&format!("已删掉 {slug}。")),
            Self::KeptAfterAsking { .. } => ui::say("没有删。"),
            Self::Versions { slug, list } if list.versions.is_empty() => {
                ui::say(&format!("{slug} 还没发过任何版本。"));
            }
            Self::Versions { slug, list } => {
                for v in &list.versions {
                    let mark = if v.current {
                        "← 玩家现在看到的"
                    } else {
                        ""
                    };
                    let note = v
                        .note
                        .as_deref()
                        .map(|n| format!("  「{n}」"))
                        .unwrap_or_default();
                    ui::say(&format!(
                        "v{:<3} {}  {} 个文件 {}{}  {}",
                        v.version,
                        clock::human(&v.created_at),
                        v.file_count,
                        ui::bytes(v.total_bytes),
                        note,
                        mark
                    ));
                }
                ui::say(&format!("换回某一版：playtest rollback {slug} <版本号>"));
            }
            Self::RolledBack { site, version } => {
                ui::say(&format!("玩家现在看到的是《{}》v{}。", site.title, version));
                ui::link(&playtest_common::door_url(&site.url, &site.slug));
            }
            Self::Unlisted { slug } => {
                ui::say(&format!("已把 {slug} 从广场上拿下来了，链接照常能开。"));
            }
            Self::Card { site, path } => {
                ui::say(&format!(
                    "《{}》的邀请卡已存到 {path}——发到群里，别人长按识别就能玩",
                    site.title
                ));
            }
            Self::Nobody => {
                ui::say("这台机器上还没有身份。运行 playtest ./dist 发一个，或者 playtest login。");
            }
            Self::Whoami { me, projects } => {
                match me.login.as_deref() {
                    Some(login) => ui::say(&format!("{}（GitHub @{login}）", me.display_name)),
                    None => ui::say(&format!(
                        "{}——没登录。发出去的链接 {} 小时后失效；playtest login 之后就不会了。",
                        me.display_name,
                        playtest_common::ANON_LINK_TTL_HOURS
                    )),
                }
                if let Some(at) = &me.expires_at {
                    ui::say(&format!("这个身份 {} 到期。", clock::human(at)));
                }
                if let Some(n) = projects {
                    ui::say(&format!("{n} 个作品。"));
                }
            }
            Self::Files(files) => {
                if files.files.is_empty() {
                    ui::say(&format!(
                        "{} v{} 里一个文件都没有。",
                        files.slug, files.version
                    ));
                    return;
                }
                let mark = if files.current {
                    "（玩家现在看到的）"
                } else {
                    ""
                };
                ui::say(&format!(
                    "{} v{}{mark}：{} 个文件，共 {}",
                    files.slug,
                    files.version,
                    files.files.len(),
                    ui::bytes(files.total_bytes)
                ));
                for f in &files.files {
                    // 路径是要拿走的东西（复制去比对、去 curl），所以走 stdout。
                    ui::out(&format!(
                        "{:>9}  {}  {}",
                        ui::bytes(f.size),
                        &f.hash[..f.hash.len().min(12)],
                        f.path
                    ));
                }
            }
        }
    }
}

/// `ls` 里的一个作品。人话这一侧，一个作品几行。
fn say_site(site: &Site) {
    let version = match site.current_version {
        Some(v) => format!("v{v}"),
        None => "还没上传过版本".to_string(),
    };
    ui::out(&format!("{}（{}）", site.title, version));
    ui::out(&format!(
        "  {}",
        playtest_common::door_url(&site.url, &site.slug)
    ));
    ui::out(&format!("  slug：{}", site.slug));
    if let Some(expires_at) = &site.expires_at {
        ui::out(&format!("  {} 后失效", clock::human(expires_at)));
    }
    if let Some(line) = plaza_line(&site.listing) {
        ui::out(&format!("  {line}"));
    }
    // 关注数原来要单开一条 `playtest followers` 才看得到——它是「下一版还有没有人来」
    // 的唯一读数，值得和链接放在一起。0 不说（DESIGN §3.5 的规矩）。
    if site.listing.followers > 0 {
        ui::out(&format!(
            "  {} 人关注着它，下一版发出去他们会收到通知",
            site.listing.followers
        ));
    }
    ui::out("");
}

/// 广场上的状态，一行说完；不在广场上就不占一行。
fn plaza_line(listing: &playtest_common::api::Listing) -> Option<String> {
    if !listing.public {
        return None;
    }
    if listing.hidden {
        return Some("在广场上被撤下了（被多人举报，等人复核）；链接照常能开".to_string());
    }
    Some(match (&listing.seeking, &listing.seek_note) {
        (true, Some(note)) => format!("在广场上 · 正在找人测：{note}"),
        (true, None) => "在广场上 · 正在找人测".to_string(),
        (false, _) => "在广场上".to_string(),
    })
}

/// `--json` 里的一个作品。和 [`say_site`] 说的是同一件事，两种媒介。
fn site_json(site: &Site) -> Value {
    let l = &site.listing;
    json!({
        "slug": site.slug,
        "url": playtest_common::door_url(&site.url, &site.slug),
        "title": site.title,
        // 叫 version 不叫 current_version：脚本里不需要「当前」这个限定，
        // 而且 MCP 那一侧的 playtest_list 用的就是这个名字，两处对得上。
        "version": site.current_version,
        "created_at": site.created_at,
        "expires_at": site.expires_at,
        "public": l.public,
        "hidden": l.hidden,
        "seeking": l.seeking,
        "seek_note": l.seek_note,
        "summary": l.summary,
        "has_cover": l.has_cover,
        "seats": l.seats,
        "joined": l.joined,
        "followers": l.followers,
        "community_url": l.community_url,
        "feedback_public": l.feedback_public,
    })
}

/// 盖在每个 `--json` 对象最外层的那几样。
#[derive(Debug, Serialize)]
pub struct Envelope<'a> {
    pub ok: bool,
    pub action: &'a str,
    #[serde(flatten)]
    pub body: Value,
    pub elapsed_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::api::{Listing, VersionInfo};

    fn site() -> Site {
        Site {
            slug: "brisk-otter-41".into(),
            url: "https://brisk-otter-41.playtest.run".into(),
            title: "小球".into(),
            current_version: Some(7),
            created_at: "2026-09-08T00:00:00Z".into(),
            expires_at: None,
            listing: Listing {
                followers: 12,
                ..Default::default()
            },
        }
    }

    /// 每条命令都要能被机器读——这正是 `versions / rollback / unlist` 以前缺的那一半。
    #[test]
    fn every_report_has_a_machine_readable_body() {
        let reports = [
            Report::Sites(vec![site()]),
            Report::Opened {
                site: site(),
                opened: false,
            },
            Report::Removed {
                slug: "brisk-otter-41".into(),
            },
            Report::KeptAfterAsking {
                slug: "brisk-otter-41".into(),
            },
            Report::Versions {
                slug: "brisk-otter-41".into(),
                list: VersionList {
                    slug: "brisk-otter-41".into(),
                    current_version: Some(7),
                    versions: vec![VersionInfo {
                        version: 7,
                        created_at: "2026-09-09T00:00:00Z".into(),
                        note: Some("改了新手引导".into()),
                        file_count: 53,
                        total_bytes: 2_200_000,
                        current: true,
                    }],
                },
            },
            Report::RolledBack {
                site: site(),
                version: 3,
            },
            Report::Unlisted {
                slug: "brisk-otter-41".into(),
            },
            Report::Card {
                site: site(),
                path: "./小球-邀请卡.png".into(),
            },
        ];
        for report in &reports {
            let body = report.json();
            assert!(body.is_object(), "{} 的主体要是一个对象", report.action());
            assert!(!report.action().is_empty());
        }
    }

    #[test]
    fn whoami_says_when_there_is_nobody_yet() {
        let body = Report::Nobody.json();
        assert!(body["kind"].is_null());
        assert_eq!(body["projects"], 0);
        assert_eq!(Report::Nobody.action(), "whoami");
    }

    #[test]
    fn files_answer_with_the_hash_so_a_script_can_compare() {
        use playtest_common::api::{VersionFile, VersionFiles};
        let body = Report::Files(VersionFiles {
            slug: "brisk-otter-41".into(),
            version: 7,
            current: true,
            total_bytes: 2_200_000,
            files: vec![VersionFile {
                path: "index.html".into(),
                size: 512,
                hash: "abc123def456".into(),
                url: "https://brisk-otter-41.playtest.run/index.html".into(),
            }],
        })
        .json();
        assert_eq!(body["version"], 7);
        assert_eq!(body["current"], true);
        assert_eq!(body["files"][0]["hash"], "abc123def456");
        assert_eq!(body["files"][0]["path"], "index.html");
        assert!(
            body["files"][0]["url"]
                .as_str()
                .unwrap()
                .ends_with("/index.html"),
            "要能直接打开"
        );
    }

    #[test]
    fn rolling_back_answers_with_json_not_a_bare_url() {
        let body = Report::RolledBack {
            site: site(),
            version: 3,
        }
        .json();
        assert_eq!(body["version"], 3);
        assert_eq!(body["slug"], "brisk-otter-41");
        assert_eq!(body["url"], "https://playtest.run/p/brisk-otter-41");
    }

    #[test]
    fn versions_carry_every_field_a_script_needs() {
        let body = Report::Versions {
            slug: "brisk-otter-41".into(),
            list: VersionList {
                slug: "brisk-otter-41".into(),
                current_version: Some(7),
                versions: vec![VersionInfo {
                    version: 7,
                    created_at: "2026-09-09T00:00:00Z".into(),
                    note: None,
                    file_count: 53,
                    total_bytes: 2_200_000,
                    current: true,
                }],
            },
        }
        .json();
        assert_eq!(body["current_version"], 7);
        let v = &body["versions"][0];
        assert_eq!(v["version"], 7);
        assert_eq!(v["file_count"], 53);
        assert_eq!(v["total_bytes"], 2_200_000u64);
        assert_eq!(v["current"], true);
        assert!(v["note"].is_null(), "没写就是 null，不是空字符串");
    }

    #[test]
    fn deleting_and_keeping_share_an_action_but_not_an_answer() {
        let removed = Report::Removed { slug: "a".into() };
        let kept = Report::KeptAfterAsking { slug: "a".into() };
        assert_eq!(removed.action(), kept.action());
        assert_eq!(removed.json()["removed"], true);
        assert_eq!(kept.json()["removed"], false);
    }

    #[test]
    fn the_card_answers_with_a_path_not_the_bytes() {
        let body = Report::Card {
            site: site(),
            path: "./小球-邀请卡.png".into(),
        }
        .json();
        assert_eq!(body["card_path"], "./小球-邀请卡.png");
        assert_eq!(
            body["card_url"],
            "https://brisk-otter-41.playtest.run/_playtest/card.png"
        );
        assert!(body.get("png").is_none(), "几百 KB 的 base64 不进 stdout");
    }

    #[test]
    fn a_listed_site_says_so_in_json_too() {
        let mut s = site();
        s.listing.public = true;
        s.listing.seeking = true;
        s.listing.seek_note = Some("新手引导看得懂吗".into());
        let body = Report::Sites(vec![s]).json();
        let one = &body["sites"][0];
        assert_eq!(one["public"], true);
        assert_eq!(one["seeking"], true);
        assert_eq!(one["seek_note"], "新手引导看得懂吗");
        assert_eq!(one["followers"], 12);
    }
}
