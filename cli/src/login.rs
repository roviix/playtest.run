//! `playtest login`：GitHub 登录一次，之后不再问（DESIGN §3.2）。
//!
//! 走 GitHub 的设备码流程：终端打一个八位码和一个网址，人在任何一个浏览器里输完，
//! 这里轮询到结果。不在本机开回调端口——远程 SSH、容器、没有图形界面的机器都一样能登。
//! 手里若还有一个没到期的匿名令牌，会一起带过去：那个身份下的作品归到账号里，不再 24 小时后失效。

use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use playtest_common::api::LoginPollResponse;
use playtest_common::DEVELOPER_API_URL;
use serde::Serialize;

use crate::args;
use crate::client::Client;
use crate::config;
use crate::{output, sites, ui};

/// 就算 GitHub 说的 `expires_in` 更长，也别在这儿挂一天。
const MAX_WAIT: Duration = Duration::from_secs(20 * 60);

#[derive(Debug, Serialize)]
struct LoginReport {
    ok: bool,
    action: &'static str,
    login: String,
    display_name: String,
    migrated_sites: u32,
}

pub async fn run(api_flag: Option<&str>) -> Result<()> {
    let config_path = config::default_path()?;
    let mut config = config::load(&config_path)?;
    let api = args::api_base(api_flag, config.api.as_deref());
    let machine = output::is_json();

    let mut client = Client::new(&api)?;
    if config.is_logged_in() {
        if let Some(token) = config.usable_token(&api, crate::clock::now()) {
            client.set_token(Some(token.to_string()));
            if let Ok(me) = client.me().await {
                let who = me.login.as_deref().unwrap_or(&me.display_name);
                if !machine {
                    ui::say(&format!(
                        "This machine is already signed in as @{who}. Continuing replaces it with whichever account you sign in with now."
                    ));
                }
            }
        }
    }

    // 匿名令牌带着去：登录成功时作品一起归进来。
    let anon_token = config
        .usable_token(&api, crate::clock::now())
        .filter(|_| !config.is_logged_in())
        .map(str::to_string);
    client.set_token(anon_token.clone());

    let start = client.device_login_start().await?;
    if !machine {
        ui::say(&format!("Open {} in your browser", start.verification_uri));
        ui::say(&format!("Enter this code: {}", start.user_code));
        ui::say("Sign in to playtest, then confirm connecting this terminal. (Ctrl-C cancels)");
        sites::launch_browser(&start.verification_uri);
    } else {
        // 机器模式下 stdout 只能有最后那个对象，码和网址走 stderr，让调用方能转给人。
        eprintln!("{}", start.verification_uri);
        eprintln!("{}", start.user_code);
    }

    let deadline = Instant::now() + Duration::from_secs(u64::from(start.expires_in)).min(MAX_WAIT);
    let mut interval = Duration::from_secs(u64::from(start.interval.max(1)));
    let login = loop {
        tokio::time::sleep(interval).await;
        if Instant::now() >= deadline {
            bail!(
                "That code expired after {} minutes. Run playtest login again for a new one.",
                start.expires_in / 60
            );
        }
        match client.device_login_poll(&start.device_code).await? {
            LoginPollResponse::Pending { interval: next } => {
                interval = Duration::from_secs(u64::from(next.max(1)));
            }
            LoginPollResponse::Ok(login) => break login,
        }
    };

    config.set_login(&api, login.token.clone(), login.login.clone());
    config::save(&config_path, &config)?;

    if machine {
        output::emit_login(&LoginReport {
            ok: true,
            action: "login",
            login: login.login,
            display_name: login.display_name,
            migrated_sites: login.migrated_sites,
        });
        return Ok(());
    }

    ui::say(&format!("Signed in as {}.", login.display_name));
    match login.migrated_sites {
        0 => {}
        n => ui::say(&format!(
            "{} now belong to your account, so those links no longer expire after 24 hours. The invitation page credits {} instead of an anonymous developer.",
            ui::count(u64::from(n), "anonymous project"),
            login.display_name
        )),
    }
    ui::say(&format!(
        "Projects and followers live under the same account: {DEVELOPER_API_URL}/console/"
    ));
    Ok(())
}
