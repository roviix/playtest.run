//! 隧道令牌的签名密钥（DESIGN §4.5：边缘只验签名、不回源）。
//!
//! 启动时定下一把 Ed25519 私钥，来源按优先级是「环境变量 → 数据目录里的密钥文件 → 现生成一把」；
//! 公钥每次启动都覆盖写进对象存储的 `keys/tunnel.pub`，边缘从那里读。私钥只留在控制面这台机器上，
//! 不进数据库、不进日志、不进任何响应。
//!
//! 换掉私钥的代价：已经发出去的隧道令牌全部验不过，CLI 会重新要一个（TTL 一小时，最多等这么久）。

use std::io::Write;
use std::path::Path;

use anyhow::{anyhow, Context};
use playtest_common::tunnel::{key_files, SigningKey};

/// 直接给一把私钥（base64url 的 32 字节种子）。容器里不挂盘、或者以后有第二个控制面实例要签同一把钥匙时用它。
///
/// 不放进 [`crate::Config`]：那个结构体是 `Debug` 的，私钥进去之后随便哪行调试日志都会把它印出来。
pub const SIGNING_KEY_ENV: &str = "PLAYTEST_TUNNEL_SIGNING_KEY";

pub fn key_from_env() -> Option<String> {
    match std::env::var(SIGNING_KEY_ENV) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

/// 定下这次启动用的私钥，并把公钥发布给边缘。`from_env` 是 [`SIGNING_KEY_ENV`] 的值。
pub fn load_or_create(
    data_dir: &Path,
    store_root: &Path,
    from_env: Option<&str>,
) -> anyhow::Result<SigningKey> {
    let key = match from_env {
        Some(raw) => SigningKey::from_base64(raw).ok_or_else(|| {
            anyhow!("{SIGNING_KEY_ENV} 不是一把隧道签名私钥：要 base64url 编码的 32 字节。不设这个变量就用数据目录里的密钥文件。")
        })?,
        None => from_file(&data_dir.join(key_files::SIGNING_KEY_FILE))?,
    };
    publish_verifying_key(store_root, &key)?;
    Ok(key)
}

/// 密钥文件在就读它，不在就生成一把写进去。
fn from_file(path: &Path) -> anyhow::Result<SigningKey> {
    if let Some(key) = read_key(path)? {
        return Ok(key);
    }
    if let Some(key) = write_new_key(path)? {
        tracing::info!(path = %path.display(), "已生成新的隧道签名密钥");
        return Ok(key);
    }
    // 两个控制面同时起在同一个数据目录上，写输了：先落地的那把算数。
    read_key(path)?.ok_or_else(|| anyhow!("隧道签名密钥 {} 刚写好就不见了", path.display()))
}

/// `Ok(None)` 表示文件还不存在；内容不对是错误，不是「当作没有」——
/// 悄悄换一把新的等于让所有在线的隧道同时掉线，还查不出原因。
fn read_key(path: &Path) -> anyhow::Result<Option<SigningKey>> {
    match std::fs::read_to_string(path) {
        Ok(text) => SigningKey::from_base64(&text).map(Some).ok_or_else(|| {
            anyhow!(
                "{} 里不是一把隧道签名私钥，这个文件被改坏了。删掉它会重新生成一把，代价是已经发出去的隧道令牌全部作废。",
                path.display()
            )
        }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(anyhow::Error::new(err)
            .context(format!("读不了隧道签名密钥 {}", path.display()))),
    }
}

/// `Ok(None)` 表示这一步撞上了已经存在的文件。
fn write_new_key(path: &Path) -> anyhow::Result<Option<SigningKey>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("建不了目录 {}", parent.display()))?;
    }

    let key = SigningKey::generate();
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        // 私钥不能是同机其他用户读得到的。权限要在创建那一刻就定下，事后 chmod 中间有窗口。
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    match options.open(path) {
        Ok(mut file) => {
            let line = format!("{}\n", key.to_base64());
            file.write_all(line.as_bytes())
                .and_then(|()| file.sync_all())
                .with_context(|| format!("写不了隧道签名密钥 {}", path.display()))?;
            Ok(Some(key))
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(None),
        Err(err) => {
            Err(anyhow::Error::new(err).context(format!("建不了隧道签名密钥 {}", path.display())))
        }
    }
}

/// 公钥每次启动都重写一遍：换过私钥、或者对象存储被清过之后，边缘下一次读到的就是对的那把。
///
/// `FsStore` 只认清单、指针和 blob 这三种对象，公钥这一个键直接落文件系统。
fn publish_verifying_key(store_root: &Path, key: &SigningKey) -> anyhow::Result<()> {
    let path = store_root.join(key_files::VERIFYING_KEY_OBJECT);
    let parent = path.parent().expect("对象键至少有一级目录");
    std::fs::create_dir_all(parent).with_context(|| format!("建不了目录 {}", parent.display()))?;

    // 先写临时文件再改名：边缘随时可能在读，不能让它读到半行。
    let tmp = parent.join(format!(".tunnel.pub.{}.tmp", std::process::id()));
    let line = format!("{}\n", key.verifying_key().to_base64());
    std::fs::write(&tmp, line).with_context(|| format!("写不了 {}", tmp.display()))?;
    std::fs::rename(&tmp, &path).with_context(|| format!("改不了名字 {}", path.display()))?;

    tracing::info!(path = %path.display(), "隧道公钥已发布，边缘从这里读");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_key_file_says_what_to_do() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(key_files::SIGNING_KEY_FILE);
        std::fs::write(&path, "这不是密钥").unwrap();

        let err = load_or_create(dir.path(), &dir.path().join("store"), None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("删掉它"), "{err}");
        // 报错之后文件要原样留着，删不删由人决定。
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "这不是密钥");
    }

    #[test]
    fn broken_env_key_names_the_variable() {
        let dir = tempfile::tempdir().unwrap();
        let err = load_or_create(dir.path(), &dir.path().join("store"), Some("not-a-key"))
            .unwrap_err()
            .to_string();
        assert!(err.contains(SIGNING_KEY_ENV), "{err}");
        assert!(
            !dir.path().join(key_files::SIGNING_KEY_FILE).exists(),
            "环境变量给了钥匙就不该再生成一把落盘"
        );
    }
}
