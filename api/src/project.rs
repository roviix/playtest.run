//! 一个作品的事实，在控制面只拼一次（REWRITE §2.1）。
//!
//! 在这之前有三条拼装路径：`live.rs` 拼 `live.json`、`plaza.rs` 拼广场卡、`routes/sites.rs`
//! 拼控制台要的那份。三条各读各的列、各填各的字段，加一个字段要改三处——所以才会出现
//! 「库里有 `boosts.reason` 这一列、契约里没有这个字段、控制台还在渲染它」这种洞。
//!
//! 现在只有这一条：[`compose`] 把数据库那一行加上几个要现算的数字，拼成一个
//! [`Project`]；`live.json`、广场卡、控制台那份全是它的投影（`Project::live`、
//! `Project::card`、[`site_view`]）。加一个字段改这里和用得上它的那一个投影，没有第三处。
//!
//! **不在这里做的**：查数据库。调用方已经握着连接、而且常常是在一个循环里（广场一次拼
//! 几十张卡），让这里再去查会把一次查询变成 N 次。要现算的数字由调用方批量取好，
//! 从 [`Extras`] 递进来。

use playtest_common::api::{Listing, Site};
use playtest_common::boost::Boost;
use playtest_common::manifest::WorkKind;
use playtest_common::plan::Plan;
use playtest_common::project::{
    Access, DeliveryMode, Owner, OwnerKind, Project, ProjectLive, PublicNote,
};

use crate::clock;
use crate::db;

/// 拼一个作品要的、不在 `sites` 那一行里的东西。
///
/// 全部有默认值：广场一次拼几十张卡，它不需要公开反馈；门禁页要公开反馈，不需要推广。
/// 谁要哪几样谁自己填，不填就是「没有」，不是「查一下」。
#[derive(Debug, Default, Clone)]
pub struct Extras {
    /// [`playtest_common::plaza::PLAYERS_WINDOW_DAYS`] 天内点过「开始」的去重人数。
    pub players: u32,
    /// 点「开始」时留了名字的去重会话数。
    pub joined: u32,
    pub followers: u32,
    pub boost: Option<Boost>,
    /// 最近几条公开反馈，新的在前。`feedback_public` 关着时调用方可以不查。
    pub public_feedback: Vec<PublicNote>,
    /// 隧道此刻连着。只有边缘知道，控制面从上报里得到；还没有这个字段时是 `false`。
    pub tunnel_online: bool,
    pub last_seen: Option<String>,
}

/// 数据库那一行 + 主人 + 现算的数字 → 一个作品此刻的全部事实。
pub fn compose(
    site: db::SiteRow,
    owner: Option<db::SiteOwner>,
    url: String,
    extras: Extras,
) -> Project {
    let listing = site.listing;
    let kind = owner
        .as_ref()
        .and_then(|o| o.kind.parse::<OwnerKind>().ok())
        .unwrap_or(OwnerKind::Anon);
    let owner = Owner {
        kind,
        display_name: owner
            .as_ref()
            .map(|o| o.display_name.clone())
            .unwrap_or_else(|| crate::auth::ANON_DISPLAY_NAME.to_string()),
        login: None,
        avatar_url: owner.and_then(|o| o.avatar_url),
    };
    // 档位现在只有两级：匿名与登录。Pro 要等订阅那张表（M4），到时候这一行从
    // `users.plan` 读，不是从身份类型猜。
    let plan = match kind {
        OwnerKind::Anon => Plan::Anon,
        OwnerKind::Github => Plan::Free,
    };
    let feedback_public = listing.feedback_public;
    Project {
        slug: site.slug,
        url,
        owner,
        plan,
        created_at: site.created_at.clone(),
        expires_at: site.expires_at,

        // 交付方式还没有自己的列：上传是默认，隧道与混合由边缘知道（M1 补上）。
        mode: DeliveryMode::Upload,
        current_version: site.current_version,
        // 迁移 003 之前提交的老作品没有 `updated_at`，拿建档时间顶上——
        // 空字符串会让它在广场上排到最前面去。
        updated_at: Some(listing.updated_at.unwrap_or(site.created_at)),
        isolated: false,
        spa: false,
        gate: Default::default(),
        tunnel_online: extras.tunnel_online,
        last_seen: extras.last_seen,

        title: site.title,
        kind: site.work_kind.parse::<WorkKind>().unwrap_or_default(),
        summary: listing.summary,
        // 「这版改了什么」和「想让人看什么」是同一句话（REWRITE §9.6）。库里这一列
        // 现在叫 `seek_note`，CLI 那一侧的 `--seek` 在 M1 并进 `--note`。
        note: listing.seek_note,
        cover_hash: listing.cover_hash,
        engine: listing.engine,
        community_url: listing.community_url,

        // 口令与邀请名单是 Pro 的东西（M1 加列）；在那之前每个作品都是「有链接就能看」。
        access: Access::Link,

        public: listing.public,
        seeking: listing.seeking,
        seats: listing.seats.filter(|n| *n > 0),
        joined: extras.joined,
        hidden: listing.hidden_at.is_some(),
        players: extras.players,
        boost: extras.boost,

        followers: extras.followers,
        feedback_public,
        public_feedback: if feedback_public {
            extras.public_feedback
        } else {
            Vec::new()
        },
    }
}

/// `live.json` 那一份。`generated_at` 在这里盖章，因为它说的是「这份文件什么时候整理的」，
/// 不是作品的属性。
pub fn live_view(project: &Project) -> ProjectLive {
    ProjectLive {
        generated_at: clock::now_string(),
        ..project.live()
    }
}

