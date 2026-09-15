//! 命令行的形状。
//!
//! 第一个位置参数身兼两职：目录（上传）和端口号（隧道）。分辨规则放在 [`classify`] 里，
//! 是个纯函数，好测也好改。

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// 没有 `--api` 也没有环境变量 `PLAYTEST_API` 时用它。
pub const DEFAULT_API: &str = playtest_common::DEVELOPER_API_URL;

/// 帮助的头两行。第一次用的人只看这两行就够开始了。
const EXAMPLES: &str = "playtest ./dist            Publish this directory; get a link and a QR code
playtest 5173              Share your local dev server, until you press Ctrl-C";

pub const QUICK_HELP: &str = "Put a playable build in front of people. No login needed.

Usage:
  playtest ./dist                  Publish a directory; get a link and a QR code
  playtest 5173                    Share a local server; the link goes offline with your terminal
  playtest ./dist -m \"New tutorial\" Update the same link
  playtest ./dist --seats 10       List it on the Plaza and look for 10 testers

After publishing:
  playtest open [dir or project]   Open the link (defaults to the current directory)
  playtest card [dir or project]   Save another invite card (defaults to the current directory)
  playtest ls                      List your projects
  playtest login                   Sign in with GitHub to keep your projects

More options: playtest --help; one command: playtest card --help
Nothing is built for you. Publish the export directory that has index.html in it.";

/// 帮助的最后一段。写脚本的人靠退出码分流，不该去猜错误文案。
const EXIT_CODES: &str = "Exit codes:
  0  Done
  1  Unexpected error
  2  Something wrong with the command
  3  Login needed, or the identity expired
  4  Network unreachable
  5  Server error
  6  Something wrong with what you gave (no such directory, over a limit, not an export)
  7  Out of quota";

#[derive(Debug, Parser)]
#[command(
    name = "playtest",
    version,
    about = "One command to put the build you have in front of other people",
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

    /// JSON output; narration and progress go to stderr. Port sharing streams events, every other command prints one result
    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long = "gate", hide = true, global = true)]
    pub legacy_gate: Option<String>,
}

#[derive(Debug, Default, Args)]
pub struct UploadArgs {
    /// Directory to publish (./dist), or the port your local dev server listens on (5173)
    #[arg(value_name = "DIR|PORT")]
    pub target: Option<String>,

    /// Project name, shown when someone opens the link (defaults to the directory name)
    #[arg(short = 'n', long = "name", value_name = "NAME")]
    #[arg(help_heading = "Publishing")]
    pub name: Option<String>,

    /// What changed in this version, or what you want people to look at; used on the invitation page, in follow notifications and on the Plaza card (280 chars max)
    #[arg(short = 'm', long = "note", value_name = "TEXT")]
    #[arg(help_heading = "Publishing")]
    pub note: Option<String>,

    /// Longer-lived description of the project; kept as it was when omitted (140 chars max)
    #[arg(long, value_name = "TEXT")]
    #[arg(help_heading = "Invite and recruiting")]
    pub summary: Option<String>,

    /// Cover image (PNG / JPEG / WebP, up to 2 MB); the first thing on the invitation page, and the picture on the share card
    #[arg(long, value_name = "IMAGE")]
    #[arg(help_heading = "Invite and recruiting")]
    pub cover: Option<PathBuf>,

    /// Show it on the public Plaza (off by default); does not change who can open the link
    #[arg(long)]
    #[arg(help_heading = "Invite and recruiting")]
    pub public: bool,

    /// How many testers you want; marks the project seeking testers on the Plaza, says so on the invitation page and the invite card, and anyone who leaves a name counts as joined. Implies --public
    #[arg(long, value_name = "COUNT", value_parser = clap::value_parser!(u32).range(1..))]
    #[arg(help_heading = "Invite and recruiting")]
    pub seats: Option<u32>,

