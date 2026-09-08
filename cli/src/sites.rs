//! `playtest ls / rm / open`：看和管这台机器上发过的作品。

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{bail, Result};

use crate::args;
use crate::client::Client;
use crate::config::{self, Config};
use crate::{clock, ui};

pub async fn ls(api_flag: Option<&str>) -> Result<()> {
    let api = args::api_base(api_flag);
    let config = config::load(&config::default_path()?)?;
    let Some(client) = client_with_saved_token(&api, &config)? else {
        ui::say("这台机器上还没发过东西。运行 playtest ./dist 发一个。");
        return Ok(());
    };

    let sites = client.list_sites().await?;
    if sites.is_empty() {
        ui::say("你还没有作品。运行 playtest ./dist 发一个。");
        return Ok(());
    }
    for site in sites {
        let version = match site.current_version {
            Some(v) => format!("v{v}"),
            None => "还没上传过版本".to_string(),
        };
        ui::out(&format!("{}（{}）", site.title, version));
        ui::out(&format!("  {}", site.url));
        ui::out(&format!("  slug：{}", site.slug));
        if let Some(expires_at) = &site.expires_at {
            ui::out(&format!("  {} 后失效", clock::human(expires_at)));
        }
        ui::out("");
    }
    Ok(())
}

pub async fn rm(slug: &str, yes: bool, api_flag: Option<&str>) -> Result<()> {
    let api = args::api_base(api_flag);
    let config_path = config::default_path()?;
    let mut config = config::load(&config_path)?;
    let Some(client) = client_with_saved_token(&api, &config)? else {
        bail!("这台机器上还没发过东西，没有 {slug} 可以删。");
    };

    if !yes {
        match ui::confirm(&format!("要删掉 {slug} 吗？删了它的链接就打不开了。输入 y 确认：")) {
            Some(true) => {}
            Some(false) => {
                ui::say("没有删。");
                return Ok(());
            }
            None => bail!("这里不是终端，没法问你确认。确定要删就加 -y：playtest rm {slug} -y"),
        }
    }

    client.delete_site(slug).await?;
    config.forget_slug(slug);
    config::save(&config_path, &config)?;
    ui::say(&format!("已删掉 {slug}。"));
    Ok(())
}

pub async fn open(target: &str, api_flag: Option<&str>) -> Result<()> {
    let api = args::api_base(api_flag);
    let config = config::load(&config::default_path()?)?;
    let slug = resolve_slug(target, &config)?;
    let Some(client) = client_with_saved_token(&api, &config)? else {
        bail!("这台机器上还没发过东西，没有链接可以打开。");
    };

    let site = client.get_site(&slug).await?;
    ui::out(&site.url);
    launch_browser(&site.url);
    Ok(())
}

/// 参数可以是 slug，也可以是一个发过的目录。
fn resolve_slug(target: &str, config: &Config) -> Result<String> {
    let path = Path::new(target);
    if !path.is_dir() {
        return Ok(target.to_string());
    }
    let key = std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| target.to_string());
    match config.remembered_slug(&key) {
        Some(slug) => Ok(slug.to_string()),
        None => bail!("{target} 这个目录还没发过。先运行 playtest {target} 把它发出去。"),
    }
}

/// 用已经存过的令牌建客户端。没有就返回 `None`——查看和删除都是看已有的东西，
/// 为此凭空申请一个新的匿名令牌只会看到空列表，更让人困惑。
fn client_with_saved_token(api: &str, config: &Config) -> Result<Option<Client>> {
    let Some(token) = config.usable_token(api, clock::now()) else {
        return Ok(None);
    };
    let mut client = Client::new(api)?;
    client.set_token(Some(token.to_string()));
    Ok(Some(client))
}

fn launch_browser(url: &str) {
    let Some(mut command) = browser_command(url) else {
        ui::warn("不知道怎么在这个系统上开浏览器，复制上面的链接自己打开。");
        return;
    };
    // 浏览器自己的输出和我们的混在一起没有意义。
    command.stdout(Stdio::null()).stderr(Stdio::null());
    match command.status() {
        Ok(status) if status.success() => {}
        _ => ui::warn("没能自动打开浏览器，复制上面的链接自己打开。"),
    }
}

fn browser_command(url: &str) -> Option<Command> {
    // macOS 也是 unix，得排在前面。
    if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(url);
        Some(command)
    } else if cfg!(windows) {
        let mut command = Command::new("cmd");
        // start 会把第一个带引号的参数当窗口标题，所以先塞一个空的。
        command.args(["/c", "start", "", url]);
        Some(command)
    } else if cfg!(unix) {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        Some(command)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_that_is_not_a_directory_is_taken_as_a_slug() {
        let config = Config::default();
        assert_eq!(
            resolve_slug("brisk-otter-41", &config).unwrap(),
            "brisk-otter-41"
        );
    }

    #[test]
    fn a_known_directory_resolves_to_its_slug() {
        let dir = tempfile::tempdir().unwrap();
        let key = std::fs::canonicalize(dir.path())
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let mut config = Config::default();
        config.remember(key, "keen-yak-7".into());
        assert_eq!(
            resolve_slug(dir.path().to_str().unwrap(), &config).unwrap(),
            "keen-yak-7"
        );
    }

    #[test]
    fn an_unknown_directory_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_slug(dir.path().to_str().unwrap(), &Config::default()).unwrap_err();
        assert!(err.to_string().contains("还没发过"), "{err}");
    }
}
