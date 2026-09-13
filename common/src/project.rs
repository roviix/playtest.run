//! 一个原语：作品（REWRITE §2.1）。
//!
//! 在这一版之前，「一个作品长什么样」被写了三遍：`api::Listing`（控制台）、`plaza::PlazaItem`
//! （广场）、`live::SiteLive`（门禁页）。三份字段大幅重叠，都由控制面从同一批数据库列各拼一次，
//! 加一个字段要改六处——`boosts.reason` 那个「库里有列、契约没字段、控制台还在渲染它」的洞
//! 就是这条同步链断过一次的证据。
//!
//! 这里把它收成一份事实 [`Project`]（六组属性），另外两份是它的**投影**：
//!
//! | 投影 | 去哪 | 谁读 |
//! |---|---|---|
//! | [`Project`] | `GET /v1/projects/{slug}` | 控制台、CLI |
//! | [`ProjectCard`] | `plaza.json` 里的一项、控制台作品墙 | 边缘、控制台 |
//! | [`ProjectLive`] | `projects/<slug>/live.json` | 边缘（门禁页、邀请卡、分享页） |
//!
//! 投影只由 [`Project::card`] 与 [`Project::live`] 产出，没有第二条拼装路径。加一个字段
//! 只改 [`Project`] 和用得上它的那一个投影，控制台的 TS 类型跟着生成（REWRITE §5.2）。
//!
//! 卡上那「一件事实」（REWRITE §3.3）也在这里算：[`ProjectCard::fact`]。边缘和控制台
//! 各判一次的话，同一张卡在两个地方会说不同的话。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::boost::Boost;
use crate::manifest::{GateMode, WorkKind, GAME_ENGINES};

/// 一个作品的全部事实。控制面从库里读一次、拼一次，其余都是投影。
///
/// 六组属性的分法见 REWRITE §2.1：身份、交付、呈现、访问、上架、社会。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Project {
    // ---------------------------------------------------------------- 身份
    pub slug: String,
    /// 玩家点开的完整链接，例如 `https://brisk-otter-41.playtest.run`。
    pub url: String,
    pub owner: Owner,
    /// 跟着 owner 走，决定 [`crate::plan::Limits`]。
    #[serde(default)]
    pub plan: crate::plan::Plan,
    pub created_at: String,
    /// 匿名作品、或开发者设了 `--ttl` 的作品的到期时间。长期作品没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,

    // ---------------------------------------------------------------- 交付
    #[serde(default)]
    pub mode: DeliveryMode,
    /// 还没上传过版本时为 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<u32>,
    /// 最近一次提交版本的时间，RFC 3339。广场默认顺序的依据。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub isolated: bool,
    #[serde(default)]
    pub spa: bool,
    #[serde(default)]
    pub gate: GateMode,
    /// 隧道此刻连着（只有边缘知道，控制面从边缘的上报里得到）。
    #[serde(default)]
    pub tunnel_online: bool,
    /// 隧道上一次在线是什么时候，RFC 3339。离线页上那一行。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,

    // ---------------------------------------------------------------- 呈现
    pub title: String,
    #[serde(default)]
    pub kind: WorkKind,
    /// 一句话介绍，最多 [`crate::limits::MAX_SUMMARY_CHARS`] 字。跟着作品走，换版本不变。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// **这一版**的话（`--note`）：改了什么、想让人看什么。跟着版本走。
    ///
    /// 一个字段不是两个：对开发者「这版改了什么」和「想让人看什么」是同一句话，
    /// 对玩家在门禁页上、在通知正文里、在广场卡上看到的也是同一句话（REWRITE §9.6）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// 最新版本封面的内容哈希；没有封面就没有。
    ///
    /// 存哈希而不是一个 `bool`，是为了让封面地址带上 `?v=`：换了封面地址就变，
    /// 边缘那边才能放心让浏览器缓存一天。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_hash: Option<String>,
    /// 上传时认出来的引擎，小写标识符；认不出来就没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// 开发者的群（`--community`）。去哪是开发者的事，我们对去向不承诺。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub community_url: Option<String>,

    // ---------------------------------------------------------------- 访问
    #[serde(default)]
    pub access: Access,

    // ---------------------------------------------------------------- 上架
    /// 开发者勾了「放到广场上」。默认不公开。
    #[serde(default)]
    pub public: bool,
    /// 「正在找人测」。只在 `public` 时有意义。
    #[serde(default)]
    pub seeking: bool,
    /// 想找几位试玩者（`--seats`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats: Option<u32>,
    /// 已加入 = 点「开始」时留了名字的去重会话数（REWRITE §3.3）。
    #[serde(default)]
    pub joined: u32,
    /// 被举报到阈值、或运营者手工撤下了。`public` 仍是开发者的意愿，但广场上不出现——
    /// 控制台要如实告诉他这件事，不能让他以为自己在广场上。
    #[serde(default)]
    pub hidden: bool,
    /// [`crate::plaza::PLAYERS_WINDOW_DAYS`] 天内点了「开始」的去重人数。
    #[serde(default)]
    pub players: u32,
    /// 当前或排队中的推广（REWRITE §4.1）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boost: Option<Boost>,

    // ---------------------------------------------------------------- 社会
    /// 关注这个作品的人数。开发者只看到数字，看不到名单（REWRITE §3.3）。
    #[serde(default)]
    pub followers: u32,
    /// 「让玩家看到彼此的反馈」。
    #[serde(default)]
    pub feedback_public: bool,
    /// 公开的反馈里最近几条，新的在前。`feedback_public` 为假时为空。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_feedback: Vec<PublicNote>,
}

