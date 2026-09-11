//! 「这台机器上已经有的东西」——控制面地址、存着的令牌、目录记着的 slug。
//!
//! 在这之前这四步散在 13 处，而且 `sites.rs` 和 `output.rs` 各抄了一份 `resolve_slug` 与
//! `client_with_saved_token`（逐行一样，连注释都是同一段话的两种写法）。两份意味着
//! 人话模式和 `--json` 模式可以慢慢漂成两种行为——`playtest rollback --json` 往 stdout
//! 吐裸链接就是这么来的。现在只有这一份。

use std::path::Path;

use anyhow::Result;

use crate::args;
use crate::client::Client;
use crate::clock;
use crate::config::{self, Config};
use crate::output;

/// 一条命令要用到的本机状态。
pub struct Session {
    pub api: String,
    pub config: Config,
    config_path: std::path::PathBuf,
}

impl Session {
    /// 读配置、定下控制面地址。不联网。
    pub fn open(api_flag: Option<&str>) -> Result<Self> {
        let config_path = config::default_path()?;
        Ok(Self {
            api: args::api_base(api_flag),
            config: config::load(&config_path)?,
            config_path,
        })
    }

    /// 用已经存下来的令牌建客户端。
    ///
    /// 没有令牌就是 `None`，不是错误，也**不**凭空申请一个新的匿名令牌：`ls`、`rm`、`card`
    /// 问的都是「我已经有的东西」，拿一个崭新的身份去问只会看到空列表，更让人困惑。
    pub fn client(&self) -> Result<Option<Client>> {
        let Some(token) = self.config.usable_token(&self.api, clock::now()) else {
            return Ok(None);
        };
        let mut client = Client::new(&self.api)?;
        client.set_token(Some(token.to_string()));
        Ok(Some(client))
    }

    /// 和 [`Session::client`] 一样，但没有令牌时把「还没发过东西」说成一句人话。
    /// `what` 填这条命令想做的事，比如「没有 brisk-otter-41 可以删」。
    pub fn client_or_say(&self, what: &str) -> Result<Client> {
        self.client()?.ok_or_else(|| {
            output::bad_input(format!(
                "这台机器上还没发过东西，{what}。运行 playtest ./dist 发一个。"
            ))
        })
    }

    /// 参数可以是 slug，也可以是一个发过的目录。
    ///
    /// 是目录就查这台机器记着的「这个目录上次发到哪个作品」；没记过就说清楚，
    /// 不去猜一个 slug——猜错会改到别人的作品。
    pub fn slug_of(&self, target: &str) -> Result<String> {
        let path = Path::new(target);
        if !path.is_dir() {
            return Ok(target.to_string());
        }
        let key = std::fs::canonicalize(path)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| target.to_string());
        match self.config.remembered_slug(&key) {
            Some(slug) => Ok(slug.to_string()),
            None => Err(output::bad_input(format!(
                "{target} 这个目录还没发过。先运行 playtest {target} 把它发出去。"
            ))),
        }
    }

    /// 记下「这个目录发到了哪个作品」之类的改动。
    pub fn save(&self) -> Result<()> {
        config::save(&self.config_path, &self.config)
    }

    pub fn forget(&mut self, slug: &str) -> Result<()> {
        self.config.forget_slug(slug);
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_with(config: Config) -> Session {
        Session {
            api: "http://127.0.0.1:8787".into(),
            config,
            config_path: std::path::PathBuf::from("/dev/null"),
        }
    }

    #[test]
    fn a_name_that_is_not_a_directory_is_taken_as_a_slug() {
        let s = session_with(Config::default());
        assert_eq!(s.slug_of("brisk-otter-41").unwrap(), "brisk-otter-41");
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
        let s = session_with(config);
        assert_eq!(
            s.slug_of(dir.path().to_str().unwrap()).unwrap(),
            "keen-yak-7"
        );
    }

    #[test]
    fn an_unknown_directory_says_what_to_run() {
        let dir = tempfile::tempdir().unwrap();
        let s = session_with(Config::default());
        let err = s.slug_of(dir.path().to_str().unwrap()).unwrap_err();
        assert!(err.to_string().contains("还没发过"), "{err}");
    }

    #[test]
    fn no_token_is_not_an_error_until_someone_needs_one() {
        let s = session_with(Config::default());
        assert!(s.client().unwrap().is_none(), "没令牌不是错误");
        let err = s
            .client_or_say("没有它可以删")
            .err()
            .expect("没令牌时要说人话");
        assert!(err.to_string().contains("没有它可以删"), "{err}");
    }
}
