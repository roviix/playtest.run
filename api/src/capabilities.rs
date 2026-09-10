//! `capabilities.json`：这台机器能做什么（DESIGN §4.5）。
//!
//! 边缘和玩家页面读它来决定「留邮箱」和「浏览器通知」这两栏要不要显示。
//! 起来时写一次就够——这两件事在进程活着的时候不会变。

use playtest_common::capabilities::{Capabilities, SCHEMA};

use crate::clock;
use crate::state::AppState;

pub async fn publish(state: &AppState) {
    let capabilities = Capabilities {
        schema: SCHEMA,
        generated_at: clock::now_string(),
        email: state.notify().email_on(),
        push_public_key: Some(state.notify().push_public_key().to_string()),
    };
    if let Err(e) = state.store().put_capabilities(&capabilities).await {
        tracing::warn!(error = %e, "写 capabilities.json 失败");
    }
}
