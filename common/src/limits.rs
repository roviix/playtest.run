//! 配额常量。CLI 在本地先按这些数字拦一道并把原因说清楚，api 再校验一次；两边用同一份，
//! 报错信息才对得上。数字对应 DESIGN §6 的免费档，匿名更紧。

pub const MIB: u64 = 1024 * 1024;
pub const GIB: u64 = 1024 * MIB;

/// 单个文件上限。Unity 未拆分的 `.data` 文件常见 30–80 MB，留够余量。
pub const MAX_FILE_BYTES: u64 = 200 * MIB;

/// 一个版本所有文件之和的上限（免费档）。
pub const MAX_VERSION_BYTES: u64 = 500 * MIB;

/// 匿名链接一个版本的上限。
pub const ANON_MAX_VERSION_BYTES: u64 = 200 * MIB;

/// 一个版本最多多少个文件。引擎导出物通常几十到几百个；上千个多半是把 `node_modules` 传上来了。
pub const MAX_FILES_PER_VERSION: usize = 5000;

/// 单条路径最长字节数。
pub const MAX_PATH_BYTES: usize = 1024;

/// 「这版改了什么」一句话的长度上限。
pub const MAX_NOTE_CHARS: usize = 280;

/// 作品名长度上限。
pub const MAX_TITLE_CHARS: usize = 80;

/// 一句话介绍的长度上限（DESIGN §3.8）。广场卡片上只放得下两行，140 个字已经是上限。
pub const MAX_SUMMARY_CHARS: usize = 140;

/// 「想让你看什么」的长度上限（DESIGN §3.8）。
pub const MAX_SEEK_NOTE_CHARS: usize = 140;

/// 封面图的字节上限（DESIGN §3.3）。2 MB 够一张 1200×630 的 PNG；封面会被贴在根域那一页上，
/// 一屏十几张，再大就是让每个翻广场的人替一张图付流量。
pub const MAX_COVER_BYTES: u64 = 2 * MIB;

/// 玩家在门禁页留的名字的长度上限（DESIGN §3.3 第 5 条）。点名册一行放得下的长度。
pub const MAX_PLAYER_NAME_CHARS: usize = 24;

/// `--seats` 的上限（DESIGN §3.3 第 4 条）。5–50 个具体的人是产品的起点，500 已经是「公开测试」而不是 playtest。
pub const MAX_SEATS: u32 = 500;

/// `--community` 链接的长度上限。
pub const MAX_COMMUNITY_URL_CHARS: usize = 300;

// ------------------------------------------------------------------ 带宽配额
//
// 两个时间尺度，缺一不可（DESIGN §4.8「额度是双重上限」）：月配额挡长期滥用，
// 每小时熔断挡分钟级的 DDoS。SIMMER.io 2025-04 是被分钟级账单打死的，
// 月配额在那个尺度上完全无效——它一个月才结算一次，账单已经产生了。

/// 登录账号每月出网硬上限（DESIGN §6）。到上限硬停、不产生账单。
pub const FREE_MONTHLY_BYTES: u64 = 10 * GIB;

/// 匿名链接 24 小时内的出网硬上限（DESIGN §6）。
/// 约等于一个 30 MB 的构建被完整打开 33 次——匿名链接本来就只活 24 小时。
pub const ANON_TOTAL_BYTES: u64 = GIB;

/// 每 slug 每小时的出网上限，边缘本地判定、不回源（DESIGN §4.8）。
///
/// DESIGN 没给数字，这里定 3 GiB，理由是两头夹出来的：往下，一个 30 MB 的构建
/// 一小时内能被完整打开约 100 次，一次 jam 或一轮 30 人的测试都够用
/// （DESIGN §8 的 T3 目标是每版打开人数中位数 ≥ 5，差两个数量级）；往上，
/// 按 §6 的 $0.1/GB，一个 slug 被刷满一小时的账单封顶在 $0.3 量级，
/// 一夜十小时也只有个位数美元。这是个保守值，私测里量到真实利用率再调。
pub const SLUG_HOURLY_BYTES: u64 = 3 * GIB;

/// 匿名链接的每小时上限更紧：它 24 小时总共才 [`ANON_TOTAL_BYTES`]，
/// 一小时就烧掉一半已经说明这不是「发给几个朋友」。
pub const ANON_SLUG_HOURLY_BYTES: u64 = ANON_TOTAL_BYTES / 2;
