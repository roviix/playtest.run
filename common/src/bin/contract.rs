//! 把契约写成控制台用的 TS：`cargo run -p playtest-common --bin playtest-contract`。
//!
//! 不带参数写到 `console/src/generated/api.ts`；`--check` 只比对不写，不一致退出码 1（CI 用）。

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let check = std::env::args().any(|a| a == "--check");
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../console/src/generated/api.ts");
    let fresh = playtest_common::contract::typescript();
    let on_disk = std::fs::read_to_string(&target).unwrap_or_default();
    if on_disk == fresh {
        println!("{} 已是最新。", target.display());
        return ExitCode::SUCCESS;
    }
    if check {
        eprintln!(
            "{} 过时了：跑 `cargo run -p playtest-common --bin playtest-contract` 再提交。",
            target.display()
        );
        return ExitCode::FAILURE;
    }
    if let Some(dir) = target.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("建不了 {}：{e}", dir.display());
            return ExitCode::FAILURE;
        }
    }
    match std::fs::write(&target, fresh) {
        Ok(()) => {
            println!("已写 {}。", target.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("写不了 {}：{e}", target.display());
            ExitCode::FAILURE
        }
    }
}
