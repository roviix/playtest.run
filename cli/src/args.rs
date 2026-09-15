//! 命令行的形状。
//!
//! 第一个位置参数身兼两职：目录（上传）和端口号（隧道）。分辨规则放在 [`classify`] 里，
//! 是个纯函数，好测也好改。

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// 没有 `--api` 也没有环境变量 `PLAYTEST_API` 时用它。
pub const DEFAULT_API: &str = playtest_common::DEVELOPER_API_URL;

/// 帮助的头两行。第一次用的人只看这两行就够开始了。
const EXAMPLES: &str = "playtest ./dist            把这个目录发出去，拿到链接和二维码
playtest 5173              把本地开发服务器接出去，一直开着直到 Ctrl-C";

pub const QUICK_HELP: &str = "把能玩的版本发给别人，不用先登录。

用法：
  playtest ./dist                 发布目录，得到链接和二维码
  playtest 5173                   分享本地服务；关闭终端后链接下线
  playtest ./dist -m \"改了新手引导\" 更新同一个链接
  playtest ./dist --seats 10      放到广场，找 10 位试玩者

发布以后：
  playtest open [目录或作品标识]  打开链接（默认当前目录）
  playtest card [目录或作品标识]  再存一张邀请卡（默认当前目录）
  playtest ls                    查看作品
  playtest login                 用 GitHub 登录，保留作品

更多用法：playtest --help；单条命令：playtest card --help
不自动构建项目；请发布包含 index.html 的导出目录。";

/// 帮助的最后一段。写脚本的人靠退出码分流，不该去猜错误文案。
const EXIT_CODES: &str = "退出码：
  0  做成了
  1  没预料到的错误
  2  命令写错了
  3  需要登录，或者身份失效了
  4  网络不通
  5  服务端出错
  6  给的东西有问题（目录不在、超限、不像导出物）
  7  配额用完了";

#[derive(Debug, Parser)]
#[command(
    name = "playtest",
    version,
    about = "一条命令，把你手上这个能玩的版本放到别人面前",
    before_help = EXAMPLES,
    after_help = EXIT_CODES,
    // 不用 clap 的 args_conflicts_with_subcommands：它会让「参数之后不再认子命令」，
    // 于是 `playtest --json ls` 里的 ls 被当成叫 ls 的目录。目录和子命令不能一起用这条规则
    // 改在 main 里检查（`UploadArgs::any_set`），报的错也能说得更像人话。
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(flatten)]
    pub upload: UploadArgs,
    #[command(subcommand)]
    pub command: Option<Command>,

    /// JSON 输出，说明和进度走 stderr；本地端口分享持续输出事件，其他命令输出一个结果
    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long = "gate", hide = true, global = true)]
    pub legacy_gate: Option<String>,
}

#[derive(Debug, Default, Args)]
pub struct UploadArgs {
    /// 要发出去的目录（./dist），或者本地开发服务器的端口（5173）
    #[arg(value_name = "目录或端口")]
    pub target: Option<String>,

    /// 作品名，玩家点开链接时看到（默认用目录名）
    #[arg(short = 'n', long = "name", value_name = "作品名")]
    #[arg(help_heading = "发布信息")]
    pub name: Option<String>,

    /// 这版改了什么、想让人重点看什么；邀请函、关注通知、广场卡上都是这一句（最多 280 字）
    #[arg(short = 'm', long = "note", value_name = "一句话")]
    #[arg(help_heading = "发布信息")]
    pub note: Option<String>,

    /// 作品长期简介，省略时沿用已有内容（最多 140 字）
    #[arg(long, value_name = "一句话")]
    #[arg(help_heading = "邀请与招募")]
    pub summary: Option<String>,

    /// 封面图（PNG / JPEG / WebP，2 MB 以内）；邀请函的第一眼，分享出去时的卡片图
    #[arg(long, value_name = "图片文件")]
    #[arg(help_heading = "邀请与招募")]
    pub cover: Option<PathBuf>,

