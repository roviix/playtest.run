// 由 `cargo run -p playtest-common --bin playtest-contract` 生成，不要手改。
// 来源是 common/src/*.rs 上的类型与路由常量；那边改了这里跟着重新生成，
// `playtest-common` 的测试会在两边不一致时变红。
// 两个进程不会同时升级，所以带 `?` 的字段在旧控制面上可能不存在。

/** 收下了几条。被形状挡掉的不算在里面，但整批不会因为一条坏的就全退。 */
export type Accepted = {
  accepted: number;
};

/** 谁能玩（REWRITE §3.3）。三档都在门禁页上完成，玩家不需要注册。 */
export type Access = "link" | "password" | "invite";

export type AnonSessionResponse = {
  /** RFC 3339。到期后令牌与它创建的作品一起失效。 */
  expires_at: string;
  token: string;
};

export type Boost = {
  created_at: string;
  ends_at?: string;
  /** 运营者赠送的，不是买的。广场上的标签一样是「推广」，不区分。 */
  granted?: boolean;
  id: number;
  kind: BoostKind;
  /** 支付订单号；赠送的没有。 */
  order_id?: string;
  /** 人工没放行时给开发者看的理由；只在 `Rejected` 时有。 */
  reason?: string;
  slug: string;
  /** RFC 3339。`Pending` 时是排到的那一天；周报那一项是那一期发出的时间。 */
  starts_at: string;
  status: BoostStatus;
};

/** 两个 SKU 加一个附加项（DESIGN §6）。 */
export type BoostKind = "days3" | "days7" | "digest";

export type BoostList = Boost[];

export type BoostStatus = "pending" | "live" | "ended" | "rejected";

export type Capabilities = {
  /** 配了发信实现（`resend` / `smtp` / 本机的 `log`）。为假时门禁页与广场不出现邮箱输入框。 */
  email?: boolean;
  generated_at?: string;
  /** Web Push 的 VAPID 公钥（base64url）。没有就不出现「用浏览器通知」按钮。 */
  push_public_key?: string;
  schema?: number;
};

/** 连载小说的有序章节（DESIGN §3.17）。 */
export type ChapterEntry = {
  /** 章节生成的安全展示产物哈希。 */
  hash: string;
  /** 稳定章节标识（如 `c1`、`c2`），在作品生命周期内不可变。 */
  id: string;
  /** 原稿文件在 `files` 里的路径（如 `01-wake.md`）。 */
  path: string;
  size: number;
  /** 章节标题（如「第一章：沉睡的三百年」）。 */
  title: string;
};

export type Collection = {
  blocked_slugs?: string[];
  closes_at?: string;
  created_at: string;
  creator: string;
  entries?: CollectionEntry[];
  hidden: boolean;
  kind: CollectionKind;
  prompt: string;
  public: boolean;
  rules: string;
  slug: string;
  summary: string;
  title: string;
  updated_at: string;
};

export type CollectionDraft = {
  closes_at?: string;
  kind?: CollectionKind;
  prompt?: string;
  public?: boolean;
  rules?: string;
  slug?: string;
  summary?: string;
  title: string;
};

export type CollectionEntry = {
  note?: string;
  slug: string;
  submitted_at: string;
  submitted_version: number;
  title: string;
};

export type CollectionKind = "collection" | "challenge";

export type CollectionList = Collection[];

export type CommitUploadResponse = {
  expires_at?: string;
  slug: string;
  url: string;
  version: number;
};

export type ConfirmRequest = {
  token: string;
};

export type ConfirmResponse = {
  me: MeView;
  /** 种进 `pt_me` 的值。长期有效，撤销靠退订。 */
  me_token: string;
};

/**
 * 封面（DESIGN §3.3）。**不在 `files` 里**：它不是开发者目录的一部分，是清单单独的一条引用，
 * 边缘在 `/_playtest/cover` 提供。这样开发者目录的字节一个不多一个不少（§3.7 最后一条）。
 */
export type Cover = {
  hash: string;
  /** `image/png`、`image/jpeg` 或 `image/webp`，见 [`COVER_MIMES`]。 */
  mime: string;
  size: number;
};

export type CreateSiteRequest = {
  /** 想要的 slug；匿名用户忽略此项，随机分配。 */
  slug?: string;
  /** 作品名；没给就在上传时用目录名。 */
  title?: string;
};

