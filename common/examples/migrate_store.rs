//! 把停止写入的旧 `store/` 复制到按环境变量配置的 S3，并逐对象读回校验。
//!
//! 用法：
//! `PLAYTEST_STORAGE_BACKEND=s3 PLAYTEST_S3_BUCKET=... PLAYTEST_S3_REGION=... cargo run -p playtest-common --example migrate_store -- /旧数据/store`

use std::path::PathBuf;

use anyhow::Context;
use playtest_common::store::{Store, StoreConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let source_root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| {
            anyhow::anyhow!("请在最后写旧对象存储目录，例如 /home/ubuntu/playtest-data/store")
        })?;
    anyhow::ensure!(
        source_root.is_dir(),
        "旧对象存储目录不存在或不是目录：{}",
        source_root.display()
    );

    let destination_config = StoreConfig::from_env(
        PathBuf::from(".data/store"),
        std::env::temp_dir().join("playtest-storage-migration"),
    )?;
    anyhow::ensure!(
        matches!(destination_config, StoreConfig::S3 { .. }),
        "迁移目标必须显式设置 PLAYTEST_STORAGE_BACKEND=s3；不会把旧目录复制到另一个本地目录"
    );

    let source = Store::new(&source_root);
    let destination = Store::from_config(&destination_config)?;
    destination
        .probe_write()
        .await
        .context("目标 S3 的写、读、删探针没有通过")?;

    eprintln!("源：{}", source.description());
    eprintln!("目标：{}", destination.description());
    eprintln!("迁移期间必须停止 API 写入；旧目录不会被修改或删除。");
    let report = source.migrate_to(&destination).await?;
    println!(
        "校验完成：{} 个对象，复制 {}，已一致 {}，共 {} 字节",
        report.objects, report.copied, report.skipped, report.bytes
    );
    Ok(())
}