/// 作品的主人。匿名开发者也是一个 owner，只是没有名字和头像。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Owner {
    #[serde(default)]
    pub kind: OwnerKind,
    /// 门禁页与邀请卡上「某某 邀请你」的那个某某。匿名是「匿名开发者」。
    pub display_name: String,
    /// GitHub 用户名；匿名没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OwnerKind {
    #[default]
    Anon,
    Github,
}

impl OwnerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anon => "anon",
            Self::Github => "github",
        }
    }
}

impl std::str::FromStr for OwnerKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "anon" => Ok(Self::Anon),
            "github" => Ok(Self::Github),
            other => Err(format!("不认识的身份类型：{other}")),
        }
    }
}

/// 这个作品的字节从哪来（REWRITE §2.1「交付」）。切换对玩家透明，同一个 slug。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    /// 上传的静态目录，按清单分发。
    #[default]
    Upload,
    /// 开发者机器上跑着的进程，经隧道转发。
    Tunnel,
    /// 清单里有的从边缘给，清单外的走隧道到后端（`--backend`）。
    Hybrid,
}

impl DeliveryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Upload => "upload",
            Self::Tunnel => "tunnel",
            Self::Hybrid => "hybrid",
        }
    }

    /// 这条路上有没有「版本」这个概念。隧道只有在线 / 离线。
    pub fn has_versions(self) -> bool {
        matches!(self, Self::Upload | Self::Hybrid)
    }
}

impl std::str::FromStr for DeliveryMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "upload" => Ok(Self::Upload),
            "tunnel" => Ok(Self::Tunnel),
            "hybrid" => Ok(Self::Hybrid),
            other => Err(format!("不认识的交付方式：{other}")),
        }
    }
}

/// 谁能玩（REWRITE §3.3）。三档都在门禁页上完成，玩家不需要注册。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    /// 任何拿到链接的人。默认。
    #[default]
    Link,
    /// 要口令。
    Password,
    /// 只允许邀请名单（链接带一次性令牌，或玩家填邀请码）。
    Invite,
}

impl Access {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::Password => "password",
            Self::Invite => "invite",
        }
    }

    /// 门禁页上要不要多问一句。
    pub fn needs_challenge(self) -> bool {
        !matches!(self, Self::Link)
    }

    /// 带门禁的作品不上广场、卡上没有「分享」（REWRITE §3.3）：
    /// 私测的邀请不该被转发。
    pub fn can_be_listed(self) -> bool {
        matches!(self, Self::Link)
    }
}

impl std::str::FromStr for Access {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "link" => Ok(Self::Link),
            "password" => Ok(Self::Password),
            "invite" => Ok(Self::Invite),
            other => Err(format!("不认识的访问方式：{other}")),
        }
    }
}

