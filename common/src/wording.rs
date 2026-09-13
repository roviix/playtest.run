//! 玩家会读到、而且不止一处会读到的几个用词。定一次，门禁页、邀请卡、分享描述照抄——
//! 同一张卡和它点开的那一页要一字不差（DESIGN §3.3、§3.4）。
//!
//! 这里只放「选词」的逻辑，不放句子。整句在渲染处拼，文案表（多语言）落地时再搬。

use crate::manifest::WorkKind;

/// 「邀请你试玩 / 体验 / 阅读 / 观看」（DESIGN §3.3）。
///
/// 用 AI 写小东西的人做的多数不是游戏——抽样里工具、微型 SaaS、生成器占近八成——
/// 对着一个数据看板说「邀请你试玩」是把话说错了。认不出引擎时也说「体验」：宁可少说一句。
pub fn invite_verb(kind: WorkKind, is_game: bool) -> &'static str {
    match kind {
        WorkKind::Article => "邀请你阅读",
        WorkKind::Video => "邀请你观看",
        WorkKind::Web if is_game => "邀请你试玩",
        WorkKind::Web => "邀请你体验",
    }
}

/// 按钮、卡片悬停等不带「邀请你」的短动作。
pub fn action_verb(kind: WorkKind, is_game: bool) -> &'static str {
    match kind {
        WorkKind::Article => "阅读",
        WorkKind::Video => "观看",
        WorkKind::Web if is_game => "试玩",
        WorkKind::Web => "体验",
    }
}

/// 招募与反馈署名所用的体验者称呼。
pub fn audience_noun(kind: WorkKind, is_game: bool) -> &'static str {
    match kind {
        WorkKind::Article => "读者",
        WorkKind::Video => "观众",
        WorkKind::Web if is_game => "试玩者",
        WorkKind::Web => "体验者",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_work_kind_uses_the_action_people_will_really_take() {
        assert_eq!(invite_verb(WorkKind::Web, true), "邀请你试玩");
        assert_eq!(invite_verb(WorkKind::Web, false), "邀请你体验");
        assert_eq!(invite_verb(WorkKind::Article, false), "邀请你阅读");
        assert_eq!(invite_verb(WorkKind::Video, false), "邀请你观看");
        assert_eq!(action_verb(WorkKind::Article, false), "阅读");
        assert_eq!(audience_noun(WorkKind::Video, false), "观众");
    }
}