/** 这个作品的字节从哪来（REWRITE §2.1「交付」）。切换对玩家透明，同一个 slug。 */
export type DeliveryMode = "upload" | "tunnel" | "hybrid";

export type DeviceLoginPoll = {
  device_code: string;
};

/** GitHub 设备码流程的第一步：CLI 把 `user_code` 和 `verification_uri` 打给人看，然后拿 `device_code` 轮询。 */
export type DeviceLoginStart = {
  device_code: string;
  /** 这个码还能用几秒。 */
  expires_in: number;
  /** 两次轮询之间至少隔几秒；GitHub 说 `slow_down` 时控制面会把它加大。 */
  interval: number;
  /** 给人在浏览器里输入的那串，形如 `WDJB-MJHT`。 */
  user_code: string;
  /** 一般是 `https://github.com/login/device`。 */
  verification_uri: string;
};

export type EntryDraft = {
  note?: string;
  slug: string;
};

/** 所有非 2xx 响应的体。`code` 给程序判断，`message` 给人看（中文，第一次用的人看得懂）。 */
export type ErrorBody = {
  code: ErrorCode;
  message: string;
};

export type ErrorCode = "unauthorized" | "token_expired" | "not_found" | "internal" | "invalid" | "quota_exceeded" | "hash_mismatch" | "blobs_missing" | "slug_unavailable" | "tunnel_replaced" | "backend_offline" | "login_unavailable" | "login_failed";

/** 错误按 fingerprint 归堆之后的样子。 */
export type ErrorSummary = {
  /** 去重之后有几种。 */
  distinct: number;
  /** 撞得最多的几条，最多 [`TOP_ERRORS`] 条。 */
  top?: ErrorTally[];
  /** 一共撞了几次。 */
  total: number;
};

export type ErrorTally = {
  count: number;
  /** SDK 报上来的 `name`，通常是「错误类型 + 出错的那一行」。 */
  fingerprint: string;
};

export type Event = {
  data?: unknown;
  /** [`kind`] 里的一个。 */
  kind: string;
  name?: string;
  /** 客户端的 RFC 3339 时间。客户端的钟不可信，服务端只在它落在合理窗口里时采信。 */
  ts: string;
};

export type EventBatch = {
  events?: Event[];
  /** 会话 id，来自 [`Me::sid`]；SDK 拿不到时是它自己在 localStorage 里生成的匿名 id。 */
  session: string;
  slug: string;
};

export type FeedbackAccepted = {
  /** 这个会话还能再提几条。 */
  remaining: number;
};

/** 一条反馈：一句话 + 让这句话能被定位的上下文（DESIGN §3.4）。 */
export type FeedbackItem = {
  browser?: string;
  device?: string;
  id: number;
  /** 说这句话的人在门禁页留的名字（DESIGN §3.5）。 */
  name?: string;
  /** 这一条在门禁页上公开着（作品开了公开反馈、且开发者没把它单独藏起来）。 */
  public?: boolean;
  /** 截图 v0.2 才做，现在永远是 `None`（DESIGN §3.5 说的「附截图」还没实现）。 */
  screenshot_hash?: string;
  /** 进入多久说的这句话。 */
  seconds_in?: number;
  session_id: string;
  status: FeedbackStatus;
  text: string;
  ts: string;
  version: number;
};

export type FeedbackList = {
  /** 时间倒序，最新的在最前面。 */
  items: FeedbackItem[];
  slug: string;
};

export type FeedbackRequest = {
  /** 玩家进来多少秒了。SDK 自己算，用来在点名册里显示「进入 47 秒」。 */
  seconds_in?: number;
  session: string;
  slug: string;
  text: string;
};

export type FeedbackStatus = "new" | "seen" | "done";

/** 清单里的一个文件。 */
export type FileEntry = {
  /** 内容哈希，见 [`crate::hash`]。 */
  hash: string;
  /**
   * 相对于上传目录的路径。正斜杠分隔，不以斜杠开头，没有 `.` / `..` / 空段。
   * 例：`index.html`、`Build/game.wasm.br`。
   */
  path: string;
  size: number;
};

export type FollowChannel = {
  email: string;
  kind: "email";
} | {
  kind: "push";
  subscription: PushSubscription;
} | {
  kind: "me";
  me_token: string;
};

export type FollowRequest = {
  channel: FollowChannel;
  /** 见 [`form::FROM`]。 */
  from?: string;
  target: FollowTarget;
};

