//! 推广位（DESIGN §3.11）：广场顶部最多两个位子，一周最多两个周报位。
//!
//! 这里只有排期与状态，没有收款——v0.1 的推广是我们手工赠送或线下收款后录进来的
//! （DESIGN §6），所以 `order_id` 是一个备注字段而不是支付网关的回执。

use playtest_common::boost::{Boost, BoostKind, BoostStatus, MAX_SLOTS};
use rusqlite::Connection;
use time::OffsetDateTime;

use crate::clock;
use crate::db;

/// 排期时最多往后找多少个候选时段。真实占用最多两个位子，正常两三次就能排上；
/// 这个上限是防手滑写出死循环，不是产品规则。
const MAX_LOOKAHEAD: usize = 64;

pub fn kind_as_db(kind: BoostKind) -> &'static str {
    match kind {
        BoostKind::Days3 => "days3",
        BoostKind::Days7 => "days7",
        BoostKind::Digest => "digest",
    }
}

pub fn kind_from_db(value: &str) -> BoostKind {
    match value {
        "days7" => BoostKind::Days7,
        "digest" => BoostKind::Digest,
        _ => BoostKind::Days3,
    }
}

pub fn status_as_db(status: BoostStatus) -> &'static str {
    match status {
        BoostStatus::Pending => "pending",
        BoostStatus::Live => "live",
        BoostStatus::Ended => "ended",
        BoostStatus::Rejected => "rejected",
    }
}

pub fn status_from_db(value: &str) -> BoostStatus {
    match value {
        "live" => BoostStatus::Live,
        "ended" => BoostStatus::Ended,
        "rejected" => BoostStatus::Rejected,
        _ => BoostStatus::Pending,
    }
}

pub fn to_boost(row: db::BoostRow) -> Boost {
    Boost {
        id: row.id,
        slug: row.slug,
        kind: kind_from_db(&row.kind),
        status: status_from_db(&row.status),
        granted: row.granted,
        starts_at: row.starts_at,
        ends_at: row.ends_at,
        created_at: row.created_at,
        order_id: row.order_id,
    }
}

/// 一档推广占多少天。周报那一档不按天算，它的窗口是「到下一次周报发出为止」。
pub fn days(kind: BoostKind) -> i64 {
    match kind {
        BoostKind::Days3 => 3,
        BoostKind::Days7 => 7,
        BoostKind::Digest => 0,
    }
}

/// 从 `from` 起，第一个还有空位的开始时刻。
///
/// 判据放宽成「整段窗口里重叠的推广数 < `MAX_SLOTS`」而不是逐时刻算并发数：
/// 这样只会把人排得更靠后，不会把位子卖超。位子只有两个，宽一点没人看得出来。
pub fn earliest_slot(
    conn: &Connection,
    kind: BoostKind,
    from: OffsetDateTime,
) -> rusqlite::Result<OffsetDateTime> {
    if kind == BoostKind::Digest {
        return Ok(from);
    }
    let span = time::Duration::days(days(kind));
    let mut start = from;
    for _ in 0..MAX_LOOKAHEAD {
        let end = start + span;
        let busy = db::boosts_overlapping(conn, &clock::format(start), &clock::format(end))?;
        if busy.len() < MAX_SLOTS {
            return Ok(start);
        }
        // 下一个候选：这些占位里最早结束的那一刻。没有结束时间的（理论上不会有）就往后跳一天。
        let next = busy
            .iter()
            .filter_map(|b| b.ends_at.as_deref())
            .filter_map(clock::parse)
            .filter(|t| *t > start)
            .min()
            .unwrap_or(start + time::Duration::days(1));
        start = next;
    }
    Ok(start)
}

/// 排期结果：什么时候上，什么时候下。
pub struct Window {
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
}

pub fn window(
    conn: &Connection,
    kind: BoostKind,
    requested: Option<OffsetDateTime>,
    now: OffsetDateTime,
) -> rusqlite::Result<Window> {
    let asked = requested.unwrap_or(now).max(now);
    let starts_at = match requested {
        // 指定了时间就照办：运营答应了人家周五上，就别自作主张挪走。
        Some(_) => asked,
        None => earliest_slot(conn, kind, asked)?,
    };
    let ends_at = match kind {
        // 周报位的窗口开到这一期周报发出为止，发完就下（DESIGN §3.11）。
        BoostKind::Digest => crate::notify::digest::next_run_at(starts_at),
        _ => starts_at + time::Duration::days(days(kind)),
    };
    Ok(Window { starts_at, ends_at })
}

/// 到点上位、到点下位。返回有没有变过——变了广场要重排。
pub fn advance(conn: &Connection, now: &str) -> rusqlite::Result<bool> {
    db::advance_boosts(conn, now)
}