    /// 展示在公开广场（默认不展示）；不影响持有链接者访问
    #[arg(long)]
    #[arg(help_heading = "邀请与招募")]
    pub public: bool,

    /// 想找几位试玩者；在广场上标「正在找人测」，邀请函和邀请卡上会写出来，留了名字的人算加入；蕴含 --public
    #[arg(long, value_name = "人数", value_parser = clap::value_parser!(u32).range(1..))]
    #[arg(help_heading = "邀请与招募")]
    pub seats: Option<u32>,

    /// 你的群：QQ 群、微信群二维码页、Discord、Telegram 都行；玩家在邀请函和反馈之后看到「开发者的群」
    #[arg(long, value_name = "链接")]
    #[arg(help_heading = "邀请与招募")]
    pub community: Option<String>,

    /// 下载邀请卡到指定路径（默认不下载）；写 - 也不下载
    #[arg(long = "card", value_name = "路径")]
    #[arg(help_heading = "输出")]
    pub card: Option<String>,

    /// 让页面跑在隔离环境里。auto（默认，看出要多线程就开）、on、off
    #[arg(long, value_name = "开关", default_value = "auto", value_parser = parse_isolation)]
    #[arg(help_heading = "运行设置")]
    pub isolated: Isolation,

    /// 导航请求找不到页面时回到 index.html；资源请求不回退（前端路由用）
    #[arg(long)]
    #[arg(help_heading = "运行设置")]
    pub spa: bool,

    /// 更新指定作品；写 new 新建链接（默认更新这个目录上次发布的作品）
    #[arg(long, value_name = "作品标识或 new")]
    #[arg(help_heading = "发布信息")]
    pub to: Option<String>,

    /// 检查说「传上去一定打不开」时也照传（比如你知道 index.html 不在最外层是故意的）
    #[arg(short = 'y', long = "yes")]
    #[arg(help_heading = "运行设置")]
    pub force: bool,

    /// 目录照常上传，目录里没有的路径（/api/…、WebSocket）走隧道到你电脑的这个端口（带后端的小应用用这个）
    #[arg(long, value_name = "端口", value_parser = clap::value_parser!(u16).range(1..))]
    #[arg(help_heading = "运行设置")]
    pub backend: Option<u16>,

    /// API 服务地址（自托管与开发内部使用；也可以设置环境变量 PLAYTEST_API）
    #[arg(long, global = true, hide = true, value_name = "网址")]
    pub api: Option<String>,

    /// 不画二维码
    #[arg(long = "no-qr")]
    #[arg(help_heading = "输出")]
    pub no_qr: bool,
}

impl UploadArgs {
    /// 用户有没有写任何上传相关的东西。子命令（ls / rm / open / mcp …）和这些不能一起用。
    pub fn any_set(&self) -> bool {
        self.target.is_some()
            || self.name.is_some()
            || self.note.is_some()
            || self.summary.is_some()
            || self.cover.is_some()
            || self.public
            || self.seats.is_some()
            || self.community.is_some()
            || self.card.is_some()
            || self.isolated != Isolation::Auto
            || self.spa
            || self.to.is_some()
            || self.force
            || self.backend.is_some()
            || self.no_qr
    }

    /// 要不要标「正在找人测」。
    ///
    /// `--seats` 就是它：说了想找 10 位试玩者，却不在广场上标出来，那 10 个人不会自己出现。
    /// 想让人重点看什么写在 `--note` 里，和「这版改了什么」是同一句话。
    pub fn seeking(&self) -> bool {
        self.seats.is_some()
    }

    /// 邀请卡存到哪。`None` 是默认位置，`Some(None)` 是「不要存」（`--card -`）。
    pub fn card_path(&self) -> Option<Option<PathBuf>> {
        match self.card.as_deref() {
            None => None,
            Some("-") => Some(None),
            Some(path) => Some(Some(PathBuf::from(path))),
        }
    }