export type FollowResponse = {
  status: "confirm_sent";
} | {
  status: "subscribed";
} | {
  status: "already_following";
};

/** 一个作品，或广场本身。 */
export type FollowTarget = {
  kind: "site";
  slug: string;
} | {
  kind: "collection";
  slug: string;
} | {
  kind: "plaza";
};

export type FollowView = {
  /** RFC 3339。 */
  since: string;
  target: FollowTarget;
  /** 作品名；广场那一项没有。 */
  title?: string;
  /** 作品链接；广场那一项没有。 */
  url?: string;
};

/** 门禁页出现的策略（DESIGN §3.3）。 */
export type GateMode = "once" | "always" | "never";

export type GrantBoostRequest = {
  kind: BoostKind;
  /** 赠送的默认直接 `Live`（运营者自己就是审核的人）；传 `true` 让它走 `Pending`。 */
  review?: boolean;
  slug: string;
  /** 不给就是「最早能排上的那一天」。RFC 3339。 */
  starts_at?: string;
};

export type JobList = JobStatus[];

/** 控制面一件后台活的现状（清过期作品、回收 blob、周报、发通知、重写广场与 `live.json`）。 */
export type JobStatus = {
  every_secs: number;
  failures: number;
  last_count?: number;
  last_error?: string;
  last_ok?: boolean;
  /** RFC 3339；还没跑过是 `None`。 */
  last_run_at?: string;
  name: string;
  runs: number;
};

/** 一个档位的全部上限。边缘拿到的是这一份（跟着 `current.json` 下来），不认识档位这个词。 */
export type Limits = {
  active_projects: number;
  /** 同一个作品同时在玩的人数上限。 */
  concurrent_players: number;
  /**
   * 每作品每小时出网上限，边缘本地判定、不回源。
   * 
   * 月配额挡长期滥用，每小时熔断挡分钟级的 DDoS——SIMMER.io 2025-04 是被分钟级账单
   * 打死的，月配额在那个尺度上完全无效，它一个月才结算一次。两个尺度缺一不可。
   */
  hourly_bytes: number;
  kept_versions: number;
  /** 一个版本所有文件之和的上限。 */
  max_version_bytes: number;
  /** 会话、事件、反馈保留多少天。 */
  results_days: number;
  /** 窗口内出网字节上限。到顶硬停，不产生账单。 */
  traffic_bytes: number;
  traffic_window: TrafficWindow;
};

/** 一个作品在广场（DESIGN §3.8）上的状态。 */
export type Listing = {
  /** 推广状态（DESIGN §3.11）：`None` 没有；否则是当前或排队中的那一段。 */
  boost?: Boost;
  /** 开发者的群（`--community`）。 */
  community_url?: string;
  /** 「让玩家看到彼此的反馈」（DESIGN §3.5）。 */
  feedback_public?: boolean;
  /** 关注这个作品的人数（DESIGN §3.6）。开发者只看到数字。 */
  followers?: number;
  /** 最新版本有没有封面。 */
  has_cover?: boolean;
  /**
   * 被举报到阈值、或我们手工撤下了：`public` 仍是开发者的意愿，但广场上不出现。
   * 控制台要把这件事告诉开发者，不能让他以为自己在广场上。
   */
  hidden?: boolean;
  joined?: number;
  /** 开发者勾了「放到广场上」。默认不公开。 */
  public?: boolean;
  /** 想找几位试玩者（`--seats`，DESIGN §3.3）；`joined` 是留了名字的人数。 */
  seats?: number;
  /** 想让来的人重点看什么，最多 [`crate::limits::MAX_SEEK_NOTE_CHARS`] 字。 */
  seek_note?: string;
  /** 「正在找人测」。只在 `public` 时有意义。 */
  seeking?: boolean;
  /** 一句话介绍，最多 [`crate::limits::MAX_SUMMARY_CHARS`] 字。随最新版本的清单走。 */
  summary?: string;
};

/** 轮询的结果。`pending` 继续等；`ok` 里就是登录结果。 */
export type LoginPollResponse = {
  interval: number;
  status: "pending";
} | LoginResponse & {
  status: "ok";
};

