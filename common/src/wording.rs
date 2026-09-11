//! 玩家会读到、而且不止一处会读到的几个用词。定一次，门禁页、邀请卡、分享描述照抄——
//! 同一张卡和它点开的那一页要一字不差（DESIGN §3.3、§3.4）。
//!
//! 这里只放「选词」的逻辑，不放句子。整句在渲染处拼，文案表（多语言）落地时再搬。

/// 「邀请你试玩」还是「邀请你体验」（DESIGN §3.3）。
///
/// 用 AI 写小东西的人做的多数不是游戏——抽样里工具、微型 SaaS、生成器占近八成——
/// 对着一个数据看板说「邀请你试玩」是把话说错了。认不出引擎时也说「体验」：宁可少说一句。
pub fn invite_verb(is_game: bool) -> &'static str {
    if is_game {
        "邀请你试玩"
    } else {
        "邀请你体验"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn games_are_played_and_everything_else_is_tried() {
        assert_eq!(invite_verb(true), "邀请你试玩");
        assert_eq!(invite_verb(false), "邀请你体验");
    }
}
