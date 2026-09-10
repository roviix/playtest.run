//! 命令行的形状。
//!
//! 第一个位置参数身兼两职：目录（上传）和端口号（隧道）。分辨规则放在 [`classify`] 里，
//! 是个纯函数，好测也好改。

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use playtest_common::manifest::GateMode;

/// 没有 `--api` 也没有环境变量 `PLAYTEST_API` 时用它。
pub const DEFAULT_API: &str = playtest_common::DEVELOPER_API_URL;

/// 帮助的头两行。第一次用的人只看这两行就够开始了。
const EXAMPLES: &str = "playtest ./dist            把这个目录发出去，拿到链接和二维码
playtest 5173              把本地开发服务器接出去，一直开着直到 Ctrl-C";

/// 帮助的最后一段。写脚本的人靠退出码分流，不该去猜错误文案。
const EXIT_CODES: &str = "退出码：
  0  做成了
  1  没预料到的错误
  2  命令写错了，或者要的功能还没做好
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

    /// 只往 stdout 输出一个 JSON 对象，说明和进度走 stderr（给脚本和 agent 用）
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Debug, Default, Args)]
pub struct UploadArgs {
    /// 要发出去的目录（./dist），或者本地开发服务器的端口（5173）
    #[arg(value_name = "目录或端口")]
    pub target: Option<String>,

    /// 作品名，玩家点开链接时看到（默认用目录名）
    #[arg(short = 'n', long = "name", value_name = "作品名")]
    pub name: Option<String>,

    /// 这版改了什么，玩家点开链接时看到
    #[arg(short = 'm', long = "note", value_name = "一句话")]
    pub note: Option<String>,

    /// 一句话介绍这个作品是什么；门禁页、分享卡片、广场卡片上都用（最多 140 字）
    #[arg(long, value_name = "一句话")]
    pub summary: Option<String>,

    /// 封面图（PNG / JPEG / WebP，2 MB 以内）；门禁页的第一眼，分享出去时的卡片图
    #[arg(long, value_name = "图片文件")]
    pub cover: Option<PathBuf>,

    /// 上传后放到广场（playtest.run 首页）上，路过的人点开就能玩；默认不放
    #[arg(long)]
    pub public: bool,

    /// 在广场上标「正在找人测」，并告诉来的人你想让他们重点看什么（最多 140 字）；蕴含 --public
    #[arg(long, value_name = "想让人看什么")]
    pub seek: Option<String>,

    /// 想找几位试玩者；门禁页和邀请卡上会写出来，留了名字的人算加入；蕴含 --seek
    #[arg(long, value_name = "人数")]
    pub seats: Option<u32>,

    /// 你的群：QQ 群、微信群二维码页、Discord、Telegram 都行；玩家在门禁页和反馈之后看到「开发者的群」
    #[arg(long, value_name = "链接")]
    pub community: Option<String>,

    /// 邀请卡存到哪（默认存到当前目录，叫「<作品名>-邀请卡.png」）
    #[arg(long = "card-out", value_name = "路径")]
    pub card_out: Option<PathBuf>,

    /// 不存邀请卡
    #[arg(long = "no-card", conflicts_with = "card_out")]
    pub no_card: bool,

    /// 让页面跑在隔离环境里；Godot 4 的线程导出需要这个才能运行
    #[arg(long)]
    pub isolated: bool,

    /// 就算看出这个构建要多线程也不开隔离（不加这个的话，没终端时会自动开）
    #[arg(long = "no-isolated", conflicts_with = "isolated")]
    pub no_isolated: bool,

    /// 找不到的路径都回到 index.html（前端路由用）
    #[arg(long)]
    pub spa: bool,

    /// 玩家进来前那一页什么时候出：once（默认）、always、never
    #[arg(long, value_name = "策略", default_value = "once", value_parser = parse_gate)]
    pub gate: GateMode,

    /// 发到指定的作品，而不是这个目录上次用的那个
    #[arg(long, value_name = "slug")]
    pub site: Option<String>,

    /// 新建一个作品，拿一个新链接
    #[arg(long)]
    pub new: bool,

    /// 检查说「传上去一定打不开」时也照传（比如你知道 index.html 不在最外层是故意的）
    #[arg(long)]
    pub force: bool,

    /// 目录照常上传，目录里没有的路径（/api/…、WebSocket）走隧道到你电脑的这个端口（带后端的小应用用这个）
    #[arg(long, value_name = "端口")]
    pub backend: Option<u16>,