/// 门禁页上显示的一条公开反馈。开发者看到的那份（`results::FeedbackItem`）字段多得多，
/// 带设备、浏览器、会话号——那些不该给玩家，所以这里是裁剪过的投影，不是同一个类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PublicNote {
    /// 留下的名字；没留就显示「一位试玩者」，由渲染方决定，这里是 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub text: String,
    pub version: u32,
    /// RFC 3339。
    pub at: String,
}

impl Project {
    /// 这份作品该不该用「试玩」这个词，而不是「体验」（REWRITE §3.3）。
    ///
    /// 判据是引擎名：用 AI 写小东西的人多数做的不是游戏，自绘 Canvas 的游戏也拿不到这个词——
    /// 猜错的代价是对着看板说「试玩」，所以宁可少说。
    pub fn is_game(&self) -> bool {
        self.engine
            .as_deref()
            .is_some_and(|e| GAME_ENGINES.contains(&e))
    }

    /// 名额到齐了没有。到齐之后不拦人，只如实说（REWRITE §3.3）。
    pub fn seats_full(&self) -> bool {
        matches!(self.seats, Some(n) if n > 0 && self.joined >= n)
    }

    /// 此刻在广场上：开发者公开了、没被撤下、而且没有门禁。
    pub fn listed(&self) -> bool {
        self.public && !self.hidden && self.access.can_be_listed()
    }

    /// 此刻占着推广位。周报那一项（[`crate::boost::BoostKind::Digest`]）不占位。
    pub fn boosted(&self) -> bool {
        self.boost
            .as_ref()
            .is_some_and(|b| b.status == crate::boost::BoostStatus::Live && b.kind.days().is_some())
    }

    /// 免费档与匿名链接带角标，Pro 可以去掉（REWRITE §4.1）。
    pub fn badge(&self) -> bool {
        !self.plan.can_remove_badge()
    }

    /// 广场卡与控制台作品墙上的那一张。
    pub fn card(&self) -> ProjectCard {
        ProjectCard {
            slug: self.slug.clone(),
            url: self.url.clone(),
            title: self.title.clone(),
            kind: self.kind,
            developer: self.owner.display_name.clone(),
            avatar_url: self.owner.avatar_url.clone(),
            summary: self.summary.clone(),
            note: self.note.clone(),
            engine: self.engine.clone(),
            is_game: self.is_game(),
            cover_hash: self.cover_hash.clone(),
            version: self.current_version.unwrap_or(0),
            updated_at: self.updated_at.clone().unwrap_or_default(),
            expires_at: self.expires_at.clone(),
            players: self.players,
            seeking: self.seeking,
            seats: self.seats,
            joined: self.joined,
            followers: self.followers,
            boosted: self.boosted(),
        }
    }

    /// `live.json`：这个作品会变、又不值得为它发一个版本的那几样。
    pub fn live(&self) -> ProjectLive {
        ProjectLive {
            schema: LIVE_SCHEMA,
            slug: self.slug.clone(),
            generated_at: String::new(),
            seats: self.seats,
            joined: self.joined,
            followers: self.followers,
            community_url: self.community_url.clone(),
            feedback_public: self.feedback_public,
            public_feedback: if self.feedback_public {
                self.public_feedback.clone()
            } else {
                Vec::new()
            },
            avatar_url: self.owner.avatar_url.clone(),
            listed: self.listed(),
            seeking: self.seeking && self.listed(),
        }
    }
}

/// 广场那面墙上、控制台作品墙上的一张卡（REWRITE §3.3）。
///
/// 两处用同一个类型不是省事：开发者从广场点进控制台，看到的应该是同一张卡、
/// 同一件事实。各写一份的话，同一个作品在两个地方会说不同的话。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectCard {
    pub slug: String,
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub kind: WorkKind,
    pub developer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// 这一版的话。正在找人测时卡上写它（「这次想测：…」），否则写 `summary`——
    /// 哪一句由 [`ProjectCard::blurb`] 一处决定。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// 说「试玩」还是「体验」。控制面算好，边缘不再判。
    pub is_game: bool,
    /// 封面的内容哈希；没有就排一张字卡。见 [`ProjectCard::cover_url`]。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_hash: Option<String>,
    pub version: u32,
    /// RFC 3339。
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub players: u32,
    #[serde(default)]
    pub seeking: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats: Option<u32>,
    #[serde(default)]
    pub joined: u32,
    #[serde(default)]
    pub followers: u32,
    /// 在推广位上。渲染时永远标「推广」。
    #[serde(default)]
    pub boosted: bool,
}