/// 控制台与 CLI 要的那一份（`GET /v1/projects/{slug}`）。
///
/// 线上的形状暂时还是 [`Site`]：控制台和已经发出去的 CLI 都在读它，换形状要和它们一起换
/// （M3）。但它现在是 [`Project`] 的**投影**，不是第四条拼装路径——字段从哪来一目了然。
pub fn site_view(project: &Project) -> Site {
    Site {
        slug: project.slug.clone(),
        url: project.url.clone(),
        title: project.title.clone(),
        kind: project.kind,
        current_version: project.current_version,
        created_at: project.created_at.clone(),
        expires_at: project.expires_at.clone(),
        listing: Listing {
            public: project.public,
            seeking: project.seeking,
            seek_note: project.note.clone(),
            summary: project.summary.clone(),
            hidden: project.hidden,
            has_cover: project.cover_hash.is_some(),
            seats: project.seats,
            joined: project.joined,
            followers: project.followers,
            community_url: project.community_url.clone(),
            feedback_public: project.feedback_public,
            boost: project.boost.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::project::Fact;

    fn row() -> db::SiteRow {
        db::SiteRow {
            slug: "brisk-otter-41".into(),
            title: "小球".into(),
            created_at: "2026-09-08T00:00:00Z".into(),
            expires_at: None,
            current_version: Some(7),
            work_kind: "web".into(),
            listing: db::ListingRow {
                public: true,
                seeking: true,
                seek_note: Some("改了新手引导".into()),
                summary: Some("一个滚来滚去的小球".into()),
                cover_hash: Some("abcdef0123456789".into()),
                cover_mime: Some("image/png".into()),
                engine: Some("godot".into()),
                updated_at: Some("2026-09-09T03:00:00Z".into()),
                hidden_at: None,
                hidden_reason: None,
                reviewed_at: None,
                seats: Some(10),
                community_url: None,
                feedback_public: false,
            },
        }
    }

    fn owner() -> db::SiteOwner {
        db::SiteOwner {
            user_id: "u1".into(),
            kind: "github".into(),
            display_name: "小雨".into(),
            avatar_url: Some("https://avatars.githubusercontent.com/u/1".into()),
        }
    }

    fn url() -> String {
        "https://brisk-otter-41.playtest.run".into()
    }

    #[test]
    fn every_projection_reads_the_same_row() {
        let p = compose(
            row(),
            Some(owner()),
            url(),
            Extras {
                players: 12,
                joined: 6,
                followers: 3,
                ..Default::default()
            },
        );
        let card = p.card();
        let live = p.live();
        let site = site_view(&p);

        assert_eq!(card.seats, Some(10));
        assert_eq!(live.seats, Some(10));
        assert_eq!(site.listing.seats, Some(10));

        assert_eq!(card.joined, 6);
        assert_eq!(live.joined, 6);
        assert_eq!(site.listing.joined, 6);

        assert_eq!(card.followers, 3);
        assert_eq!(live.followers, 3);
        assert_eq!(site.listing.followers, 3);

        assert!(card.has_cover());
        assert!(site.listing.has_cover);

        assert_eq!(
            card.fact(),
            Fact::Seats {
                joined: 6,
                seats: 10
            }
        );
    }

    #[test]
    fn an_owner_we_cannot_find_is_an_anonymous_one() {
        let p = compose(row(), None, url(), Extras::default());
        assert_eq!(p.owner.kind, OwnerKind::Anon);
        assert_eq!(p.owner.display_name, crate::auth::ANON_DISPLAY_NAME);
        assert_eq!(p.plan, Plan::Anon);
        assert_eq!(p.card().developer, crate::auth::ANON_DISPLAY_NAME);
    }

    #[test]
    fn logging_in_moves_the_project_off_the_anonymous_allowance() {
        let p = compose(row(), Some(owner()), url(), Extras::default());
        assert_eq!(p.plan, Plan::Free);
        assert!(!p.plan.link_expires());
        assert!(p.badge(), "免费档还带角标");
    }

    #[test]
    fn an_old_row_without_an_update_time_falls_back_to_its_birthday() {
        let mut r = row();
        r.listing.updated_at = None;
        let p = compose(r, Some(owner()), url(), Extras::default());
        assert_eq!(p.updated_at.as_deref(), Some("2026-09-08T00:00:00Z"));
        assert!(
            !p.card().updated_at.is_empty(),
            "空的时间会让它排到广场最前面"
        );
    }

    #[test]
    fn a_closed_feedback_wall_never_leaks_a_note() {
        let note = PublicNote {
            name: None,
            text: "这里有个洞".into(),
            version: 7,
            at: "2026-09-09T04:00:00Z".into(),
        };
        let p = compose(
            row(),
            Some(owner()),
            url(),
            Extras {
                public_feedback: vec![note.clone()],
                ..Default::default()
            },
        );
        assert!(!p.feedback_public);
        assert!(p.public_feedback.is_empty(), "关着就一条都不带出来");

        let mut r = row();
        r.listing.feedback_public = true;
        let p = compose(
            r,
            Some(owner()),
            url(),
            Extras {
                public_feedback: vec![note],
                ..Default::default()
            },
        );
        assert_eq!(p.live().public_feedback.len(), 1);
    }

    #[test]
    fn taken_down_keeps_the_developers_wish_but_leaves_the_wall() {
        let mut r = row();
        r.listing.hidden_at = Some("2026-09-09T05:00:00Z".into());
        let p = compose(r, Some(owner()), url(), Extras::default());
        assert!(p.hidden);
        assert!(p.public, "public 仍是开发者的意愿");
        assert!(!p.listed());
        assert!(!p.live().listed);
        assert!(site_view(&p).listing.hidden, "控制台要如实看到这件事");
    }
}
