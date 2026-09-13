//! `playtest-edge`：起一个明文 HTTP 监听。
//!
//! 明文是本机阶段的有意选择：Chrome 与 Firefox 把 `*.localhost` 当作安全上下文，
//! 陀螺仪、摄像头、WebGPU、SharedArrayBuffer 这些只有 HTTPS 才有的能力在本机也能试
//! （DESIGN §4.7）。泛域名证书是第四周的事，这个二进制里没有任何 TLS。

use std::sync::Arc;

use anyhow::Context;
use playtest_edge::{router, App, Config};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer().with_target(false))
        .init();

    let config = Config::from_env()?;
    let listen = config.listen;
    let events_path = config.events_path();
    let suffix = config.host_suffix.clone();
    let scheme = config.public_scheme.clone();
    let api_internal_url = config.api_internal_url.clone();
    let edge_ingest_token = config.edge_ingest_token.clone();

    let app = Arc::new(App::try_new(config)?);
    // S3 凭据、桶或网络不对时，边缘必须明确起不来，不能把所有作品伪装成 404。
    app.sites.store().probe_read().await?;
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("绑不上 {listen}，端口可能被占了"))?;

    tracing::info!("边缘在 http://{listen} 上（明文，没有 TLS）");
    tracing::info!("作品从 {} 读", app.sites.store().description());
    tracing::info!("事件写到 {}", events_path.display());
    tracing::info!("一个作品就是一个 {scheme}://<slug>.{suffix}");
    match api_internal_url {
        Some(api) => {
            let token = edge_ingest_token.expect("Config 已检查内网 API 与事件凭据成对出现");
            tracing::info!(
                "事件每 {} 秒送一批到 {api}",
                playtest_edge::ship::INTERVAL.as_secs()
            );
            tokio::spawn(playtest_edge::ship::Shipper::new(events_path, api, token).run());
        }
        None => tracing::info!("没设 PLAYTEST_API_INTERNAL_URL，事件只留在本地文件里，不送控制面"),
    }

    axum::serve(listener, router(app))
        .await
        .context("监听中断")?;
    Ok(())
}