impl ProjectCard {
    pub fn has_cover(&self) -> bool {
        self.cover_hash.is_some()
    }

    /// 封面地址。作品自己的域下，走那个 slug 的每小时熔断。
    ///
    /// 地址上带哈希的前几位：换了封面地址就变，所以边缘可以放心让浏览器缓存一天；
    /// 直接打不带 `?v=` 的那个地址也只是最多旧一天。
    pub fn cover_url(&self) -> Option<String> {
        self.cover_hash.as_ref().map(|hash| {
            format!(
                "{}{}?v={}",
                self.url.trim_end_matches('/'),
                crate::COVER_PATH,
                &hash[..hash.len().min(8)]
            )
        })
    }

    /// 卡上作品名下面那一行字（REWRITE §3.3）。
    ///
    /// 正在找人测时写这一版的话，否则写一句话介绍。两个都没有就不写——
    /// 卡上不留一行「暂无简介」，那是模板在说话，不是作品在说话。
    pub fn blurb(&self) -> Option<Blurb<'_>> {
        if self.seeking {
            if let Some(note) = self.note.as_deref().filter(|n| !n.trim().is_empty()) {
                return Some(Blurb::Seeking(note));
            }
        }
        self.summary
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(Blurb::Summary)
    }

    /// 卡最底下那一行右边的**一件事实**（REWRITE §3.3）。
    ///
    /// 一张卡上只出现一个。优先级是「这个作品此刻最要紧的一句话」：
    /// 在找人且设了名额 → 还差几位；匿名快到期 → 还剩多久；有人玩过 → 几个人玩过；
    /// 其余 → 上一版什么时候发的。三个都没有就只剩时间，那也是一句真话。
    pub fn fact(&self) -> Fact {
        if self.seeking {
            if let Some(seats) = self.seats.filter(|n| *n > 0) {
                return Fact::Seats {
                    joined: self.joined,
                    seats,
                };
            }
        }
        if let Some(at) = &self.expires_at {
            return Fact::Expires { at: at.clone() };
        }
        if self.players > 0 {
            return Fact::Players {
                count: self.players,
            };
        }
        Fact::Updated {
            at: self.updated_at.clone(),
        }
    }
}

/// 卡上作品名下面那一行说的是哪一句。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blurb<'a> {
    /// 「这次想测：改了新手引导」
    Seeking(&'a str),
    /// 一句话介绍。
    Summary(&'a str),
}

/// 卡上那一件事实的四种形态。渲染成什么字由各自的文案表决定（`crate::i18n`），
/// 但**是哪一种**由 [`ProjectCard::fact`] 一处判定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fact {
    /// 「6 / 10 位」
    Seats { joined: u32, seats: u32 },
    /// 「还剩 5 小时」
    Expires { at: String },
    /// 「12 人玩过」
    Players { count: u32 },
    /// 「12 分钟前」
    Updated { at: String },
}

/// `live.json` 的格式版本号。不兼容的改动才加一。
pub const LIVE_SCHEMA: u32 = 1;

/// 对象存储里 `live.json` 的键。
///
/// 目录名仍是 `sites/`，和清单、当前指针放在一起——REWRITE §5.2 里写的是 `projects/`，
/// 但那要给已经在线的作品做一次目录迁移，而这个名字玩家和开发者都看不见。
/// 用户看得见的地方（API 路径、CLI 用词、文档）该叫作品就叫作品；盘上叫什么不值一次搬家。
pub fn live_key(slug: &str) -> String {
    format!("sites/{slug}/live.json")
}

/// 门禁页上最多显示几条公开反馈（REWRITE §3.3）。
pub const PUBLIC_NOTES_ON_GATE: usize = 3;

