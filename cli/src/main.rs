//! `playtest`：一条命令，把你手上这个能玩的版本放到别人面前。
//!
//! 退出码分层，见 [`output::Code`]；`--json` 的输出形状也在那个模块的开头。

mod args;
mod card;
mod client;
mod clock;
mod commands;
mod config;
mod inspect;
mod login;
mod mcp;
mod output;
mod report;
mod scan;
mod session;
mod sites;
mod tunnel;
mod ui;
mod upload;

use std::process::ExitCode;

use anyhow::Result;
use clap::error::ErrorKind;
use clap::{CommandFactory, FromArgMatches};

use args::{Cli, Command, Target};

fn main() -> ExitCode {
    // 「几秒」要从最早的一刻开始算（DESIGN §3.2），所以这是第一行。
    output::begin();
    // 时区要在起线程之前问，拿到了后面各处都能显示本地时间。
    clock::init_local_offset();
    client::install_crypto_provider();

    let matches = match Cli::command().try_get_matches() {
        Ok(matches) => matches,
        Err(error) => return report_parse_outcome(error),
    };
    let explicit_publish_options = ["isolated"]
        .iter()
        .any(|name| matches.value_source(name) == Some(clap::parser::ValueSource::CommandLine));
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(e) => return report_parse_outcome(e),
    };
    // 命令行解析出来的才作数：`-m "--json"` 这种在 begin() 里会被当成开了机器模式。
    output::set_json(cli.json);
    if cli.legacy_gate.is_some() {
        return output::report_failure(&output::classify(&output::usage(
            "--gate 已撤出：主域始终展示邀请函，作品子域直接运行。请移除 --gate 及其值后重试。",
        )));
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => {
            return output::report_failure(&output::classify(&anyhow::anyhow!(
                "起不来后台任务：{e}"
            )))
        }
    };

    match runtime.block_on(dispatch(cli, explicit_publish_options)) {
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
        if e.kind() == ErrorKind::DisplayHelp && std::env::args().skip(1).eq(["-h"]) {
            println!("{}", args::QUICK_HELP);
            return ExitCode::SUCCESS;
        }
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

async fn dispatch(cli: Cli, explicit_publish_options: bool) -> Result<()> {
    let machine = output::is_json();
    let api = cli.upload.api.as_deref();
    if cli.command.is_some() && (cli.upload.any_set() || explicit_publish_options) {
        return Err(output::usage(
            "管理命令不和发布选项一起用。要发目录就只写 playtest ./dist；\
             要用子命令就把目录和 --name 这类参数去掉。",
        ));
    }
    // 这几条都产出一个 Report，说出来只有一处（`output::say`）——人话和 `--json`
    // 不可能只做到一半（REWRITE §3.1）。
    let report = match cli.command {
        Some(Command::Login) => return login::run(api).await,
        Some(Command::Mcp { setup }) => return mcp::run(setup, cli.upload.api).await,
        None => return run_default(cli, explicit_publish_options).await,

        Some(Command::Ls) => commands::ls(api).await?,
        Some(Command::Whoami) => commands::whoami(api).await?,
        Some(Command::Logout { yes }) => commands::logout(yes, api, !machine).await?,
        Some(Command::Files { target, version }) => {
            commands::files(&target, version.as_deref(), api).await?
        }
        Some(Command::Open { target }) => {
            // `--json` 下不弹浏览器：跑在 agent 或 CI 里多半没有浏览器，也不该抢焦点。
            commands::open(&target, api, !machine).await?
        }
        Some(Command::Rm { slug, yes }) => commands::rm(&slug, yes, api, !machine).await?,
        Some(Command::Versions { target }) => commands::versions(&target, api).await?,
        Some(Command::Rollback { target, version }) => {
            commands::rollback(&target, &version, api).await?
        }
        Some(Command::Unlist { slug }) => commands::unlist(&slug, api).await?,
        Some(Command::Card { target, out }) => commands::card(&target, out.as_deref(), api).await?,
    };
    output::say(&report);
    Ok(())
}

async fn run_default(cli: Cli, explicit_publish_options: bool) -> Result<()> {
    let Some(target) = cli.upload.target.clone() else {
        if output::is_json() || cli.upload.any_set() || explicit_publish_options {
            return Err(output::usage(
                "没说要发什么。给一个目录（playtest ./dist），或者一个本地端口（playtest 5173）。",
            ));
        }
        println!("{}", args::QUICK_HELP);
        return Ok(());
    };

    match args::classify(&target) {
        Target::Dir(_) => match cli.upload.backend {
            Some(port) => tunnel::run_hybrid(&cli.upload, &target, port).await,
            None => {
                let report = upload::run(&cli.upload, &target).await?;
                output::report_upload(&report);
                Ok(())
            }
        },
        Target::Port(_) if cli.upload.backend.is_some() => Err(output::usage(
            "--backend 只和目录一起用：playtest ./dist --backend 3000 是目录上传、目录里没有的路径走隧道。\
             要把整个开发服务器接出去，直接 playtest <端口> 就行。",
        )),
        Target::Port(port) => tunnel::run(&cli.upload, port).await,
        Target::PortOutOfRange(raw) => Err(output::usage(format!(
            "端口号要在 1 到 65535 之间，「{raw}」不是。如果这是一个目录的名字，写成 ./{raw}。"
        ))),
    }
}