    /// 控制面地址（也可以用环境变量 PLAYTEST_API）
    #[arg(long, value_name = "网址")]
    pub api: Option<String>,

    /// 不画二维码
    #[arg(long = "no-qr")]
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
            || self.seek.is_some()
            || self.seats.is_some()
            || self.community.is_some()
            || self.card_out.is_some()
            || self.no_card
            || self.isolated
            || self.no_isolated
            || self.spa
            || self.gate != GateMode::Once
            || self.site.is_some()
            || self.new
            || self.force
            || self.backend.is_some()
            || self.api.is_some()
            || self.no_qr
    }

    /// 要不要标「正在找人测」。
    ///
    /// `--seats` 蕴含它：说了想找 10 位试玩者，却不在广场上标出来，那 10 个人不会自己出现。
    /// 没写 `--seek` 的文案就只标一下，不编一句「想让你看什么」。
    pub fn seeking(&self) -> bool {
        self.seek.is_some() || self.seats.is_some()
    }

    /// 要不要放到广场上。求测必须先在广场上，否则没人看得到这个标。
    pub fn wants_plaza(&self) -> bool {
        self.public || self.seeking()
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 用 GitHub 登录一次；之后发的作品留下来，不再 24 小时后失效
    Login {
        /// 控制面地址（也可以用环境变量 PLAYTEST_API）
        #[arg(long, value_name = "网址")]
        api: Option<String>,
    },

    /// 列出这台机器上发过的作品
    Ls {
        /// 控制面地址（也可以用环境变量 PLAYTEST_API）
        #[arg(long, value_name = "网址")]
        api: Option<String>,
    },

    /// 删掉一个作品，它的链接立刻打不开
    Rm {
        #[arg(value_name = "slug")]
        slug: String,

        /// 不问一句，直接删
        #[arg(short = 'y', long = "yes")]
        yes: bool,

        #[arg(long, value_name = "网址")]
        api: Option<String>,
    },

    /// 打印链接，并用系统浏览器打开
    Open {
        #[arg(value_name = "slug 或目录")]
        target: String,

        #[arg(long, value_name = "网址")]
        api: Option<String>,
    },

    /// 列出一个作品发过的每一版，标出玩家现在看到的是哪一版
    Versions {
        #[arg(value_name = "slug 或目录")]
        target: String,

        #[arg(long, value_name = "网址")]
        api: Option<String>,
    },

    /// 让玩家看到的换回某一版；清单都在，一个字节不用重传
    Rollback {
        #[arg(value_name = "slug 或目录")]
        target: String,

        /// 例如 3 或 v3
        #[arg(value_name = "版本")]
        version: String,

        #[arg(long, value_name = "网址")]
        api: Option<String>,
    },

    /// 重新拿一张邀请卡，存成 PNG；发到群里，别人长按识别就能玩
    Card {
        #[arg(value_name = "slug 或目录")]
        target: String,

        /// 存到哪（默认存到当前目录，叫「<作品名>-邀请卡.png」）
        #[arg(long, value_name = "路径")]
        out: Option<PathBuf>,

        #[arg(long, value_name = "网址")]
        api: Option<String>,
    },

    /// 有多少人关注着这个作品；下一版发出去，他们会收到通知
    Followers {
        #[arg(value_name = "slug 或目录")]
        target: String,

        #[arg(long, value_name = "网址")]
        api: Option<String>,
    },

    /// 把一个作品从广场上拿下来；它的链接照常能开
    Unlist {
        #[arg(value_name = "slug")]
        slug: String,

        #[arg(long, value_name = "网址")]
        api: Option<String>,
    },

    /// 作为 MCP server 跑起来，让 Cursor / Claude Code 里的助手直接发链接
    Mcp {
        /// 不跑服务，只打印一段可以粘进编辑器配置的 JSON
        #[arg(long)]
        setup: bool,

        /// 控制面地址（也可以用环境变量 PLAYTEST_API）
        #[arg(long, value_name = "网址")]
        api: Option<String>,
    },
}

