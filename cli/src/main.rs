//! `playtest`：一条命令，把你手上这个能玩的版本放到别人面前。
//!
//! 退出码分层，见 [`output::Code`]；`--json` 的输出形状也在那个模块的开头。

mod args;
mod clock;
mod client;
mod config;
mod inspect;
mod mcp;
mod output;
mod scan;
mod sites;
mod tunnel;
mod ui;
mod upload;

use std::process::ExitCode;

use anyhow::Result;
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};

use args::{Cli, Command, Target};

/// 一个还没做好的功能。说清楚现在能做什么，脚本靠退出码 2 和 `code: not_implemented` 分流。
fn not_yet(message: &str) -> anyhow::Error {
    output::not_implemented(message)
}

fn main() -> ExitCode {
    // 「几秒」要从最早的一刻开始算（DESIGN §3.2），所以这是第一行。
    output::begin();
    // 时区要在起线程之前问，拿到了后面各处都能显示本地时间。
    clock::init_local_offset();
    client::install_crypto_provider();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => return report_parse_outcome(e),
    };
    // 命令行解析出来的才作数：`-m "--json"` 这种在 begin() 里会被当成开了机器模式。
    output::set_json(cli.json);

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => return output::report_failure(&output::classify(&anyhow::anyhow!(
            "起不来后台任务：{e}"
        ))),
    };

    match runtime.block_on(dispatch(cli)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => output::report_failure(&output::classify(&e)),
    }
}

/// clap 自己的退出码约定就是 2 = 用法错，和我们的分层一致，不用接管。
/// 要接管的是往哪儿写：`--json` 模式下 stdout 只能有那一个对象，帮助全文得装进去。
fn report_parse_outcome(e: clap::Error) -> ExitCode {
    let asked_for_help = matches!(
        e.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    );
    if !output::is_json() {
        let _ = e.print();
        return if asked_for_help {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(output::Code::Usage.exit())
        };
    }

    let rendered = e.render().to_string();
    if asked_for_help {
        output::help(rendered);
        return ExitCode::SUCCESS;
    }
    eprintln!("{rendered}");
    output::report_failure(&output::classify(&output::usage(first_line(&rendered))))
}

/// clap 的用法错第一行是「error: …」，后面跟着 Usage 和一句提示。JSON 里的 `message`
/// 只要那一行，全文已经在 stderr 上了。
fn first_line(rendered: &str) -> String {
    rendered
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("命令写错了")
        .trim_start_matches("error: ")
        .to_string()
}

async fn dispatch(cli: Cli) -> Result<()> {
    let machine = output::is_json();
    if cli.command.is_some() && cli.upload.any_set() {
        return Err(output::usage(
            "子命令（ls / rm / open / mcp）不和要发出去的目录一起用。要发目录就只写 playtest ./dist；\
             要用子命令就把目录和 --name 这类参数去掉。",
        ));
    }
    match cli.command {
        Some(Command::Login) => Err(not_yet(
            "登录还没做好。现在每次运行拿到的是匿名链接，24 小时后失效。",
        )),
        Some(Command::Ls { api }) if machine => output::ls(api.as_deref()).await,
        Some(Command::Ls { api }) => sites::ls(api.as_deref()).await,
        Some(Command::Rm { slug, yes, api }) if machine => {
            output::rm(&slug, yes, api.as_deref()).await
        }
        Some(Command::Rm { slug, yes, api }) => sites::rm(&slug, yes, api.as_deref()).await,
        Some(Command::Open { target, api }) if machine => output::open(&target, api.as_deref()).await,
        Some(Command::Open { target, api }) => sites::open(&target, api.as_deref()).await,
        Some(Command::Mcp { setup, api }) => mcp::run(setup, api).await,
        None => run_default(cli).await,
    }
}

async fn run_default(cli: Cli) -> Result<()> {
    let Some(target) = cli.upload.target.clone() else {
        if output::is_json() {
            return Err(output::usage(
                "没说要发什么。给一个目录（playtest ./dist），或者一个本地端口（playtest 5173）。",
            ));
        }
        let mut command = Cli::command();
        let _ = command.print_help();
        println!();
        return Ok(());
    };

    match args::classify(&target) {
        Target::Dir(_) => {
            let report = upload::run(&cli.upload, &target).await?;
            output::report_upload(&report);
            Ok(())
        }
        Target::Port(port) => tunnel::run(&cli.upload, port).await,
        Target::PortOutOfRange(raw) => Err(output::usage(format!(
            "端口号要在 1 到 65535 之间，「{raw}」不是。如果这是一个目录的名字，写成 ./{raw}。"
        ))),
    }
}