    /// 要不要存邀请卡。
    pub fn wants_card(&self) -> bool {
        self.card.as_deref().is_some_and(|path| path != "-")
    }

    /// 要不要放到广场上。求测必须先在广场上，否则没人看得到这个标。
    pub fn wants_plaza(&self) -> bool {
        self.public || self.seeking()
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 用 GitHub 登录一次；之后发的作品留下来，不再 24 小时后失效
    Login,

    /// 撤销当前令牌并清除本机凭据，不删除作品；匿名作品请先登录接管
    Logout {
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// 这个令牌是谁、什么档位、这台机器上有几个作品
    Whoami,

    /// 线上这一版里到底有哪些文件：每个路径、多大、内容哈希
    Files {
        #[arg(value_name = "目录或作品标识", default_value = ".")]
        target: String,

        /// 看某一版（默认看玩家现在看到的那一版）
        #[arg(long, value_name = "版本")]
        version: Option<String>,
    },

    /// 列出这台机器上发过的作品
    Ls,

    /// 删掉一个作品，它的链接立刻打不开
    Rm {
        #[arg(value_name = "目录或作品标识")]
        slug: String,

        /// 不问一句，直接删
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// 打印链接，并用系统浏览器打开
    Open {
        #[arg(value_name = "目录或作品标识", default_value = ".")]
        target: String,
    },

    /// 列出一个作品发过的每一版，标出玩家现在看到的是哪一版
    Versions {
        #[arg(value_name = "目录或作品标识", default_value = ".")]
        target: String,
    },

    /// 让玩家看到的换回某一版；清单都在，一个字节不用重传
    Rollback {
        #[arg(value_name = "目录或作品标识")]
        target: String,

        /// 例如 3 或 v3
        #[arg(value_name = "版本")]
        version: String,
    },

    /// 下载邀请卡，存成 PNG；发到群里，别人长按识别就能玩
    Card {
        #[arg(value_name = "目录或作品标识", default_value = ".")]
        target: String,

        /// 存到哪（默认存到当前目录，叫「<作品名>-邀请卡.png」）
        #[arg(long, value_name = "路径")]
        out: Option<PathBuf>,
    },

    /// 把一个作品从广场上拿下来；它的链接照常能开
    Unlist {
        #[arg(value_name = "目录或作品标识")]
        slug: String,
    },

    /// 作为 MCP server 跑起来，让 Cursor / Claude Code 里的助手直接发链接
    Mcp {
        /// 不跑服务，只打印一段可以粘进编辑器配置的 JSON
        #[arg(long)]
        setup: bool,
    },
}

/// 跨源隔离（COOP / COEP）开不开。
///
/// 三态而不是 `--isolated` / `--no-isolated` 两个互斥布尔：它本来就是一个三选一的问题，
/// 两个布尔要靠 `conflicts_with` 才能表达「不能同时说」，而且读命令的人得先知道
/// 「都不写」是第三种情况（REWRITE §3.1）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Isolation {
    /// 看出这个构建要多线程（`.wasm` 里有共享内存）就开。
    #[default]
    Auto,
    On,
    Off,
}

fn parse_isolation(s: &str) -> Result<Isolation, String> {
    match s {
        "auto" => Ok(Isolation::Auto),
        "on" | "true" | "yes" => Ok(Isolation::On),
        "off" | "false" | "no" => Ok(Isolation::Off),
        other => Err(format!(
            "只认 auto、on、off 三个词，不认识「{other}」。不写就是 auto：看出要多线程才开。"
        )),
    }
}

/// 第一个位置参数是什么。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// 一个要上传的目录。
    Dir(PathBuf),
    /// 一个本地端口，隧道模式。
    Port(u16),
    /// 全是数字，但不是一个能用的端口号。
    PortOutOfRange(String),
}

/// 全是数字且落在 1–65535 就当端口，其余一律当目录。
///
/// 想上传一个名字全是数字的目录，写成 `./5173` 就行——带了斜杠或点就不是纯数字了。
pub fn classify(raw: &str) -> Target {
    let all_digits = !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_digit());
    if all_digits {
        return match raw.parse::<u32>() {
            Ok(n) if (1..=65535).contains(&n) => Target::Port(n as u16),
            _ => Target::PortOutOfRange(raw.to_string()),
        };
    }
    Target::Dir(PathBuf::from(raw))
}