/** 登录成功。`token` 长期有效，之后所有请求都带它。 */
export type LoginResponse = {
  /** 玩家在门禁页和广场上看到的名字：GitHub 上的显示名，没有就是用户名。 */
  display_name: string;
  /** GitHub 用户名（不带 @）。 */
  login: string;
  /** 这次顺带归入账号的匿名作品数（请求带了匿名令牌才会大于 0）。 */
  migrated_sites: number;
  token: string;
};

/** `GET /v1/me`。 */
export type Me = {
  /** GitHub 头像；匿名没有。广场卡片与门禁页上显示（DESIGN §3.9）。 */
  avatar_url?: string;
  display_name: string;
  /** 匿名身份的到期时间；登录用户没有。 */
  expires_at?: string;
  /** `anon` 或 `github`。 */
  kind: string;
  /** GitHub 用户名；匿名没有。 */
  login?: string;
};

export type MeRequest = {
  me_token: string;
};

/** 关注页（DESIGN §3.10）：一个抽屉，不是一个 profile。 */
export type MeView = {
  /** 打码显示的邮箱，如 `z***@example.com`；只用浏览器通知、没留邮箱的人没有。 */
  email_masked?: string;
  follows?: FollowView[];
  /** 这个人开了浏览器通知。 */
  push?: boolean;
};

export type ModerateCollectionRequest = {
  hidden: boolean;
};

export type NotificationQueue = {
  /** 进了死信的（DESIGN §4.10：失败三次）。 */
  dead: number;
  failed_24h: number;
  /** 下一期周报什么时候发，RFC 3339。 */
  next_digest_at?: string;
  pending: number;
  sent_24h: number;
};

/** 作品的主人。匿名开发者也是一个 owner，只是没有名字和头像。 */
export type Owner = {
  avatar_url?: string;
  /** 门禁页与邀请卡上「某某 邀请你」的那个某某。匿名是「匿名开发者」。 */
  display_name: string;
  kind?: OwnerKind;
  /** GitHub 用户名；匿名没有。 */
  login?: string;
};

export type OwnerKind = "anon" | "github";

/** 三档。匿名也是一档——它有自己的上限，不是「Free 的特例」。 */
export type Plan = "anon" | "free" | "pro";

export type Plaza = {
  /** 关注广场本身的人数（DESIGN §3.6 的周报收件人）。页面上不写，给周报用。 */
  club_followers?: number;
  collections?: Collection[];
  /** RFC 3339，这份是什么时候整理的。排查用，页面上不显示。 */
  generated_at: string;
  /**
   * 已经按广场的默认顺序排好：推广中的在最前（最多 [`crate::boost::MAX_SLOTS`] 个），
   * 然后正在找人测的，其余按最近更新。边缘按这个顺序铺一面网格，不再分段。
   */
  items: ProjectCard[];
  schema: number;
};

export type PrepareUploadRequest = {
  /** 连载小说的章节列表（DESIGN §3.17）。 */
  chapters?: ChapterEntry[];
  /**
   * 封面。CLI 先把它当普通 blob 传上来（`PUT /v1/blobs/{hash}`），提交时服务端检查它在不在。
   * 没给就沿用上一版的封面。
   */
  cover?: Cover;
  /** CLI 上传时认出来的引擎，见 [`crate::manifest::Manifest::engine`]。认不出来就没有。 */
  engine?: string;
  /** 文章原稿或视频文件在 `files` 里的路径；网页没有固定入口。 */
  entry?: string;
  files: FileEntry[];
  gate?: GateMode;
  isolated?: boolean;
  /** 单文件发布明确告诉控制面怎样验证和展示；旧 CLI 没有此字段，按网页处理。 */
  kind?: WorkKind;
  note?: string;
  spa?: boolean;
  /** 一句话介绍（DESIGN §3.8）。没给就沿用这个作品上一版的。 */
  summary?: string;
  title?: string;
};

export type PrepareUploadResponse = {
  /** 服务端没有的哈希，CLI 只传这些。去重后、按清单顺序。 */
  missing: string[];
  /** 这次要传的字节数之和，给进度条用。 */
  missing_bytes: number;
  upload_id: string;
};

/**
 * 一个作品的全部事实。控制面从库里读一次、拼一次，其余都是投影。
 * 
 * 六组属性的分法见 REWRITE §2.1：身份、交付、呈现、访问、上架、社会。
 */