fn parse_gate(s: &str) -> Result<GateMode, String> {
    s.parse()
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

/// `--api`、环境变量、默认值，按这个顺序。
pub fn api_base(flag: Option<&str>) -> String {
    if let Some(url) = flag {
        return url.trim_end_matches('/').to_string();
    }
    match std::env::var("PLAYTEST_API") {
        Ok(url) if !url.trim().is_empty() => url.trim().trim_end_matches('/').to_string(),
        _ => DEFAULT_API.to_string(),
    }
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
        let cli = Cli::try_parse_from(["playtest", "./dist", "-n", "小球", "--isolated"]).unwrap();
        assert_eq!(cli.upload.target.as_deref(), Some("./dist"));
        assert_eq!(cli.upload.name.as_deref(), Some("小球"));
        assert!(cli.upload.isolated);
        assert_eq!(cli.upload.gate, GateMode::Once);
        assert!(cli.command.is_none());

        let cli = Cli::try_parse_from(["playtest", "ls"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Ls { .. })));

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
            "--seek",
            "新手引导看得懂吗",
            "--summary",
            "三关五分钟",
            "--cover",
            "cover.png",
        ])
        .unwrap();
        assert!(cli.upload.public);
        assert_eq!(cli.upload.seek.as_deref(), Some("新手引导看得懂吗"));
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

        // --seek 也蕴含上广场；--public 单独给不算找人测。
        let sought = Cli::try_parse_from(["playtest", "./dist", "--seek", "看新手引导"]).unwrap();
        assert!(sought.upload.seeking() && sought.upload.wants_plaza());
        let opened = Cli::try_parse_from(["playtest", "./dist", "--public"]).unwrap();
        assert!(!opened.upload.seeking() && opened.upload.wants_plaza());
    }

    #[test]
    fn seats_takes_a_number() {
        assert!(Cli::try_parse_from(["playtest", "./dist", "--seats", "十"]).is_err());
        assert!(Cli::try_parse_from(["playtest", "./dist", "--seats", "-3"]).is_err());
        // 范围（1..=MAX_SEATS）在 upload.rs 里查，那里能把话说得更清楚。
        assert!(Cli::try_parse_from(["playtest", "./dist", "--seats", "0"]).is_ok());
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

    #[test]
    fn the_card_can_go_somewhere_else_or_nowhere() {
        let cli =
            Cli::try_parse_from(["playtest", "./dist", "--card-out", "~/Desktop/卡.png"]).unwrap();
        assert_eq!(cli.upload.card_out, Some(PathBuf::from("~/Desktop/卡.png")));
        assert!(!cli.upload.no_card);
        assert!(cli.upload.any_set());

        let cli = Cli::try_parse_from(["playtest", "./dist", "--no-card"]).unwrap();
        assert!(cli.upload.no_card);
        assert!(cli.upload.card_out.is_none());
        assert!(cli.upload.any_set());

        // 「存到这里」和「不要存」一起说，是自相矛盾，当场拦下。
        assert!(
            Cli::try_parse_from(["playtest", "./dist", "--no-card", "--card-out", "a.png"])
                .is_err()
        );
    }

    #[test]
    fn card_and_followers_take_a_slug_or_a_directory() {
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

        let cli = Cli::try_parse_from(["playtest", "followers", "brisk-otter-41"]).unwrap();
        assert!(
            matches!(cli.command, Some(Command::Followers { target, .. }) if target == "brisk-otter-41")
        );

        // 不给作品就不知道要哪一张卡。
        assert!(Cli::try_parse_from(["playtest", "card"]).is_err());
        assert!(Cli::try_parse_from(["playtest", "followers"]).is_err());
    }

    #[test]
    fn backend_is_a_port_and_counts_as_an_upload_flag() {
        let cli = Cli::try_parse_from(["playtest", "./dist", "--backend", "3000"]).unwrap();
        assert_eq!(cli.upload.backend, Some(3000));
        assert!(cli.upload.any_set());
        assert!(Cli::try_parse_from(["playtest", "./dist", "--backend", "70000"]).is_err());
        assert!(Cli::try_parse_from(["playtest", "./dist", "--backend", "api"]).is_err());
    }

    #[test]
    fn gate_only_takes_the_three_words() {
        let cli = Cli::try_parse_from(["playtest", "./dist", "--gate", "always"]).unwrap();
        assert_eq!(cli.upload.gate, GateMode::Always);
        assert!(Cli::try_parse_from(["playtest", "./dist", "--gate", "sometimes"]).is_err());
    }

    #[test]
    fn bare_invocation_has_no_target() {
        let cli = Cli::try_parse_from(["playtest"]).unwrap();
        assert!(cli.upload.target.is_none());
        assert!(cli.command.is_none());
    }
}
