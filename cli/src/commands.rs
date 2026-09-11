//! 每条命令一次实现，产出一个 [`Report`]（REWRITE §3.1）。
//!
//! 这些函数不打印任何东西——说出来是 `main` 的事，人话还是 JSON 由它决定。
//! 这样「人话模式做得到、`--json` 做不到」这种事在类型上就不可能发生。

use std::path::Path;

use anyhow::Result;
use playtest_common::api::UpdateSiteRequest;

use crate::report::Report;
use crate::session::Session;
use crate::{card, output, sites, ui};

/// `<slug 或目录>` 指的那个作品此刻的样子。要先知道它是什么才能拿卡、开浏览器。
pub async fn look_up(target: &str, api: Option<&str>) -> Result<playtest_common::api::Site> {
    let session = Session::open(api)?;
    let slug = session.slug_of(target)?;
    let client = session.client_or_say(&format!("没有 {slug} 可看"))?;
    Ok(client.get_site(&slug).await?)
}

pub async fn ls(api: Option<&str>) -> Result<Report> {
    let session = Session::open(api)?;
    // 没令牌就是还没发过——回一张空列表，不报错。问「我有什么」的答案是「没有」。
    let Some(client) = session.client()? else {
        return Ok(Report::Sites(Vec::new()));
    };
    Ok(Report::Sites(client.list_sites().await?))
}

pub async fn open(target: &str, api: Option<&str>, launch: bool) -> Result<Report> {
    let session = Session::open(api)?;
    let slug = session.slug_of(target)?;
    let client = session.client_or_say("没有链接可以打开")?;
    let site = client.get_site(&slug).await?;
    if launch {
        sites::launch_browser(&site.url);
    }
    Ok(Report::Opened {
        site,
        opened: launch,
    })
}

/// `ask` 为真时没加 `-y` 就停下来问一句。`--json` 模式下调用方传 `false`——
/// 机器模式不该卡在一个没人回答的提问上。
pub async fn rm(slug: &str, yes: bool, api: Option<&str>, ask: bool) -> Result<Report> {
    // 「你没法确认」要先说，再去碰网络：控制面连不上时报「还没发过东西」会把人引到
    // 错的方向上，而问题其实是这条命令在机器模式下缺一个 -y。
    if !yes && !ask {
        return Err(output::usage(format!(
            "--json 模式不会停下来问你。确定要删就加 -y：playtest rm {slug} -y --json"
        )));
    }

    let mut session = Session::open(api)?;
    let client = session.client_or_say(&format!("没有 {slug} 可以删"))?;

    if !yes {
        match ui::confirm(&format!(
            "要删掉 {slug} 吗？删了它的链接就打不开了。输入 y 确认："
        )) {
            Some(true) => {}
            Some(false) => {
                return Ok(Report::KeptAfterAsking {
                    slug: slug.to_string(),
                })
            }
            None => {
                return Err(output::bad_input(format!(
                    "这里不是终端，没法问你确认。确定要删就加 -y：playtest rm {slug} -y"
                )))
            }
        }
    }

    client.delete_site(slug).await?;
    session.forget(slug)?;
    Ok(Report::Removed {
        slug: slug.to_string(),
    })
}

pub async fn versions(target: &str, api: Option<&str>) -> Result<Report> {
    let session = Session::open(api)?;
    let slug = session.slug_of(target)?;
    let client = session.client_or_say(&format!("没有 {slug} 的版本可看"))?;
    let list = client.list_versions(&slug).await?;
    Ok(Report::Versions { slug, list })
}

pub async fn rollback(target: &str, version: &str, api: Option<&str>) -> Result<Report> {
    // 版本号写错先说，不必等到发现「这台机器还没发过东西」——那会把人引到错的方向上。
    let number = parse_version(version)?;
    let session = Session::open(api)?;
    let slug = session.slug_of(target)?;
    let client = session.client_or_say(&format!("没有 {slug} 可以回滚"))?;
    let site = client.activate_version(&slug, number).await?;
    Ok(Report::RolledBack {
        site,
        version: number,
    })
}

/// `3` 和 `v3` 都认。别的形状当场说清楚，不去猜。
fn parse_version(raw: &str) -> Result<u32> {
    raw.trim()
        .trim_start_matches(['v', 'V'])
        .parse()
        .map_err(|_| output::bad_input(format!("版本要写成 3 或 v3，不认识「{raw}」。")))
}

pub async fn unlist(slug: &str, api: Option<&str>) -> Result<Report> {
    let session = Session::open(api)?;
    let client = session.client_or_say(&format!("没有 {slug} 可以从广场上拿下来"))?;
    client
        .update_site(
            slug,
            &UpdateSiteRequest {
                public: Some(false),
                seeking: Some(false),
                // 名额、群、反馈公开与否都不动：下架的是「在广场上出现」这一件事，
                // 链接还能开，已经进来的玩家看到的东西不该跟着变。
                ..UpdateSiteRequest::default()
            },
        )
        .await?;
    Ok(Report::Unlisted {
        slug: slug.to_string(),
    })
}

/// 再拿一张这个作品**此刻**的邀请卡。
///
/// 发布那一刻已经存过一张，但人会删掉、会换电脑、会在名额加满之后想要一张写着新数字的。
/// 卡是边缘按当前状态渲染的，所以每次拿到的都是最新那一张。
pub async fn card(target: &str, out: Option<&Path>, api: Option<&str>) -> Result<Report> {
    let site = look_up(target, api).await?;
    let png = card::fetch_or_explain(&site).await?;
    let path = card::place(&site, out);
    card::save(&path, &png).map_err(output::as_bad_input)?;
    Ok(Report::Card {
        path: card::shown(&path),
        site,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_can_be_written_with_or_without_the_v() {
        assert_eq!(parse_version("3").unwrap(), 3);
        assert_eq!(parse_version("v3").unwrap(), 3);
        assert_eq!(parse_version("V3").unwrap(), 3);
        assert_eq!(parse_version(" 7 ").unwrap(), 7);
    }

    #[test]
    fn anything_else_says_what_it_wanted() {
        let err = parse_version("最新").unwrap_err();
        assert!(err.to_string().contains("写成 3 或 v3"), "{err}");
        assert!(parse_version("").is_err());
        assert!(parse_version("v").is_err());
    }
}