export type Project = {
  access?: Access;
  /** 当前或排队中的推广（REWRITE §4.1）。 */
  boost?: Boost;
  /** 开发者的群（`--community`）。去哪是开发者的事，我们对去向不承诺。 */
  community_url?: string;
  /**
   * 最新版本封面的内容哈希；没有封面就没有。
   * 
   * 存哈希而不是一个 `bool`，是为了让封面地址带上 `?v=`：换了封面地址就变，
   * 边缘那边才能放心让浏览器缓存一天。
   */
  cover_hash?: string;
  created_at: string;
  /** 还没上传过版本时为 `None`。 */
  current_version?: number;
  /** 上传时认出来的引擎，小写标识符；认不出来就没有。 */
  engine?: string;
  /** 匿名作品、或开发者设了 `--ttl` 的作品的到期时间。长期作品没有。 */
  expires_at?: string;
  /** 「让玩家看到彼此的反馈」。 */
  feedback_public?: boolean;
  /** 关注这个作品的人数。开发者只看到数字，看不到名单（REWRITE §3.3）。 */
  followers?: number;
  gate?: GateMode;
  /**
   * 被举报到阈值、或运营者手工撤下了。`public` 仍是开发者的意愿，但广场上不出现——
   * 控制台要如实告诉他这件事，不能让他以为自己在广场上。
   */
  hidden?: boolean;
  isolated?: boolean;
  /** 已加入 = 点「开始」时留了名字的去重会话数（REWRITE §3.3）。 */
  joined?: number;
  kind?: WorkKind;
  /** 隧道上一次在线是什么时候，RFC 3339。离线页上那一行。 */
  last_seen?: string;
  mode?: DeliveryMode;
  /**
   * **这一版**的话（`--note`）：改了什么、想让人看什么。跟着版本走。
   * 
   * 一个字段不是两个：对开发者「这版改了什么」和「想让人看什么」是同一句话，
   * 对玩家在门禁页上、在通知正文里、在广场卡上看到的也是同一句话（REWRITE §9.6）。
   */
  note?: string;
  owner: Owner;
  /** 跟着 owner 走，决定 [`crate::plan::Limits`]。 */
  plan?: Plan;
  /** [`crate::plaza::PLAYERS_WINDOW_DAYS`] 天内点了「开始」的去重人数。 */
  players?: number;
  /** 开发者勾了「放到广场上」。默认不公开。 */
  public?: boolean;
  /** 公开的反馈里最近几条，新的在前。`feedback_public` 为假时为空。 */
  public_feedback?: PublicNote[];
  /** 想找几位试玩者（`--seats`）。 */
  seats?: number;
  /** 「正在找人测」。只在 `public` 时有意义。 */
  seeking?: boolean;
  slug: string;
  spa?: boolean;
  /** 一句话介绍，最多 [`crate::limits::MAX_SUMMARY_CHARS`] 字。跟着作品走，换版本不变。 */
  summary?: string;
  title: string;
  /** 隧道此刻连着（只有边缘知道，控制面从边缘的上报里得到）。 */
  tunnel_online?: boolean;
  /** 最近一次提交版本的时间，RFC 3339。广场默认顺序的依据。 */
  updated_at?: string;
  /** 玩家点开的完整链接，例如 `https://brisk-otter-41.playtest.run`。 */
  url: string;
};

/**
 * 广场那面墙上、控制台作品墙上的一张卡（REWRITE §3.3）。
 * 
 * 两处用同一个类型不是省事：开发者从广场点进控制台，看到的应该是同一张卡、
 * 同一件事实。各写一份的话，同一个作品在两个地方会说不同的话。
 */
export type ProjectCard = {
  avatar_url?: string;
  /** 在推广位上。渲染时永远标「推广」。 */
  boosted?: boolean;
  /** 封面的内容哈希；没有就排一张字卡。见 [`ProjectCard::cover_url`]。 */
  cover_hash?: string;
  developer: string;
  engine?: string;
  expires_at?: string;
  followers?: number;
  /** 说「试玩」还是「体验」。控制面算好，边缘不再判。 */
  is_game: boolean;
  joined?: number;
  kind?: WorkKind;
  /**
   * 这一版的话。正在找人测时卡上写它（「这次想测：…」），否则写 `summary`——
   * 哪一句由 [`ProjectCard::blurb`] 一处决定。
   */
  note?: string;
  players: number;
  seats?: number;
  seeking?: boolean;
  slug: string;
  summary?: string;
  title: string;
  /** RFC 3339。 */
  updated_at: string;
  url: string;
  version: number;
};

