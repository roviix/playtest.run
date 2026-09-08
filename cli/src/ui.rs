//! 往终端写东西。
//!
//! 分工是固定的：**只有最终链接走 stdout**，一行，别的什么都没有，这样 `playtest ./dist | pbcopy`
//! 拿到的就是链接本身。进度、提示、二维码、报错全走 stderr。

use std::io::{IsTerminal, Write};

use playtest_common::limits::MIB;
use qrcode::render::unicode;
use qrcode::QrCode;

/// 一句说明。
pub fn say(line: &str) {
    eprintln!("{line}");
}

pub fn blank() {
    eprintln!();
}

/// 需要用户留意，但不影响继续跑。
pub fn warn(line: &str) {
    eprintln!("提醒：{line}");
}

/// 出错了，跑不下去。
pub fn fail(line: &str) {
    eprintln!("{line}");
}

/// 命令要回答的内容本身（`ls` 的列表、`open` 的链接），走 stdout。
pub fn out(line: &str) {
    println!("{line}");
}

/// 最终链接。单独一行，走 stdout。
pub fn link(url: &str) {
    // stderr 不带缓冲、stdout 管道里带，两边不同步就会错行；进出各刷一次。
    let _ = std::io::stderr().flush();
    println!("{url}");
    let _ = std::io::stdout().flush();
}

/// 终端里能画二维码吗。重定向到文件或管道里画了也是乱码。
pub fn can_draw_qr() -> bool {
    std::io::stdout().is_terminal() && std::io::stderr().is_terminal()
}

/// 二维码的原文。`--json` 模式下不画，把这段字交给调用方（agent 可以贴回对话里）。
///
/// 链接长到二维码装不下时返回 `None`——这不值得中断上传，链接本身已经给出去了。
pub fn qr_text(url: &str) -> Option<String> {
    let code = QrCode::new(url).ok()?;
    Some(
        code.render::<unicode::Dense1x2>()
            .quiet_zone(true)
            .module_dimensions(1, 1)
            .build(),
    )
}

/// 把画好的二维码写到 stderr 上。
pub fn print_qr(rendered: &str) {
    eprintln!("{rendered}");
}

/// `12.3 MB`。1024 进位。
pub fn bytes(n: u64) -> String {
    const KIB: u64 = 1024;
    if n < KIB {
        return format!("{n} B");
    }
    if n < MIB {
        return format!("{:.1} KB", n as f64 / KIB as f64);
    }
    format!("{:.1} MB", n as f64 / MIB as f64)
}

/// 在终端上问一句是不是。不是终端就返回 `None`，让调用方说清楚为什么没法问。
pub fn confirm(question: &str) -> Option<bool> {
    if !std::io::stdin().is_terminal() {
        return None;
    }
    eprint!("{question}");
    let _ = std::io::stderr().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return Some(false);
    }
    let answer = answer.trim().to_lowercase();
    Some(answer == "y" || answer == "yes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_sizes_read_like_a_file_manager() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(1023), "1023 B");
        assert_eq!(bytes(1024), "1.0 KB");
        assert_eq!(bytes(1536), "1.5 KB");
        assert_eq!(bytes(MIB), "1.0 MB");
        assert_eq!(bytes(12_897_484), "12.3 MB");
    }
}
