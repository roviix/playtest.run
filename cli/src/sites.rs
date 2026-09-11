//! 开浏览器。
//!
//! 这个文件以前还放着 `ls / rm / open / versions / rollback / unlist / card / followers`
//! 的人话实现，而 `output.rs` 里放着同样几条的 `--json` 实现。两份各自演化的结果是
//! `rollback --json` 会往 stdout 吐裸链接、`versions --json` 什么都不吐。
//! 现在命令在 `commands.rs`，说出来在 `report.rs`，这里只剩下一件真正和系统打交道的事。

use std::process::{Command, Stdio};

use crate::ui;

pub fn launch_browser(url: &str) {
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