/**
 * 一个作品**会变的那些**：控制面写、边缘只读、短缓存。
 * 
 * 清单不可变、每版一份；但门禁页上还有几样东西随时在变，又不值得为它们发一个版本。
 * 控制面挂了门禁页照常出，只是数字旧几分钟——所以没有这份文件的作品一律按
 * [`ProjectLive::default`] 解析：没名额、没人关注、没有群、反馈不公开，
 * 门禁页上对应的那几行不出现，不报错。
 */
export type ProjectLive = {
  avatar_url?: string;
  community_url?: string;
  feedback_public?: boolean;
  followers?: number;
  /** RFC 3339，这份是什么时候整理的。 */
  generated_at?: string;
  joined?: number;
  /** 现在在广场上。门禁页的「分享」只给这样的作品。 */
  listed?: boolean;
  /** 最多 [`PUBLIC_NOTES_ON_GATE`] 条，新的在前。 */
  public_feedback?: PublicNote[];
  schema?: number;
  seats?: number;
  seeking?: boolean;
  slug?: string;
};

/**
 * 门禁页上显示的一条公开反馈。开发者看到的那份（`results::FeedbackItem`）字段多得多，
 * 带设备、浏览器、会话号——那些不该给玩家，所以这里是裁剪过的投影，不是同一个类型。
 */
export type PublicNote = {
  /** RFC 3339。 */
  at: string;
  /** 留下的名字；没留就显示「一位试玩者」，由渲染方决定，这里是 `None`。 */
  name?: string;
  text: string;
  version: number;
};

export type PushKeys = {
  auth: string;
  p256dh: string;
};

/** 浏览器 `PushSubscription.toJSON()` 的形状。原样存、原样用，我们不解读里面的字节。 */
export type PushSubscription = {
  endpoint: string;
  keys: PushKeys;
};

export type ReviewBoostRequest = {
  approve: boolean;
  reason?: string;
};

/** 点名册的排法。 */
export type RosterSort = "dwell" | "time";

export type SendLinkRequest = {
  email: string;
};

export type SessionEvent = {
  data?: unknown;
  /** [`crate::ingest::kind`] 或 [`crate::ingest::edge_kind`] 里的一个。 */
  kind: string;
  name?: string;
  /** edge / sdk。 */
  source: string;
  ts: string;
};

/** 点名册里的一个人。每一列都要能回答「我下一步该看谁」，回答不了的列不加。 */
export type SessionRow = {
  /** 打开的时间。 */
  at: string;
  browser?: string;
  device?: string;
  /** 停留秒数：最后一次看见减第一次看见。 */
  dwell_s: number;
  /** 进到游戏了没。 */
  entered: boolean;
  errors: number;
  /** 展开这一行看到的：这个会话的事件与错误，时间正序，最多 [`MAX_EVENTS_PER_SESSION`] 条。 */
  events?: SessionEvent[];
  /** 留了几句话。 */
  feedback: number;
  /** 报过首帧。要 SDK，没接的作品这一列永远是 false。 */
  first_frame: boolean;
  id: string;
  is_return: boolean;
  /** 最后一次输入距「进入」多久。没接 SDK、或者一次都没动过是 `None`。 */
  last_input_after_s: number;
  /** 事件多到被截断了。 */
  more_events?: boolean;
  /** 点「开始」时留的名字（DESIGN §3.3 第 5 条）。没留就没有，点名册显示会话 id 的头几位。 */
  name?: string;
  os?: string;
  /** 玩到哪：最后一个自定义事件的名字。 */
  reached?: string;
  /** [`crate::ingest::source`] 里的一个：card / notice / plaza / wechat / discord / direct / other，尽力而为。 */
  referrer_kind?: string;
  /** 点过门禁页的「开始」。 */
  started: boolean;
  wechat: boolean;
};

export type Site = {
  created_at: string;
  /** 还没上传过版本时为 `None`。 */
  current_version?: number;
  /** 匿名作品的到期时间。 */
  expires_at?: string;
  /** 网页 / 文章 / 视频。还没有版本以及旧控制面返回的作品按网页显示。 */
  kind?: WorkKind;
  /** 广场上的状态（DESIGN §3.8）。旧控制面不返回这一段，按「不公开」解析。 */
  listing?: Listing;
  slug: string;
  title: string;
  /** 玩家点开的完整链接，例如 `https://brisk-otter-41.playtest.run`。 */
  url: string;
};