/// `--api`、环境变量、配置里上次记下的地址、默认线上，按这个顺序。
pub fn api_base(flag: Option<&str>, saved: Option<&str>) -> String {
    pick_api(flag, std::env::var("PLAYTEST_API").ok().as_deref(), saved)
}

fn pick_api(flag: Option<&str>, env: Option<&str>, saved: Option<&str>) -> String {
    for candidate in [flag, env, saved] {
        if let Some(url) = candidate.map(str::trim).filter(|s| !s.is_empty()) {
            return url.trim_end_matches('/').to_string();
        }
    }
    DEFAULT_API.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_in_port_range_are_tunnel_mode() {
        assert_eq!(classify("5173"), Target::Port(5173));
        assert_eq!(classify("1"), Target::Port(1));
        assert_eq!(classify("65535"), Target::Port(65535));
    }

    #[test]
    fn digits_outside_port_range_are_neither() {
        assert_eq!(classify("0"), Target::PortOutOfRange("0".into()));
        assert_eq!(classify("65536"), Target::PortOutOfRange("65536".into()));
        // 长到 u32 都装不下也不能 panic。
        assert_eq!(
            classify("99999999999999999999"),
            Target::PortOutOfRange("99999999999999999999".into())
        );
    }

    #[test]
    fn anything_else_is_a_directory() {
        for raw in [
            "./dist",
            "dist",
            "/tmp/build",
            "5173/",
            "./5173",
            "web-export",
        ] {
            assert_eq!(classify(raw), Target::Dir(PathBuf::from(raw)), "{raw}");
        }
    }

    #[test]
    fn cli_parses_upload_and_subcommands() {
        let cli =
            Cli::try_parse_from(["playtest", "./dist", "-n", "小球", "--isolated=on"]).unwrap();
        assert_eq!(cli.upload.target.as_deref(), Some("./dist"));
        assert_eq!(cli.upload.name.as_deref(), Some("小球"));
        assert_eq!(cli.upload.isolated, Isolation::On);
        assert!(cli.command.is_none());

        let cli = Cli::try_parse_from(["playtest", "ls"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Ls)));

        let cli = Cli::try_parse_from(["playtest", "rm", "brisk-otter-41", "-y"]).unwrap();
        match cli.command {
            Some(Command::Rm { slug, yes, .. }) => {
                assert_eq!(slug, "brisk-otter-41");
                assert!(yes);
            }
            other => panic!("解析成了 {other:?}"),
        }
    }

    #[test]
    fn plaza_flags_parse() {
        let cli = Cli::try_parse_from([
            "playtest",
            "./dist",
            "--public",
            "-m",
            "新手引导看得懂吗",
            "--summary",
            "三关五分钟",
            "--cover",
            "cover.png",
        ])
        .unwrap();
        assert!(cli.upload.public);
        assert_eq!(cli.upload.note.as_deref(), Some("新手引导看得懂吗"));
        assert_eq!(cli.upload.summary.as_deref(), Some("三关五分钟"));
        assert_eq!(cli.upload.cover, Some(PathBuf::from("cover.png")));
        assert!(cli.upload.any_set());

        let cli = Cli::try_parse_from(["playtest", "unlist", "brisk-otter-41"]).unwrap();
        assert!(
            matches!(cli.command, Some(Command::Unlist { slug, .. }) if slug == "brisk-otter-41")
        );
    }

    #[test]
    fn seats_implies_looking_for_testers_and_therefore_the_plaza() {
        let cli = Cli::try_parse_from(["playtest", "./dist", "--seats", "10"]).unwrap();
        assert_eq!(cli.upload.seats, Some(10));
        assert!(!cli.upload.public, "没写 --public");
        assert!(cli.upload.seeking(), "说了想找 10 位试玩者就是在找人测");
        assert!(
            cli.upload.wants_plaza(),
            "找人测得先在广场上，否则没人看得到"
        );
        assert!(cli.upload.any_set());

        // 什么都没说的时候，广场一动不动。
        let plain = Cli::try_parse_from(["playtest", "./dist"]).unwrap();
        assert!(!plain.upload.seeking());
        assert!(!plain.upload.wants_plaza());

        // --public 单独给不算找人测；-m 只是这一版的话，不把作品挂上广场。
        let opened = Cli::try_parse_from(["playtest", "./dist", "--public"]).unwrap();
        assert!(!opened.upload.seeking() && opened.upload.wants_plaza());
        let noted = Cli::try_parse_from(["playtest", "./dist", "-m", "看新手引导"]).unwrap();
        assert!(!noted.upload.seeking() && !noted.upload.wants_plaza());
    }

    #[test]
    fn seats_takes_a_number() {
        assert!(Cli::try_parse_from(["playtest", "./dist", "--seats", "十"]).is_err());
        assert!(Cli::try_parse_from(["playtest", "./dist", "--seats", "-3"]).is_err());
        assert!(Cli::try_parse_from(["playtest", "./dist", "--seats", "0"]).is_err());
    }

    #[test]
    fn community_is_a_flag_of_its_own_and_does_not_touch_the_plaza() {
        let cli =
            Cli::try_parse_from(["playtest", "./dist", "--community", "https://t.me/x"]).unwrap();
        assert_eq!(cli.upload.community.as_deref(), Some("https://t.me/x"));
        assert!(cli.upload.any_set());
        assert!(
            !cli.upload.wants_plaza(),
            "填了个群号不该把作品挂到广场上去"
        );
    }

    /// 一个参数管「存到哪」和「不要存」两件事：`-` 是 shell 里「不要文件」的老约定，
    /// 比 `--card-out` 加一个 `--no-card` 少一个参数，也不可能自相矛盾（REWRITE §3.1）。
    #[test]
    fn the_card_can_go_somewhere_else_or_nowhere() {
        let cli =
            Cli::try_parse_from(["playtest", "./dist", "--card", "~/Desktop/卡.png"]).unwrap();
        assert_eq!(
            cli.upload.card_path(),
            Some(Some(PathBuf::from("~/Desktop/卡.png")))
        );
        assert!(cli.upload.wants_card());
        assert!(cli.upload.any_set());

        let cli = Cli::try_parse_from(["playtest", "./dist", "--card", "-"]).unwrap();
        assert_eq!(cli.upload.card_path(), Some(None));
        assert!(!cli.upload.wants_card());
        assert!(cli.upload.any_set());

        // 不写就是默认位置。
        let plain = Cli::try_parse_from(["playtest", "./dist"]).unwrap();
        assert_eq!(plain.upload.card_path(), None);
        assert!(!plain.upload.wants_card());
    }

    #[test]
    fn isolation_is_one_three_way_switch_not_two_booleans() {
        let auto = Cli::try_parse_from(["playtest", "./dist"]).unwrap();
        assert_eq!(auto.upload.isolated, Isolation::Auto);
        assert!(!auto.upload.any_set() || auto.upload.target.is_some());

        for (word, want) in [
            ("auto", Isolation::Auto),
            ("on", Isolation::On),
            ("off", Isolation::Off),
        ] {
            let cli =
                Cli::try_parse_from(["playtest", "./dist", &format!("--isolated={word}")]).unwrap();
            assert_eq!(cli.upload.isolated, want, "{word}");
        }
        assert!(Cli::try_parse_from(["playtest", "./dist", "--isolated=maybe"]).is_err());
        // 两个互斥布尔没了，也就不存在「同时说」这件事。
        assert!(Cli::try_parse_from(["playtest", "./dist", "--no-isolated"]).is_err());
    }

    /// 发到哪个作品是一个问题，不是两个（原来的 `--site` 与 `--new` 能同时给）。
    #[test]
    fn where_to_publish_is_one_question() {
        let cli = Cli::try_parse_from(["playtest", "./dist", "--to", "keen-yak-7"]).unwrap();
        assert_eq!(cli.upload.to.as_deref(), Some("keen-yak-7"));

        let fresh = Cli::try_parse_from(["playtest", "./dist", "--to", "new"]).unwrap();
        assert_eq!(fresh.upload.to.as_deref(), Some("new"));

        assert!(Cli::try_parse_from(["playtest", "./dist", "--new"]).is_err());
        assert!(Cli::try_parse_from(["playtest", "./dist", "--site", "a"]).is_err());
    }

    #[test]
    fn card_takes_a_slug_or_a_directory() {
        let cli = Cli::try_parse_from(["playtest", "card", "brisk-otter-41"]).unwrap();
        match cli.command {
            Some(Command::Card { target, out, .. }) => {
                assert_eq!(target, "brisk-otter-41");
                assert!(out.is_none());
            }
            other => panic!("解析成了 {other:?}"),
        }

        let cli =
            Cli::try_parse_from(["playtest", "card", "./dist", "--out", "/tmp/卡.png"]).unwrap();
        match cli.command {
            Some(Command::Card { target, out, .. }) => {
                assert_eq!(target, "./dist");
                assert_eq!(out, Some(PathBuf::from("/tmp/卡.png")));
            }
            other => panic!("解析成了 {other:?}"),
        }

        let current = Cli::try_parse_from(["playtest", "card"]).unwrap();
        assert!(matches!(current.command, Some(Command::Card { target, .. }) if target == "."));

        // 关注数并进了 ls：单开一条命令只为看一个数字，不值一个命令名（REWRITE §3.1）。
        assert!(Cli::try_parse_from(["playtest", "followers", "brisk-otter-41"]).is_err());
    }

    #[test]
    fn backend_is_a_port_and_counts_as_an_upload_flag() {
        let cli = Cli::try_parse_from(["playtest", "./dist", "--backend", "3000"]).unwrap();
        assert_eq!(cli.upload.backend, Some(3000));
        assert!(cli.upload.any_set());
        assert!(Cli::try_parse_from(["playtest", "./dist", "--backend", "70000"]).is_err());
        assert!(Cli::try_parse_from(["playtest", "./dist", "--backend", "api"]).is_err());
        assert!(Cli::try_parse_from(["playtest", "./dist", "--backend", "0"]).is_err());
    }

    #[test]
    fn bare_invocation_has_no_target() {
        let cli = Cli::try_parse_from(["playtest"]).unwrap();
        assert!(cli.upload.target.is_none());
        assert!(cli.command.is_none());
    }

    #[test]
    fn api_is_global_and_not_a_publish_option() {
        for words in [
            vec!["playtest", "--api", "http://localhost:8787", "ls"],
            vec!["playtest", "ls", "--api", "http://localhost:8787"],
        ] {
            let cli = Cli::try_parse_from(words).unwrap();
            assert_eq!(cli.upload.api.as_deref(), Some("http://localhost:8787"));
            assert!(!cli.upload.any_set());
        }
    }

    #[test]
    fn mutations_still_require_explicit_targets() {
        for name in ["rm", "unlist", "rollback"] {
            assert!(Cli::try_parse_from(["playtest", name]).is_err());
        }
    }

    #[test]
    fn control_plane_flag_then_env_then_saved_then_online() {
        assert_eq!(
            pick_api(Some("http://a/"), Some("http://b"), Some("http://c")),
            "http://a"
        );
        assert_eq!(
            pick_api(None, Some(" http://b/ "), Some("http://c")),
            "http://b"
        );
        assert_eq!(pick_api(None, Some(""), Some("http://c/")), "http://c");
        assert_eq!(pick_api(None, None, None), DEFAULT_API);
    }
}