    /// Your group chat: Discord, Telegram, a Matrix room, a page with a QR code — anything with a URL. Players see it on the invitation page and after leaving feedback
    #[arg(long, value_name = "URL")]
    #[arg(help_heading = "Invite and recruiting")]
    pub community: Option<String>,

    /// Save the invite card to this path (not saved by default); - also means don't save
    #[arg(long = "card", value_name = "PATH")]
    #[arg(help_heading = "Output")]
    pub card: Option<String>,

    /// Serve the page cross-origin isolated. auto (default, on when the build looks threaded), on, off
    #[arg(long, value_name = "MODE", default_value = "auto", value_parser = parse_isolation)]
    #[arg(help_heading = "Runtime")]
    pub isolated: Isolation,

    /// Fall back to index.html for navigation requests that match no file; asset requests don't fall back (for client-side routing)
    #[arg(long)]
    #[arg(help_heading = "Runtime")]
    pub spa: bool,

    /// Publish to this project; write new for a fresh link (defaults to whatever this directory published last time)
    #[arg(long, value_name = "PROJECT|new")]
    #[arg(help_heading = "Publishing")]
    pub to: Option<String>,

    /// Upload anyway when a check says it definitely won't open (say you nested index.html on purpose)
    #[arg(short = 'y', long = "yes")]
    #[arg(help_heading = "Runtime")]
    pub force: bool,

    /// Upload the directory as usual, and tunnel every path it doesn't contain (/api/…, WebSocket) to this port on your machine — for small apps with a backend
    #[arg(long, value_name = "PORT", value_parser = clap::value_parser!(u16).range(1..))]
    #[arg(help_heading = "Runtime")]
    pub backend: Option<u16>,

    /// API address (for self-hosting and internal development; the PLAYTEST_API environment variable works too)
    #[arg(long, global = true, hide = true, value_name = "URL")]
    pub api: Option<String>,

    /// Don't draw the QR code
    #[arg(long = "no-qr")]
    #[arg(help_heading = "Output")]
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
    /// Sign in with GitHub once; projects published after that stay, instead of expiring in 24 hours
    Login,

    /// Revoke the current token and clear local credentials. Projects are not deleted; sign in first to take over anonymous ones
    Logout {
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Who this token belongs to, which plan, and how many projects are on this machine
    Whoami,

    /// What is actually in a published version: every path, its size and its content hash
    Files {
        #[arg(value_name = "DIR|PROJECT", default_value = ".")]
        target: String,

        /// Look at one version (defaults to the one players see now)
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,
    },

    /// List the projects published from this machine
    Ls,

    /// Delete a project; its link stops working immediately
    Rm {
        #[arg(value_name = "DIR|PROJECT")]
        slug: String,

        /// Delete without asking
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Print the link and open it in your browser
    Open {
        #[arg(value_name = "DIR|PROJECT", default_value = ".")]
        target: String,
    },

    /// List every version of a project and mark the one players see now
    Versions {
        #[arg(value_name = "DIR|PROJECT", default_value = ".")]
        target: String,
    },

    /// Put players back on an earlier version; the manifests are already there, so nothing is uploaded again
    Rollback {
        #[arg(value_name = "DIR|PROJECT")]
        target: String,

        /// For example 3 or v3
        #[arg(value_name = "VERSION")]
        version: String,
    },

    /// Download the invite card as a PNG; send it to a chat and anyone who scans it can play
    Card {
        #[arg(value_name = "DIR|PROJECT", default_value = ".")]
        target: String,

        /// Where to save it (defaults to <project name>-invite-card.png in the current directory)
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },

    /// Take a project off the Plaza; its link keeps working
    Unlist {
        #[arg(value_name = "DIR|PROJECT")]
        slug: String,
    },

    /// Run as an MCP server, so an assistant in Cursor / Claude Code can publish links directly
    Mcp {
        /// Don't run the server; just print JSON you can paste into your editor's config
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
            "Only auto, on and off. Don't know \"{other}\". Leaving it out means auto: on only when the build looks threaded."
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
