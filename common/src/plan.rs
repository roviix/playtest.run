//! 档位与它带来的上限（REWRITE §4.1）。
//!
//! 一个作品的档位跟着它的主人走。这里是**唯一**一处写着「哪个档能做什么」的地方：
//! CLI 在本地先按它拦一道并把原因说清楚，控制面提交时再校验一次，边缘按 `current.json` 里
//! 带下来的这一份切页。三处用同一份数字，报错信息才对得上。
//!
//! 为什么把上限做成数据而不是散在各处的 `if plan == Pro`：M4 要卖东西，卖的就是这张表里的行；
//! 表在一处，加一档（教育版、团队版）就是加一个分支，不是全仓找 `if`。
//!
//! **不做后付费**（REWRITE §4.1）：链接发出去之后开发者控制不了有多少人点开，
//! 任何后付费都会把「分享」变成财务风险。所以每一项都是硬上限，到顶硬停。

use serde::{Deserialize, Serialize};

use crate::limits::{GIB, MIB};

/// 三档。匿名也是一档——它有自己的上限，不是「Free 的特例」。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    /// 没登录就发的链接：24 小时，额度更紧。
    #[default]
    Anon,
    /// GitHub 登录之后。
    Free,
    /// $15 / 月或 $150 / 年。
    Pro,
}

impl Plan {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anon => "anon",
            Self::Free => "free",
            Self::Pro => "pro",
        }
    }

    pub fn limits(self) -> Limits {
        match self {
            Self::Anon => Limits {
                // 匿名链接只活 24 小时，所以它的「月配额」其实是这一条链接一辈子的总量。
                traffic_bytes: GIB,
                traffic_window: TrafficWindow::LinkLifetime,
                hourly_bytes: GIB / 2,
                active_projects: 1,
                kept_versions: 1,
                concurrent_players: 20,
                results_days: 1,
                max_version_bytes: 200 * MIB,
            },
            Self::Free => Limits {
                traffic_bytes: 10 * GIB,
                traffic_window: TrafficWindow::Monthly,
                hourly_bytes: 3 * GIB,
                active_projects: 3,
                kept_versions: 5,
                concurrent_players: 50,
                results_days: 30,
                max_version_bytes: 500 * MIB,
            },
            Self::Pro => Limits {
                traffic_bytes: 50 * GIB,
                traffic_window: TrafficWindow::Monthly,
                hourly_bytes: 3 * GIB,
                active_projects: 20,
                kept_versions: 50,
                concurrent_players: 200,
                results_days: 180,
                max_version_bytes: 2 * GIB,
            },
        }
    }

    /// 链接会不会到期。匿名固定 24 小时；登录之后默认长期，Pro 还能自己设 `--ttl`。
    pub fn link_expires(self) -> bool {
        matches!(self, Self::Anon)
    }

    /// 能不能自己挑链接的名字（`playtest rename`）。
    pub fn can_choose_slug(self) -> bool {
        matches!(self, Self::Pro)
    }

    /// 能不能去掉门禁页与邀请卡角上那个「由 playtest.run 提供」。
    pub fn can_remove_badge(self) -> bool {
        matches!(self, Self::Pro)
    }

    /// 能不能加口令或邀请名单（[`crate::project::Access`] 的后两档）。
    pub fn can_gate(self) -> bool {
        matches!(self, Self::Pro)
    }

    /// 能不能自己设 `--ttl`。
    pub fn can_set_ttl(self) -> bool {
        matches!(self, Self::Pro)
    }

    /// 能不能导出结果、比较版本。
    pub fn can_export(self) -> bool {
        matches!(self, Self::Pro)
    }

    /// 能带几位只读 reviewer（发行商、老师、评委看结果）。
    pub fn reviewer_seats(self) -> u32 {
        match self {
            Self::Pro => 3,
            _ => 0,
        }
    }

    /// 能不能买推广。匿名不能——责任要落到人（REWRITE §5.8）。
    pub fn can_buy_boost(self) -> bool {
        !matches!(self, Self::Anon)
    }
}

impl std::str::FromStr for Plan {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "anon" => Ok(Self::Anon),
            "free" => Ok(Self::Free),
            "pro" => Ok(Self::Pro),
            other => Err(format!("不认识的档位：{other}")),
        }
    }
}

/// 流量额度算在哪个窗口上。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrafficWindow {
    /// 这条链接一辈子（匿名链接只活 24 小时）。
    LinkLifetime,
    /// 自然月，每月 1 日 00:00 UTC 重置。
    Monthly,
}

