//! 命令行的形状。
//!
//! 第一个位置参数身兼两职：目录（上传）和端口号（隧道）。分辨规则放在 [`classify`] 里，
//! 是个纯函数，好测也好改。

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use playtest_common::manifest::GateMode;

/// 没有 `--api` 也没有环境变量 `PLAYTEST_API` 时用它。
pub const DEFAULT_API: &str = "https://api.playtest.sh";

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
            || self.isolated
            || self.no_isolated
            || self.spa
            || self.gate != GateMode::Once
            || self.site.is_some()
            || self.new
            || self.api.is_some()
            || self.no_qr
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 登录（还没做好）
    Login,

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
        for raw in ["./dist", "dist", "/tmp/build", "5173/", "./5173", "web-export"] {
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