/// 一个作品**会变的那些**：控制面写、边缘只读、短缓存。
///
/// 清单不可变、每版一份；但门禁页上还有几样东西随时在变，又不值得为它们发一个版本。
/// 控制面挂了门禁页照常出，只是数字旧几分钟——所以没有这份文件的作品一律按
/// [`ProjectLive::default`] 解析：没名额、没人关注、没有群、反馈不公开，
/// 门禁页上对应的那几行不出现，不报错。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectLive {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub slug: String,
    /// RFC 3339，这份是什么时候整理的。
    #[serde(default)]
    pub generated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats: Option<u32>,
    #[serde(default)]
    pub joined: u32,
    #[serde(default)]
    pub followers: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub community_url: Option<String>,
    #[serde(default)]
    pub feedback_public: bool,
    /// 最多 [`PUBLIC_NOTES_ON_GATE`] 条，新的在前。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_feedback: Vec<PublicNote>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// 现在在广场上。门禁页的「分享」只给这样的作品。
    #[serde(default)]
    pub listed: bool,
    #[serde(default)]
    pub seeking: bool,
}

impl ProjectLive {
    pub fn empty(slug: &str) -> Self {
        Self {
            schema: LIVE_SCHEMA,
            slug: slug.to_string(),
            ..Self::default()
        }
    }

