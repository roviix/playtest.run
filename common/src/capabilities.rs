//! 控制面现在能做什么（DESIGN §4.5 的 `capabilities.json`）。
//!
//! 边缘不认识控制面，但门禁页要决定「有新版本时告诉我」那一行显示邮箱输入、浏览器通知按钮、
//! 还是什么都不显示——这取决于控制面有没有配邮件服务商、有没有 Web Push 的密钥。
//! 控制面启动时把答案写进对象存储，边缘读它。**做不到的就不显示，不解释**（AGENTS 第 4 条）。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const SCHEMA: u32 = 1;
pub const KEY: &str = "capabilities.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Capabilities {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub generated_at: String,
    /// 配了发信实现（`resend` / `smtp` / 本机的 `log`）。为假时门禁页与广场不出现邮箱输入框。
    #[serde(default)]
    pub email: bool,
    /// Web Push 的 VAPID 公钥（base64url）。没有就不出现「用浏览器通知」按钮。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub push_public_key: Option<String>,
}

impl Capabilities {
    /// 一样都做不了：门禁页和广场上「有新版本时告诉我」整行不出现。
    pub fn nothing(&self) -> bool {
        !self.email && self.push_public_key.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_means_no_follow_ui_at_all() {
        let caps: Capabilities = serde_json::from_str("{}").unwrap();
        assert!(caps.nothing());
        assert!(!caps.email);
        assert!(caps.push_public_key.is_none());
    }

    #[test]
    fn either_channel_is_enough_to_show_the_row() {
        let caps = Capabilities {
            email: true,
            ..Default::default()
        };
        assert!(!caps.nothing());
        let caps = Capabilities {
            push_public_key: Some("BPublicKey".into()),
            ..Default::default()
        };
        assert!(!caps.nothing());
    }
}