/// 一个档位的全部上限。边缘拿到的是这一份（跟着 `current.json` 下来），不认识档位这个词。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    /// 窗口内出网字节上限。到顶硬停，不产生账单。
    pub traffic_bytes: u64,
    pub traffic_window: TrafficWindow,
    /// 每作品每小时出网上限，边缘本地判定、不回源。
    ///
    /// 月配额挡长期滥用，每小时熔断挡分钟级的 DDoS——SIMMER.io 2025-04 是被分钟级账单
    /// 打死的，月配额在那个尺度上完全无效，它一个月才结算一次。两个尺度缺一不可。
    pub hourly_bytes: u64,
    pub active_projects: u32,
    pub kept_versions: u32,
    /// 同一个作品同时在玩的人数上限。
    pub concurrent_players: u32,
    /// 会话、事件、反馈保留多少天。
    pub results_days: u32,
    /// 一个版本所有文件之和的上限。
    pub max_version_bytes: u64,
}

/// 配额此刻的状态。控制面算好写进 `live.json`，边缘据此决定给字节还是给一页说明。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaState {
    #[default]
    Ok,
    /// 用掉八成以上。控制台提醒一句，玩家侧一切照常。
    Warning,
    /// 到顶了。玩家看到一页如实的说明，开发者收到一封信。
    Exhausted,
}

impl QuotaState {
    /// 用了多少字节、上限多少 → 现在是什么状态。
    pub fn of(used: u64, limit: u64) -> Self {
        if limit == 0 || used >= limit {
            Self::Exhausted
        } else if used * 10 >= limit * 8 {
            Self::Warning
        } else {
            Self::Ok
        }
    }

    /// 还给不给字节。
    pub fn serving(self) -> bool {
        !matches!(self, Self::Exhausted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_step_up_is_never_a_step_down() {
        let anon = Plan::Anon.limits();
        let free = Plan::Free.limits();
        let pro = Plan::Pro.limits();
        assert!(anon.traffic_bytes < free.traffic_bytes);
        assert!(free.traffic_bytes < pro.traffic_bytes);
        assert!(anon.active_projects < free.active_projects);
        assert!(free.active_projects < pro.active_projects);
        assert!(free.kept_versions < pro.kept_versions);
        assert!(free.concurrent_players < pro.concurrent_players);
        assert!(free.results_days < pro.results_days);
    }

    #[test]
    fn only_pro_buys_the_things_we_sell() {
        for plan in [Plan::Anon, Plan::Free] {
            assert!(!plan.can_choose_slug());
            assert!(!plan.can_remove_badge());
            assert!(!plan.can_gate());
            assert!(!plan.can_set_ttl());
            assert!(!plan.can_export());
            assert_eq!(plan.reviewer_seats(), 0);
        }
        assert!(Plan::Pro.can_choose_slug());
        assert!(Plan::Pro.can_gate());
        assert_eq!(Plan::Pro.reviewer_seats(), 3);
    }

    #[test]
    fn anonymous_cannot_pay_because_nobody_is_accountable() {
        assert!(!Plan::Anon.can_buy_boost());
        assert!(Plan::Free.can_buy_boost());
        assert!(Plan::Pro.can_buy_boost());
    }

    #[test]
    fn only_anonymous_links_expire_by_themselves() {
        assert!(Plan::Anon.link_expires());
        assert!(!Plan::Free.link_expires());
        assert!(!Plan::Pro.link_expires());
    }

    #[test]
    fn quota_warns_at_eight_tenths_and_stops_at_the_top() {
        let limit = 10 * GIB;
        assert_eq!(QuotaState::of(0, limit), QuotaState::Ok);
        assert_eq!(QuotaState::of(7 * GIB, limit), QuotaState::Ok);
        assert_eq!(QuotaState::of(8 * GIB, limit), QuotaState::Warning);
        assert_eq!(QuotaState::of(limit, limit), QuotaState::Exhausted);
        assert_eq!(QuotaState::of(limit + 1, limit), QuotaState::Exhausted);
        assert!(QuotaState::Warning.serving(), "提醒不是停服");
        assert!(!QuotaState::Exhausted.serving());
    }

    #[test]
    fn a_plan_with_no_allowance_is_exhausted_not_infinite() {
        assert_eq!(QuotaState::of(0, 0), QuotaState::Exhausted);
    }

    #[test]
    fn plan_round_trips_through_its_db_string() {
        for plan in [Plan::Anon, Plan::Free, Plan::Pro] {
            assert_eq!(plan.as_str().parse::<Plan>().unwrap(), plan);
        }
    }

    #[test]
    fn anonymous_traffic_is_counted_over_the_link_not_the_month() {
        assert_eq!(
            Plan::Anon.limits().traffic_window,
            TrafficWindow::LinkLifetime
        );
        assert_eq!(Plan::Free.limits().traffic_window, TrafficWindow::Monthly);
    }
}