export type SiteList = Site[];

/** 作品时间线：一屏能看完的全部（DESIGN §3.4「点开是这一版的会话列表」的上一层）。 */
export type SiteResults = {
  current_version?: number;
  /** 关注这个作品的人数（DESIGN §3.6）——「下一版发出去他们会收到通知」那一句的数字。 */
  followers?: number;
  slug: string;
  title: string;
  /** 版本倒序，最新的在最前面。 */
  versions: VersionResults[];
};

export type SourceTally = {
  count: number;
  kind: string;
};

/** 流量额度算在哪个窗口上。 */
export type TrafficWindow = "link_lifetime" | "monthly";

/** `POST /v1/projects/{slug}/tunnel` 的响应：CLI 拿它去连边缘。 */
export type TunnelGrant = {
  /** 握手地址，例如 `wss://brisk-otter-41.playtest.run/_playtest/tunnel`（本机开发是 `ws://…localhost:8443/…`）。 */
  connect_url: string;
  /** RFC 3339，CLI 在此之前换新。 */
  expires_at: string;
  site_expires_at?: string;
  slug: string;
  token: string;
  /** 玩家链接，例如 `https://brisk-otter-41.playtest.run`。 */
  url: string;
};

/** `POST /v1/projects/{slug}/tunnel` 的请求体。 */
export type TunnelRequest = {
  gate?: GateMode;
  /** 见 [`Claims::hybrid`]。控制面只在作品已有上传过的版本时才签它。 */
  hybrid?: boolean;
  isolated?: boolean;
  title?: string;
};

export type UnfollowRequest = {
  me_token: string;
  target: FollowTarget;
};

export type UnsubscribeRequest = {
  token: string;
};

/**
 * `PATCH /v1/projects/{slug}/feedback/{id}`：只改带了的字段。
 * `public: Some(false)` 是「把这一条藏起来」——作品级的开关在 [`crate::api::UpdateSiteRequest::feedback_public`]。
 */
export type UpdateFeedbackRequest = {
  public?: boolean;
  status?: FeedbackStatus;
};

/**
 * `PATCH /v1/projects/{slug}`：只改带了的字段。
 * `seek_note` / `community_url` 传空字符串表示清掉；`seats` 传 0 表示清掉。
 */
export type UpdateSiteRequest = {
  community_url?: string;
  feedback_public?: boolean;
  public?: boolean;
  seats?: number;
  seek_note?: string;
  seeking?: boolean;
};

export type VersionFile = {
  /** 内容哈希（SHA-256 小写十六进制）。同样的哈希就是同样的字节，不用再传一遍。 */
  hash: string;
  path: string;
  size: number;
  /** 直接能打开的地址。 */
  url: string;
};

/**
 * 一个版本里的文件清单（`GET /v1/projects/{slug}/versions/{version}/files`）。
 * 
 * 只有清单和哈希，没有字节：文件本身在作品自己的域上按路径取就行。
 */
export type VersionFiles = {
  /** 玩家现在看到的就是这一版。 */
  current: boolean;
  /** 按路径排序，和清单里一致。 */
  files: VersionFile[];
  slug: string;
  total_bytes: number;
  version: number;
};

/** 一个已发布的版本。 */
export type VersionInfo = {
  created_at: string;
  /** 玩家现在看到的就是这一版。 */
  current: boolean;
  file_count: number;
  note?: string;
  total_bytes: number;
  version: number;
};

export type VersionList = {
  current_version?: number;
  slug: string;
  /** 新的在前。 */
  versions: VersionInfo[];
};

