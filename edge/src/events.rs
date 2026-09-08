//! 第一层数据（DESIGN §3.4）：不用改游戏一行代码就有的那部分。
//!
//! v0.1 先落本地一个 JSONL 文件，第三周再按 60 秒批量送控制面。
//! **不记 IP**：DESIGN §3.4 明说不收集精确位置，IP 是最容易顺手记下的那一样，
//! 所以这里连字段都不留。

use std::path::{Path, PathBuf};

use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::io::AsyncWriteExt;

/// UA 与 Referer 截断长度。日志是一行一条，别让某个客户端的超长头把行撑爆。
const MAX_FIELD_CHARS: usize = 512;
/// 举报正文留得长一些，但也有上限。
const MAX_DETAIL_CHARS: usize = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 出了门禁页。
    GateView,
    /// 玩家点了「开始」。
    Start,
    /// 出了一页 HTML（门禁过了之后每一次）。
    HtmlView,
    Report,
    /// 这个 slug 这一小时的流量用完了，边缘把它关上了（DESIGN §4.8）。
    BreakerTrip,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::GateView => "gate_view",
            Kind::Start => "start",
            Kind::HtmlView => "html_view",
            Kind::Report => "report",
            Kind::BreakerTrip => "breaker_trip",
        }
    }
}

#[derive(Debug, Serialize)]
struct Line<'a> {
    ts: String,
    #[serde(rename = "type")]
    kind: &'static str,
    slug: &'a str,
    version: u32,
    sid: &'a str,
    ua: &'a str,
    referer: &'a str,
    wechat: bool,
    /// 举报才有：下拉里选的那一项，和玩家自己写的一段。
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<&'a str>,
    /// 熔断才有：滚动一小时里已经出了多少字节，上限是多少。
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u64>,
}

/// 一条事件里跟着请求走的那部分。
#[derive(Debug, Default, Clone)]
pub struct Visitor {
    pub sid: String,
    pub ua: String,
    pub referer: String,
    pub wechat: bool,
}

#[derive(Debug, Clone)]
pub struct EventLog {
    path: PathBuf,
}

impl EventLog {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn append(
        &self,
        kind: Kind,
        slug: &str,
        version: u32,
        visitor: &Visitor,
        reason: Option<&str>,
        detail: Option<&str>,
    ) {
        let line = Line {
            ts: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_default(),
            kind: kind.as_str(),
            slug,
            version,
            sid: &visitor.sid,
            ua: clip(&visitor.ua, MAX_FIELD_CHARS),
            referer: clip(&visitor.referer, MAX_FIELD_CHARS),
            wechat: visitor.wechat,
            reason: reason.map(|r| clip(r, MAX_FIELD_CHARS)),
            detail: detail.map(|d| clip(d, MAX_DETAIL_CHARS)),
            bytes: None,
            limit: None,
        };
        if let Err(err) = self.write(&line).await {
            tracing::warn!(path = %self.path.display(), %err, "事件写不进去");
        }
    }

    /// 熔断单独一个入口：它比别的事件多两个数字，而且**一轮只写一条**——
    /// 熔断本来就发生在被刷的时候，每个被拦的请求都写一行等于自己给自己放大攻击
    /// （谁在拦的时候写日志，磁盘就替攻击者遭殃）。由调用方保证只在第一次触发时调。
    pub async fn breaker_trip(
        &self,
        slug: &str,
        version: u32,
        visitor: &Visitor,
        bytes: u64,
        limit: u64,
    ) {
        let line = Line {
            ts: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_default(),
            kind: Kind::BreakerTrip.as_str(),
            slug,
            version,
            sid: &visitor.sid,
            ua: clip(&visitor.ua, MAX_FIELD_CHARS),
            referer: clip(&visitor.referer, MAX_FIELD_CHARS),
            wechat: visitor.wechat,
            reason: None,
            detail: None,
            bytes: Some(bytes),
            limit: Some(limit),
        };
        if let Err(err) = self.write(&line).await {
            tracing::warn!(path = %self.path.display(), %err, "熔断事件写不进去");
        }
    }

    async fn write(&self, line: &Line<'_>) -> std::io::Result<()> {
        let mut buf = serde_json::to_vec(line).map_err(std::io::Error::other)?;
        buf.push(b'\n');
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        file.write_all(&buf).await?;
        // tokio 的 File 自己带一层缓冲，不 flush 的话字节可能还没交给内核就随 File 一起 drop 了。
        // 不做 fsync：这是统计事件，掉电丢最后几条可以接受，每条都同步落盘换来的延迟不值。
        file.flush().await
    }
}

fn clip(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// 微信内置浏览器的 UA 里一定有这个词，X5 内核的能力缺口都从这里判断（DESIGN §5）。
pub fn is_wechat(user_agent: &str) -> bool {
    user_agent.contains("MicroMessenger")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wechat_ua() {
        assert!(is_wechat(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) MicroMessenger/8.0.49"
        ));
        assert!(!is_wechat("Mozilla/5.0 (Macintosh) Chrome/140.0"));
    }

    #[test]
    fn clips_long_fields_on_char_boundary() {
        let long = "中".repeat(MAX_FIELD_CHARS + 10);
        assert_eq!(clip(&long, MAX_FIELD_CHARS).chars().count(), MAX_FIELD_CHARS);
        assert_eq!(clip("短", MAX_FIELD_CHARS), "短");
    }

    #[tokio::test]
    async fn appends_one_json_line_per_event() {
        let dir = tempfile::tempdir().unwrap();
        let log = EventLog::new(dir.path().join("nested/edge-events.jsonl"));
        let visitor = Visitor {
            sid: "abc".into(),
            ua: "curl/8".into(),
            referer: String::new(),
            wechat: false,
        };
        log.append(Kind::GateView, "brisk-otter-41", 7, &visitor, None, None)
            .await;
        log.append(Kind::Report, "brisk-otter-41", 7, &visitor, Some("phishing"), Some("假的"))
            .await;

        let body = std::fs::read_to_string(log.path()).unwrap();
        let lines: Vec<_> = body.lines().collect();
        assert_eq!(lines.len(), 2);

        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first["type"], "gate_view");
        assert_eq!(first["slug"], "brisk-otter-41");
        assert_eq!(first["version"], 7);
        assert_eq!(first["sid"], "abc");
        assert_eq!(first["wechat"], false);
        assert!(first.get("reason").is_none());
        assert!(first["ts"].as_str().unwrap().contains('T'));
        // IP 不记，字段都不留。
        assert!(first.get("ip").is_none());

        let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second["type"], "report");
        assert_eq!(second["reason"], "phishing");
        assert_eq!(second["detail"], "假的");
        // 只有熔断带这两个数字。
        assert!(second.get("bytes").is_none());
    }

    #[tokio::test]
    async fn breaker_trip_records_how_far_over_it_went() {
        let dir = tempfile::tempdir().unwrap();
        let log = EventLog::new(dir.path().join("edge-events.jsonl"));
        log.breaker_trip("brisk-otter-41", 7, &Visitor::default(), 3_221_225_472, 3_221_225_472)
            .await;

        let body = std::fs::read_to_string(log.path()).unwrap();
        let line: serde_json::Value = serde_json::from_str(body.trim()).unwrap();
        assert_eq!(line["type"], "breaker_trip");
        assert_eq!(line["slug"], "brisk-otter-41");
        assert_eq!(line["bytes"], 3_221_225_472u64);
        assert_eq!(line["limit"], 3_221_225_472u64);
        assert!(line.get("reason").is_none());
        assert!(line.get("ip").is_none());
    }
}
