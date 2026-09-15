//! 把一封信交出去。三种实现，用哪一种由 `PLAYTEST_EMAIL_PROVIDER` 决定。
//!
//! 抽象成 trait 不是为了将来换：是为了本机能在不真发信的情况下走完整条队列
//! （DESIGN §4.10「先能看见，再能送到」）。

use crate::config::EmailProvider;

#[derive(Debug, Clone)]
pub struct Letter {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub html: String,
}

impl Letter {
    fn resend_payload(&self, from: &str) -> serde_json::Value {
        serde_json::json!({
            "from": from,
            "to": [self.to],
            "subject": self.subject,
            "text": self.body,
            "html": self.html,
        })
    }

    pub fn smtp_message(&self, from: &str) -> Result<lettre::Message, SendError> {
        let from = from.parse::<lettre::message::Mailbox>().map_err(|error| {
            SendError::Permanent(format!("the sender is not a valid mailbox: {error}"))
        })?;
        let to = self
            .to
            .parse::<lettre::message::Mailbox>()
            .map_err(|error| {
                SendError::Permanent(format!("the recipient is not a valid mailbox: {error}"))
            })?;
        lettre::Message::builder()
            .from(from)
            .to(to)
            .subject(&self.subject)
            .multipart(lettre::message::MultiPart::alternative_plain_html(
                self.body.clone(),
                self.html.clone(),
            ))
            .map_err(|error| SendError::Permanent(format!("could not build this email: {error}")))
    }
}

#[derive(Debug)]
pub enum SendError {
    /// 这次不行，下次也许行（网络、限流、5xx）。队列会退避重试。
    Retry(String),
    /// 这封信本身有问题（地址不合法、被拒收），重试多少次都一样。
    Permanent(String),
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retry(m) | Self::Permanent(m) => f.write_str(m),
        }
    }
}

pub type SendResult = Result<(), SendError>;

#[async_trait::async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, from: &str, letter: &Letter) -> SendResult;
}

/// 整封信打进日志。第一次在自己机器上跑的人看到的就是它：终端里能读到完整的正文和链接，
/// 复制出来就能点。
pub struct LogMailer;

#[async_trait::async_trait]
impl Mailer for LogMailer {
    async fn send(&self, from: &str, letter: &Letter) -> SendResult {
        tracing::info!(
            "\n---- 这封信没有真的发出去（PLAYTEST_EMAIL_PROVIDER=log）----\n\
             发件人: {from}\n收件人: {to}\n主题: {subject}\n\n{body}\n\
             ---- 信到这里为止 ----",
            to = letter.to,
            subject = letter.subject,
            body = letter.body,
        );
        Ok(())
    }
}

/// [Resend](https://resend.com) 的 `POST /emails`。挑它是因为一个 API key 就能发，
/// 域名验证在他们控制台做，不用我们管 SMTP 的那一堆。
pub struct ResendMailer {
    http: reqwest::Client,
    api_key: String,
}

impl ResendMailer {
    pub fn new(http: reqwest::Client, api_key: String) -> Self {
        Self { http, api_key }
    }
}

#[async_trait::async_trait]
impl Mailer for ResendMailer {
    async fn send(&self, from: &str, letter: &Letter) -> SendResult {
        let body = letter.resend_payload(from);
        let response = self
            .http
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| SendError::Retry(format!("could not reach Resend: {e}")))?;

        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let detail = response.text().await.unwrap_or_default();
        let message = format!("Resend answered {status}: {}", detail.trim());
        // 4xx 是这封信自己的问题（地址不合法、域名没验），再试也一样；429 例外。
        if status.is_client_error() && status.as_u16() != 429 {
            Err(SendError::Permanent(message))
        } else {
            Err(SendError::Retry(message))
        }
    }
}

/// 自己的 SMTP。给不想把玩家邮箱交给第三方的自托管者（DESIGN §7）。
pub struct SmtpMailer {
    transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
}

impl SmtpMailer {
    /// `url` 形如 `smtps://用户名:密码@邮件服务器:465`。
    pub fn new(url: &str) -> anyhow::Result<Self> {
        let transport =
            lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::from_url(url)?.build();
        Ok(Self { transport })
    }
}

#[async_trait::async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, from: &str, letter: &Letter) -> SendResult {
        use lettre::AsyncTransport;

        let message = letter.smtp_message(from)?;
        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|e| SendError::Retry(format!("SMTP delivery failed: {e}")))
    }
}

/// 明确关掉发信时用。队列里不该再有邮件，真有也别悄悄丢——记一笔死信，运维看得见。
pub struct OffMailer;

#[async_trait::async_trait]
impl Mailer for OffMailer {
    async fn send(&self, _from: &str, _letter: &Letter) -> SendResult {
        Err(SendError::Permanent(
            "this server does not send email (PLAYTEST_EMAIL_PROVIDER=off)".to_string(),
        ))
    }
}

pub fn from_config(
    provider: &EmailProvider,
    http: reqwest::Client,
) -> anyhow::Result<Box<dyn Mailer>> {
    Ok(match provider {
        EmailProvider::Log => Box::new(LogMailer),
        EmailProvider::Off => Box::new(OffMailer),
        EmailProvider::Resend { api_key } => Box::new(ResendMailer::new(http, api_key.clone())),
        EmailProvider::Smtp { url } => Box::new(SmtpMailer::new(url)?),
    })
}

#[cfg(test)]
mod tests {
    use super::Letter;

    fn letter() -> Letter {
        Letter {
            to: "preview@example.com".to_string(),
            subject: "A new version".to_string(),
            body: "Plain text fallback".to_string(),
            html: "<p>HTML alternative</p>".to_string(),
        }
    }

    #[test]
    fn resend_receives_both_versions() {
        let letter = letter();
        let payload = letter.resend_payload("playtest.run <notice@playtest.run>");
        assert_eq!(payload["text"], letter.body);
        assert_eq!(payload["html"], letter.html);
        assert_eq!(payload["subject"], letter.subject);
        assert_eq!(payload["to"][0], letter.to);
    }

    #[test]
    fn smtp_is_multipart_with_plain_text_first() {
        let message = letter()
            .smtp_message("playtest.run <notice@playtest.run>")
            .unwrap();
        let wire = String::from_utf8(message.formatted()).unwrap();
        assert!(wire.contains("multipart/alternative"));
        let plain = wire.find("text/plain").unwrap();
        let html = wire.find("text/html").unwrap();
        assert!(plain < html);
        assert!(wire.contains("Plain text fallback"));
        assert!(wire.contains("<p>HTML alternative</p>"));
    }

    #[test]
    fn smtp_rejects_invalid_addresses_before_sending() {
        let mut letter = letter();
        letter.to = "not a mailbox".to_string();
        assert!(letter.smtp_message("notice@playtest.run").is_err());
        assert!(letter.smtp_message("not a mailbox").is_err());
    }
}