/** 一个版本的全部数字。控制台按这些拼那段话，数为 0 的句子不说。 */
export type VersionResults = {
  /** 这一版上传的时间。只在会话里见过、版本表里没有的版本是 `None`。 */
  created_at?: string;
  /**
   * L7：点了「开始」却没等到首帧的人数——门禁到首帧之间掉的那几个（DESIGN §3.4）。
   * 
   * `None` 表示**这一版我们不知道**：没有任何会话报过首帧，多半是没接 SDK。
   * 不知道就说不知道，不拿 0 冒充「一个都没掉」（AGENTS 第 4 条）。
   */
  dropped_before_first_frame: number | null;
  /** 停留秒数的中位数，见 [`median_seconds`]。没有会话时是 `None`。 */
  dwell_median_s: number | null;
  /** 多少人真的进到游戏：点过「开始」，或者 SDK 报过 `load`。 */
  entered: number;
  errors: ErrorSummary;
  feedback_count: number;
  /** 这一版第一次和最后一次被打开的时间。 */
  first_at?: string;
  last_at?: string;
  /** 边缘报的 `resource_fail` 条数：404、下到一半断了、MIME 不对。 */
  load_failures: number;
  /** 留了名字的人数。 */
  named?: number;
  /** 上传时附的那句「这版改了什么」（DESIGN §3.5）。 */
  note?: string;
  /** 多少人打开。一次打开就是一个会话。 */
  opened: number;
  /** 停留超过 [`LONG_PLAY_SECONDS`] 的人数。 */
  played_5min_plus: number;
  /** 回头再来一次的人数。 */
  returned: number;
  /**
   * 来自哪里，按人数降序（DESIGN §3.5「来自：邀请卡 4 · 广场 2 · 微信 2」）。
   * 键是 [`crate::ingest::source`] 里的值。0 的不列。
   */
  sources?: SourceTally[];
  version: number;
};

export type VersionSessions = {
  sessions: SessionRow[];
  slug: string;
  sort: RosterSort;
  version: number;
};

/** 网页授权码流程的最后一步：控制台把 GitHub 回传的 `code` 与 `state` 交给控制面换令牌。 */
export type WebLoginExchange = {
  code: string;
  state: string;
};

/** 作品怎样被体验。旧清单没有这一项，必须继续按网页读取。 */
export type WorkKind = "web" | "article" | "video";

/** 控制面的路径。有参数的是函数，参数会做 URL 编码。 */
export const paths = {
  health: "/healthz",
  anonSessions: "/v1/anon/sessions",
  loginDeviceStart: "/v1/login/github/device",
  loginDevicePoll: "/v1/login/github/device/poll",
  loginWebStart: "/v1/login/github/start",
  loginWebExchange: "/v1/login/github/exchange",
  me: "/v1/me",
  meToken: "/v1/me/token",
  collections: "/v1/collections",
  openCollections: "/v1/collections/open",
  collection: (slug: string | number) => `/v1/collections/${encodeURIComponent(String(slug))}`,
  collectionEntries: (slug: string | number) => `/v1/collections/${encodeURIComponent(String(slug))}/entries`,
  collectionEntry: (slug: string | number, site: string | number) => `/v1/collections/${encodeURIComponent(String(slug))}/entries/${encodeURIComponent(String(site))}`,
  collectionBlock: (slug: string | number, site: string | number) => `/v1/collections/${encodeURIComponent(String(slug))}/blocks/${encodeURIComponent(String(site))}`,
  projects: "/v1/projects",
  project: (slug: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}`,
  projectUploads: (slug: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}/uploads`,
  projectUploadCommit: (slug: string | number, upload_id: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}/uploads/${encodeURIComponent(String(upload_id))}/commit`,
  blob: (hash: string | number) => `/v1/blobs/${encodeURIComponent(String(hash))}`,
  projectTunnel: (slug: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}/tunnel`,
  projectVersions: (slug: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}/versions`,
  projectVersionActivate: (slug: string | number, version: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}/versions/${encodeURIComponent(String(version))}/activate`,
  projectVersionFiles: (slug: string | number, version: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}/versions/${encodeURIComponent(String(version))}/files`,
  projectResults: (slug: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}/results`,
  projectVersionSessions: (slug: string | number, version: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}/versions/${encodeURIComponent(String(version))}/sessions`,
  projectFeedback: (slug: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}/feedback`,
  projectFeedbackItem: (slug: string | number, id: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}/feedback/${encodeURIComponent(String(id))}`,
  adminBoosts: "/admin/boosts",
  adminBoost: (id: string | number) => `/admin/boosts/${encodeURIComponent(String(id))}`,
  adminBoostReview: (id: string | number) => `/admin/boosts/${encodeURIComponent(String(id))}/review`,
  adminPlazaHide: (slug: string | number) => `/admin/plaza/${encodeURIComponent(String(slug))}/hide`,
  adminNotifications: "/admin/notifications",
  adminJobs: "/admin/jobs",
} as const;
