use anyhow::Context;
use playtest_api::{app, sweeper, AppState, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();

    let config = Config::from_env()?;
    let state = AppState::from_config(&config).await?;
    sweeper::spawn(state.clone());

    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .with_context(|| {
            format!(
                "绑不上 {}，可能已经有一个 playtest-api 在这个端口上跑了",
                config.listen
            )
        })?;

    tracing::info!(
        "playtest 控制面已启动：http://{}",
        listener.local_addr()?
    );
    tracing::info!("数据目录：{}", config.data_dir.display());
    tracing::info!("玩家链接：{}", config.site_url_template);

    axum::serve(listener, app(state))
        .with_graceful_shutdown(ctrl_c())
        .await
        .context("HTTP 服务退出得不干净")?;
    Ok(())
}

fn init_logging() {
    // tower-http 的每请求日志在 debug 级，本机开发默认打开——看不见请求就没法查问题。
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("playtest_api=info,tower_http=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

async fn ctrl_c() {
    if tokio::signal::ctrl_c().await.is_err() {
        // 收不到信号就一直服务，交给 kill。
        std::future::pending::<()>().await;
    }
    tracing::info!("收到停止信号，等手上的请求做完再退出");
}