    pub fn seats_full(&self) -> bool {
        matches!(self.seats, Some(n) if n > 0 && self.joined >= n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Project {
        Project {
            slug: "brisk-otter-41".into(),
            url: "https://brisk-otter-41.playtest.run".into(),
            owner: Owner {
                kind: OwnerKind::Github,
                display_name: "小雨".into(),
                login: Some("xiaoyu".into()),
                avatar_url: Some("https://avatars.githubusercontent.com/u/1".into()),
            },
            plan: crate::plan::Plan::Free,
            created_at: "2026-09-08T00:00:00Z".into(),
            expires_at: None,
            mode: DeliveryMode::Upload,
            current_version: Some(7),
            updated_at: Some("2026-09-09T03:00:00Z".into()),
            isolated: false,
            spa: false,
            gate: GateMode::Once,
            tunnel_online: false,
            last_seen: None,
            title: "小球".into(),
            kind: WorkKind::Web,
            summary: Some("一个滚来滚去的小球".into()),
            note: Some("改了新手引导".into()),
            cover_hash: Some("abcdef0123456789".into()),
            engine: Some("godot".into()),
            community_url: None,
            access: Access::Link,
            public: true,
            seeking: true,
            seats: Some(10),
            joined: 6,
            hidden: false,
            players: 12,
            boost: None,
            followers: 3,
            feedback_public: false,
            public_feedback: vec![PublicNote {
                name: Some("阿松".into()),
                text: "不知道要按哪个键".into(),
                version: 7,
                at: "2026-09-09T04:00:00Z".into(),
            }],
        }
    }

    #[test]
    fn the_card_and_the_live_come_from_the_same_facts() {
        let p = sample();
        let card = p.card();
        let live = p.live();
        assert_eq!(card.slug, live.slug);
        assert_eq!(card.seats, live.seats);
        assert_eq!(card.joined, live.joined);
        assert_eq!(card.followers, live.followers);
        assert_eq!(card.avatar_url, live.avatar_url);
        assert_eq!(card.seeking, live.seeking);
    }

    #[test]
    fn a_closed_feedback_wall_keeps_the_notes_off_the_gate() {
        let mut p = sample();
        assert!(!p.feedback_public);
        assert!(p.live().public_feedback.is_empty(), "没开就一条都不给");
        p.feedback_public = true;
        assert_eq!(p.live().public_feedback.len(), 1);
    }

    #[test]
    fn a_gated_project_never_reaches_the_plaza() {
        let mut p = sample();
        assert!(p.listed());
        p.access = Access::Password;
        assert!(!p.listed(), "私测的邀请不该被转发");
        assert!(!p.live().listed);
        p.access = Access::Link;
        p.hidden = true;
        assert!(!p.listed(), "被撤下就不在墙上，但 public 仍是开发者的意愿");
        assert!(p.public);
    }

    #[test]
    fn the_verb_follows_the_engine_not_the_guess() {
        let mut p = sample();
        assert!(p.is_game(), "godot 是引擎");
        p.engine = Some("vite".into());
        assert!(!p.is_game(), "打包器不算");
        p.engine = None;
        assert!(!p.is_game(), "认不出来就说「体验」");
    }

    #[test]
    fn one_fact_per_card_in_priority_order() {
        let p = sample();
        let mut card = p.card();
        assert_eq!(
            card.fact(),
            Fact::Seats {
                joined: 6,
                seats: 10
            }
        );

        card.seeking = false;
        card.expires_at = Some("2026-09-10T20:59:00Z".into());
        assert_eq!(
            card.fact(),
            Fact::Expires {
                at: "2026-09-10T20:59:00Z".into()
            }
        );

        card.expires_at = None;
        assert_eq!(card.fact(), Fact::Players { count: 12 });

        card.players = 0;
        assert_eq!(
            card.fact(),
            Fact::Updated {
                at: "2026-09-09T03:00:00Z".into()
            }
        );
    }

    #[test]
    fn seeking_without_seats_falls_through_to_the_next_fact() {
        let mut p = sample();
        p.seats = None;
        let card = p.card();
        assert!(card.seeking);
        assert_eq!(
            card.fact(),
            Fact::Players { count: 12 },
            "「在找人」不是一件可数的事实"
        );
    }

    #[test]
    fn a_card_says_what_it_is_testing_only_while_it_is_seeking() {
        let p = sample();
        let mut card = p.card();
        assert_eq!(card.blurb(), Some(Blurb::Seeking("改了新手引导")));

        card.seeking = false;
        assert_eq!(card.blurb(), Some(Blurb::Summary("一个滚来滚去的小球")));

        // 在找人、但这一版什么都没写：退回一句话介绍，不硬挤一行空话。
        card.seeking = true;
        card.note = None;
        assert_eq!(card.blurb(), Some(Blurb::Summary("一个滚来滚去的小球")));

        card.summary = None;
        assert_eq!(card.blurb(), None, "两个都没有就一行都不写");
    }

    #[test]
    fn the_cover_lives_on_the_projects_own_host() {
        let p = sample();
        assert_eq!(
            p.card().cover_url().unwrap(),
            "https://brisk-otter-41.playtest.run/_playtest/cover?v=abcdef01",
            "地址带哈希前几位，换了封面地址就变"
        );
        let mut p2 = sample();
        p2.cover_hash = None;
        assert_eq!(p2.card().cover_url(), None);
    }

    #[test]
    fn a_boost_only_counts_while_it_is_live_and_takes_a_slot() {
        use crate::boost::{Boost, BoostKind, BoostStatus};
        let mut p = sample();
        assert!(!p.boosted());
        let mut b = Boost {
            id: 1,
            slug: p.slug.clone(),
            kind: BoostKind::Days3,
            status: BoostStatus::Pending,
            granted: true,
            starts_at: "2026-09-10T00:00:00Z".into(),
            ends_at: None,
            created_at: "2026-09-09T00:00:00Z".into(),
            order_id: None,
            reason: None,
        };
        p.boost = Some(b.clone());
        assert!(!p.boosted(), "排队中的还没上位");
        b.status = BoostStatus::Live;
        p.boost = Some(b.clone());
        assert!(p.boosted());
        b.kind = BoostKind::Digest;
        p.boost = Some(b);
        assert!(!p.boosted(), "周报那一项不占广场的位");
    }

    #[test]
    fn missing_live_file_reads_as_nothing_to_show() {
        let live: ProjectLive = serde_json::from_str("{}").unwrap();
        assert_eq!(live.seats, None);
        assert_eq!(live.joined, 0);
        assert!(!live.feedback_public);
        assert!(live.public_feedback.is_empty());
        assert!(!live.seats_full());
    }

    #[test]
    fn live_lives_next_to_the_manifests() {
        assert_eq!(live_key("brisk-otter-41"), "sites/brisk-otter-41/live.json");
    }

    #[test]
    fn access_decides_whether_a_challenge_stands_in_front() {
        assert!(!Access::Link.needs_challenge());
        assert!(Access::Password.needs_challenge());
        assert!(Access::Invite.needs_challenge());
    }

    #[test]
    fn delivery_mode_round_trips_through_its_db_string() {
        for mode in [
            DeliveryMode::Upload,
            DeliveryMode::Tunnel,
            DeliveryMode::Hybrid,
        ] {
            assert_eq!(mode.as_str().parse::<DeliveryMode>().unwrap(), mode);
        }
        assert!(DeliveryMode::Upload.has_versions());
        assert!(!DeliveryMode::Tunnel.has_versions(), "隧道只有在线 / 离线");
    }
}
