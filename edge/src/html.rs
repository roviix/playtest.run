//! 玩家看到的每一页都在这里拼出来。
//!
//! 不引模板引擎：这几页加起来不到两百行，模板引擎换来的是一层编译期魔法和一个新依赖。
//! 代价是**每一处插值都必须自己调 [`esc`]**——清单里的作品名和开发者名是用户输入，
//! 这里是玩家域，漏一个就是 XSS。
//!
//! 长相（DESIGN §3.3、§3.9）：暖炭底、象牙字、唯一一支琥珀只给动作与焦点。封面优先。
//! 门禁页、分享页、「我的」和错误页共用下面这一份记号，广场在它之上再加自己的一段（`plaza.rs`）。
//! 玩家域只有这一套颜色：群里那张邀请卡、点开的门禁页、回到的广场，不换色。门禁页和邀请卡
//! （`card.rs`）是**同一个物件的两种媒介**：同一张封面、同一句话、同一个版本号，
//! 所以卡片版式（封面出血、真人名字与作品名最大、版本日期用等宽小字、一个主按钮）
//! 在这里定，两边照着做。

/// HTML 文本与属性值通用的转义。单引号也转，属性用单引号包时才安全。
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// 字卡的色相：同一个作品每次都是同一种颜色，不同作品大概率不同。
/// 门禁页、广场卡片、邀请卡三处共用它，一个作品在三个地方是同一种蓝或同一种绿。
pub fn hue(slug: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in slug.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h % 360
}

/// 玩家页面只有这一套设计系统，分三层写，每一页只带自己用得上的两层：
///
/// - [`BASE`]：颜色记号、字体、焦点环、输入框、几个两边都用的类（`.lead` `.summary` `.row` `.more`）。
/// - [`CARD`]：一张居中的卡（门禁、分享、离线、到期、举报、确认、退订）。
/// - [`PAGE`]：整页版式（广场、「我的」）：左边一条栏、右边一面墙。
///
/// 卡页发 `BASE + CARD`，整页发 `BASE + PAGE`。两层**互不覆盖**——原来广场那一段是叠在
/// 卡的样式之上再一条条撤销（`body.page button{...}` 之类 88 行），撤不干净的就漏成怪版式
/// （广场卡右下那件事实曾经带着卡页 `.seats` 的 14px 上边距）。现在没有可撤销的东西。
///
/// 不引外部脚本与字体：玩家可能在微信里、在很慢的网上，多一个请求就多一次白屏。
/// 里面**不写 CSS 注释**：这段字节随每一页发给玩家，注释既是白花的流量，也会把内部说法
/// 漏到页面源码里（有测试盯着页面上不该出现的词）。要解释的都写在这里：
/// - `main.card` 用 auto 外边距居中，比 `align-items:center` 稳：内容比屏幕高时不会被截掉。
/// - `.hero.word` / `.cover.word` 是没有封面时的字卡，色相来自 [`hue`]，
///   门禁页、广场、邀请卡三处同一个作品是同一种颜色。
/// - `.by` 是「谁邀请你」那一行：一张脸加一个真人名字（DESIGN §3.9）。
/// - `.more` 里那几行是弱化的：一页只有一个主按钮（DESIGN §3.3 第 6 条）。
/// - `.voices` 是别人说过的话，只读：没有回帖、没有赞、没有楼层（DESIGN §3.5）。
/// - `.mine` 是「我的」那个抽屉：一行一项，右边一个「取消」，不分页。
/// - `.sidebar` 桌面 `position:fixed`：栏不随墙滚动；手机上收成顶上一条。
/// - `.cover.word` 四层背景：两层细网格、一团光、一层底色；`b` 是花押、`i` 是角上的 slug。
/// - `.tag::before` 是「正在找人测」前面那个发光的点；推广标没有点。
/// - `.fact i` 是名额那一件事实旁边 28px 的进度线，`--p` 是百分比。
/// - `.verb` 能悬停才出来；触屏（`hover:none`）上一直在；键盘焦点落到卡上也出来。
/// - `.mark` 是栏顶的竖卡线稿；`.wordmark` 在桌面上只留给读屏，手机顶条上才显示。
/// - `.nav-dot` 是当前那间房旁边的一竖琥珀，贴在栏的左边沿。
/// - 发布说明桌面居中、手机从底部升起，都是 `:target`，没有脚本；`.codebox b` 是提示符。
pub const BASE: &str = "\
:root{color-scheme:dark;--bg:#0b0b0d;--rail:#0f0f12;--card:#151518;--card2:#1c1c20;--fg:#f1ede6;--soft:#cbc6be;--dim:#8a867f;\
--line:#ffffff14;--line2:#ffffff24;\
--accent:#ffb224;--accent-ink:#17120a;--accent-soft:#ffb22429;--warn:#f0dcac;--warn-bg:#1f1a10;--warn-line:#443a1e;\
--mono:ui-monospace,SFMono-Regular,Menlo,monospace}\
*{box-sizing:border-box}\
html,body{margin:0;padding:0}\
body{min-height:100vh;background:var(--bg);color:var(--fg);\
-webkit-font-smoothing:antialiased;-webkit-text-size-adjust:100%;\
font:16px/1.65 -apple-system,BlinkMacSystemFont,\"Segoe UI\",\"PingFang SC\",\"Hiragino Sans GB\",\"Microsoft YaHei\",system-ui,sans-serif}\
:focus{outline:none}\
:focus-visible{outline:2px solid var(--accent);outline-offset:3px}\
input:focus-visible,select:focus-visible,textarea:focus-visible{outline-offset:0;border-color:var(--accent)}\
[hidden]{display:none}\
.lead{color:var(--dim)}\
.summary{margin:12px 0 0;color:var(--soft);overflow-wrap:anywhere}\
.row{display:flex;gap:8px}\
.row input{flex:1;min-width:0}\
.row button{width:auto;margin:0;padding:9px 14px;font-size:.9rem;white-space:nowrap}\
input{width:100%;min-height:44px;padding:10px 12px;border:1px solid var(--line);border-radius:10px;\
background:var(--bg);color:var(--fg);font:inherit;font-size:1rem}\
input::placeholder{color:#5c5955}\
.more{display:grid;gap:11px;margin:18px 0 0;font-size:.88rem}\
.more>*{margin:0}\
.more a{text-decoration:none}\
.more a:hover{text-decoration:underline;text-underline-offset:3px}\
.more summary{cursor:pointer;color:var(--dim);list-style:none;-webkit-tap-highlight-color:transparent}\
.more summary::-webkit-details-marker{display:none}\
.more summary::before{content:\"+ \";color:var(--accent)}\
.more details[open] summary::before{content:\"\\2212 \"}\
.more details[open] summary{color:var(--fg)}\
.more form{margin:10px 0 0}\
.more input{font-size:.95rem;padding:9px 11px}\
.more .said{color:var(--dim)}\
@media(prefers-reduced-motion:reduce){*,*::before,*::after{transition:none!important;animation:none!important}}";

pub const CARD: &str = "\
body{display:flex;flex-direction:column;padding:24px 16px;background:var(--bg) \
radial-gradient(60% 32% at 50% 0%,#17140f 0%,#17140f00 70%) no-repeat}\
main.card{width:100%;max-width:30rem;margin:auto}\
.card{background:var(--card);border-radius:18px;overflow:hidden;\
box-shadow:inset 0 0 0 1px var(--line),inset 0 1px 0 #ffffff0d,0 30px 80px -40px #000000e6}\
.hero{display:block;width:100%;aspect-ratio:16/9;object-fit:cover;background:#101012;border-bottom:1px solid var(--line)}\
.hero.word{display:flex;align-items:flex-end;padding:20px 22px;\
background:radial-gradient(120% 90% at 12% 0%,hsl(var(--h) 36% 30%),transparent 60%),\
linear-gradient(150deg,hsl(var(--h) 26% 18%),#0b0b0c 82%)}\
.hero.word span{font-size:1.5rem;font-weight:700;letter-spacing:-.01em;line-height:1.3;color:#ffffffeb;\
overflow-wrap:anywhere;display:-webkit-box;-webkit-line-clamp:3;-webkit-box-orient:vertical;overflow:hidden}\
.body{padding:26px 24px 22px}\
h1{margin:0;font-size:1.55rem;line-height:1.35;font-weight:650;letter-spacing:-.015em;overflow-wrap:anywhere}\
h1 .ver{margin-left:.25em;font-size:.9rem;font-weight:500;color:var(--dim);white-space:nowrap}\
.by{display:flex;align-items:center;gap:8px;margin:0 0 10px;font-size:.9rem;font-weight:500;letter-spacing:.02em;color:var(--dim)}\
.by img{width:26px;height:26px;flex:0 0 auto;border-radius:50%;background:var(--card2);object-fit:cover}\
p{margin:14px 0 0}\
.stamp{margin:12px 0 0;font-family:var(--mono);font-size:.78rem;line-height:1.7;color:var(--dim);\
letter-spacing:.02em;overflow-wrap:anywhere}\
.stamp b{font-weight:400;color:var(--soft)}\
.seats{margin:14px 0 0;font-size:.92rem;color:var(--accent)}\
.seats.full{color:var(--dim)}\
.tip{margin:20px 0 0;padding:14px 16px;border:1px solid var(--warn-line);background:var(--warn-bg);color:var(--warn);border-radius:12px}\
.tip p{margin:0;font-size:.92rem}\
.tip p+p{margin-top:10px}\
button{-webkit-appearance:none;appearance:none;display:block;width:100%;min-height:44px;margin:24px 0 0;padding:15px;\
border:0;border-radius:12px;background:var(--accent);color:var(--accent-ink);font:inherit;font-size:1.05rem;font-weight:700;\
letter-spacing:.03em;cursor:pointer;transition:transform .08s,filter .15s}\
button:hover{filter:brightness(1.06)}\
button:active{transform:translateY(1px) scale(.995)}\
.voices{margin:22px 0 0;padding-top:16px;border-top:1px solid var(--line)}\
.voices h2{margin:0;font-size:.72rem;font-weight:600;letter-spacing:.1em;text-transform:uppercase;color:var(--dim)}\
.voice{margin:12px 0 0;font-size:.92rem;line-height:1.6;color:var(--soft);overflow-wrap:anywhere}\
.voice cite{display:block;margin-top:2px;font-style:normal;font-family:var(--mono);font-size:.72rem;color:var(--dim)}\
.meta{margin:14px 0 0;text-align:center;font-size:.85rem;color:var(--dim)}\
label{display:block;margin:18px 0 0;font-size:.9rem;color:var(--dim)}\
label input,select,textarea{display:block;width:100%;margin-top:6px;padding:10px 12px;border:1px solid var(--line);\
border-radius:10px;background:var(--bg);color:var(--fg);font:inherit;font-size:1rem}\
textarea{resize:vertical;min-height:6.5em}\
code{padding:2px 7px;border-radius:6px;background:#080809;border:1px solid var(--line);font-size:.95em}\
pre{margin:16px 0 0;padding:14px 16px;border-radius:12px;background:#080809;border:1px solid var(--line);overflow-x:auto}\
pre code{padding:0;border:0;background:none}\
footer{display:flex;justify-content:space-between;gap:14px;margin:22px 0 0;padding-top:16px;\
border-top:1px solid var(--line);font-size:.8rem;color:var(--dim)}\
a{color:var(--dim);text-underline-offset:3px}a:hover{color:var(--fg)}";

pub const PAGE: &str = r#"
a{color:inherit;text-decoration:none}
button{-webkit-appearance:none;appearance:none;display:inline-flex;margin:0;padding:0;border:0;background:none;color:inherit;font:inherit;cursor:pointer}
p{margin:0}
.icon{width:20px;height:20px;fill:none;stroke:currentColor;stroke-width:1.6;stroke-linecap:round;stroke-linejoin:round;flex-shrink:0}
.sidebar{position:fixed;inset:0 auto 0 0;width:76px;display:flex;flex-direction:column;align-items:center;padding:18px 10px 14px;border-right:1px solid var(--line);background:var(--rail);z-index:10}
.brand{display:flex;align-items:center;gap:10px;border-radius:10px}
.mark{display:grid;place-items:center;width:38px;height:38px;border-radius:11px;background:linear-gradient(160deg,#202024,#141417);box-shadow:inset 0 0 0 1px var(--line2),inset 0 1px 0 #ffffff14,0 6px 14px -8px #000;color:var(--fg)}
.mark-svg{width:18px;height:18px;fill:none;stroke:currentColor;stroke-width:1.55;stroke-linecap:round;stroke-linejoin:round}
.mark-svg .dot{fill:var(--accent);stroke:none}
.wordmark{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0);font-size:19px;font-weight:700;letter-spacing:-.04em;line-height:1;color:var(--fg);white-space:nowrap}
.wordmark span{color:var(--dim);font-weight:400}
.nav{display:flex;flex-direction:column;gap:6px;width:100%;margin:26px 0 0}
.nav-item{position:relative;display:flex;flex-direction:column;align-items:center;gap:6px;width:100%;padding:10px 0 9px;border-radius:12px;color:var(--dim);font-size:11px;font-weight:500;letter-spacing:.02em;line-height:1;transition:color .15s,background .15s}
.nav-item .icon{transition:transform .2s cubic-bezier(.2,.7,.2,1)}
.nav-item:hover{color:var(--fg);background:#ffffff08}
.nav-item:hover .icon{transform:translateY(-1px)}
.nav-item.active{color:var(--fg);background:#ffffff0a;box-shadow:inset 0 0 0 1px var(--line),inset 0 1px 0 #ffffff0a}
.nav-dot{position:absolute;left:-10px;top:50%;width:4px;height:18px;margin-top:-9px;border-radius:2px;background:var(--accent);box-shadow:0 0 10px var(--accent)}
.sidebar-bottom{width:100%;margin-top:auto}
.publish{color:var(--soft)}
.publish .icon{width:22px;height:22px;padding:4px;border-radius:9px;background:var(--fg);color:var(--bg);box-shadow:0 4px 12px -6px #000}
.publish:hover{color:var(--fg)}
.main{margin-left:76px;min-height:100vh;background:radial-gradient(80% 36% at 50% -8%,#18140c 0%,transparent 70%)}
.content{max-width:112rem;padding:24px 28px 96px}
.grid{display:grid;gap:20px;grid-template-columns:repeat(2,minmax(0,1fr))}
@media(min-width:48rem){.grid{grid-template-columns:repeat(3,minmax(0,1fr))}}
@media(min-width:78rem){.grid{grid-template-columns:repeat(4,minmax(0,1fr))}}
@media(min-width:104rem){.grid{grid-template-columns:repeat(5,minmax(0,1fr))}}
@media(min-width:130rem){.grid{grid-template-columns:repeat(6,minmax(0,1fr))}}
a.tile{display:flex;flex-direction:column;min-width:0;padding:6px;border-radius:16px;background:var(--card);color:var(--fg);box-shadow:inset 0 0 0 1px var(--line),inset 0 1px 0 #ffffff0c;outline-offset:4px;transition:transform .3s cubic-bezier(.2,.7,.2,1),background .3s,box-shadow .3s}
a.tile:hover{transform:translateY(-3px);background:var(--card2);box-shadow:inset 0 0 0 1px var(--line2),inset 0 1px 0 #ffffff14,0 24px 48px -24px #000,0 8px 18px -10px #000c}
.shot{position:relative;overflow:hidden;aspect-ratio:4/3;border-radius:11px;background:#0e0e10;box-shadow:inset 0 0 0 1px #ffffff0a}
.cover{position:absolute;inset:0;display:block;width:100%;height:100%;object-fit:cover;transition:transform .7s cubic-bezier(.2,.7,.2,1)}
a.tile:hover img.cover{transform:scale(1.04)}
.cover.word{background:repeating-linear-gradient(0deg,hsl(var(--h) 30% 65% / .055) 0 1px,transparent 1px 22px),repeating-linear-gradient(90deg,hsl(var(--h) 30% 65% / .055) 0 1px,transparent 1px 22px),radial-gradient(90% 85% at 12% 0%,hsl(var(--h) 42% 36%),transparent 66%),linear-gradient(160deg,hsl(var(--h) 26% 19%),hsl(var(--h) 22% 9%))}
.cover.word b{position:absolute;left:14px;bottom:6px;font-size:58px;line-height:1.1;font-weight:700;letter-spacing:-.04em;color:hsl(var(--h) 45% 90% / .9);text-shadow:0 2px 24px hsl(var(--h) 50% 15% / .7)}
.cover.word i{position:absolute;top:13px;right:12px;font:10px/1 var(--mono);font-style:normal;letter-spacing:.06em;color:hsl(var(--h) 30% 85% / .42)}
.tag,.verb{position:absolute;z-index:1;display:inline-flex;align-items:center;height:24px;padding:0 9px;border-radius:999px;font-family:var(--mono);font-size:11px;letter-spacing:.03em;color:var(--fg);background:#0b0b0dbf;box-shadow:inset 0 0 0 1px #ffffff1f;-webkit-backdrop-filter:blur(10px) saturate(1.2);backdrop-filter:blur(10px) saturate(1.2)}
.tag{top:10px;left:10px;color:var(--accent)}
.tag::before{content:"";width:6px;height:6px;margin-right:6px;border-radius:50%;background:currentColor;box-shadow:0 0 8px currentColor}
.tag.ad{color:var(--soft)}
.tag.ad::before{display:none}
.verb{right:10px;bottom:10px;opacity:0;transform:translateY(4px);transition:opacity .2s,transform .2s}
.verb u{text-decoration:none;margin-left:5px;color:var(--accent)}
a.tile:hover .verb,a.tile:focus-visible .verb{opacity:1;transform:none}
@media(hover:none){.verb{opacity:1;transform:none}}
.tile-body{display:flex;flex-direction:column;gap:4px;padding:12px 8px 8px}
.tile-body h2{margin:0;font-size:15px;line-height:1.4;font-weight:600;letter-spacing:-.01em;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
.tile .summary{margin:0;min-height:1.5em;font-size:13px;line-height:1.5;color:var(--dim);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.seek-k{color:var(--soft);font-weight:500;margin-right:2px}
.tile .meta{display:flex;justify-content:space-between;align-items:center;gap:10px;margin:10px 0 0;padding-top:10px;border-top:1px solid var(--line);font-size:12px;color:var(--dim);text-align:left}
.who{display:inline-flex;align-items:center;gap:7px;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;color:var(--soft)}
.face{width:18px;height:18px;flex:0 0 auto;border-radius:50%;background:var(--card2);object-fit:cover;box-shadow:0 0 0 1px var(--line)}
.fact{display:inline-flex;align-items:center;gap:8px;flex:0 0 auto;font-family:var(--mono);font-size:11.5px;font-variant-numeric:tabular-nums;color:var(--dim)}
.fact i{position:relative;width:28px;height:3px;border-radius:2px;background:#ffffff14;overflow:hidden}
.fact i::after{content:"";position:absolute;inset:0;width:var(--p);border-radius:2px;background:var(--accent)}
.empty-plaza{max-width:32rem;padding:12px 0}
.empty-plaza .lead,.drawer .lead{margin-top:10px;color:var(--dim)}
.main>.drawer{max-width:34rem;margin:24px 28px 80px;padding:28px;border-radius:16px;background:var(--card);box-shadow:inset 0 0 0 1px var(--line),inset 0 1px 0 #ffffff0c}
.drawer h1{margin:0;font-size:22px;font-weight:650;letter-spacing:-.015em}
.drawer .summary{margin:12px 0 0;color:var(--soft)}
.drawer form.row{display:flex;gap:8px;margin:18px 0 0}
.drawer form.row input{flex:1;min-width:0;min-height:44px}
.drawer form.row button,.drawer>form button{display:inline-flex;align-items:center;justify-content:center;width:auto;min-height:44px;margin:0;padding:0 16px;border-radius:10px;background:var(--accent);color:var(--accent-ink);font-size:.95rem;font-weight:650}
.mine{margin:16px 0 0;padding:0;border-top:1px solid var(--line)}
.mine li{display:flex;align-items:center;justify-content:space-between;gap:14px;padding:10px 2px;border-bottom:1px solid var(--line);list-style:none}
.mine li>a{color:var(--fg)}
.mine form,.pushed{display:flex;align-items:center;gap:10px;margin:0}
.mine button,.pushed button,.ghost{display:inline-flex;align-items:center;width:auto;min-height:32px;margin:0;padding:0 12px;border:1px solid var(--line2);border-radius:999px;background:none;color:var(--dim);font-size:.8rem;font-weight:500}
.mine button:hover,.pushed button:hover,.ghost:hover{color:var(--fg);border-color:#ffffff40}
.drawer footer{margin-top:28px;font-size:.85rem;color:var(--dim)}
.more>.ghost{justify-self:start}
.overlay{display:none;position:fixed;inset:0;z-index:40;align-items:center;justify-content:center;padding:24px}
.overlay:target{display:flex}
.overlay-back{position:absolute;inset:0;background:#000000b3;-webkit-backdrop-filter:blur(8px);backdrop-filter:blur(8px)}
.sheet{position:relative;z-index:1;width:min(30rem,100%);max-height:calc(100dvh - 48px);overflow:auto;padding:24px;border-radius:20px;background:radial-gradient(70% 40% at 0% 0%,#ffb2240d,transparent 70%),var(--card);color:var(--fg);box-shadow:inset 0 0 0 1px var(--line2),inset 0 1px 0 #ffffff14,0 40px 100px -30px #000}
.dialog-head{display:flex;align-items:center;justify-content:space-between;gap:16px}
.dialog-head h2{margin:0;font-size:20px;line-height:1.3;font-weight:650;letter-spacing:-.02em}
.close,.tip summary{flex:0 0 auto;display:grid;place-items:center;width:32px;height:32px;border-radius:9px;color:var(--dim);box-shadow:inset 0 0 0 1px var(--line);transition:color .15s,background .15s}
.close .icon,.tip summary .icon{width:15px;height:15px;stroke-width:1.8}
.close:hover,.tip summary:hover,.tip[open] summary{color:var(--fg);background:#ffffff0a}
.pub-box{margin:22px 0 0;border-radius:14px;background:var(--bg);box-shadow:inset 0 0 0 1px var(--line);overflow:hidden}
.tabs{display:grid;grid-template-columns:repeat(3,1fr);gap:3px;margin:0;padding:5px;border-bottom:1px solid var(--line)}
.tabs input{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)}
.tabs label{display:flex;align-items:center;justify-content:center;min-height:34px;margin:0;border-radius:9px;font-size:13px;font-weight:500;color:var(--dim);cursor:pointer}
.tabs label:hover{color:var(--fg)}
.tabs input:checked+label{color:var(--fg);background:var(--card2);box-shadow:inset 0 0 0 1px var(--line2),inset 0 1px 0 #ffffff10}
.tabs input:focus-visible+label{outline:2px solid var(--accent);outline-offset:-2px}
.codebox{display:none;align-items:flex-start;gap:12px;margin:0;padding:20px 18px 22px}
.pub-box:has(#tab-static:checked) #panel-static,.pub-box:has(#tab-local:checked) #panel-local,.pub-box:has(#tab-backend:checked) #panel-backend{display:flex}
.codebox b{flex:0 0 auto;font:13.5px/1.7 var(--mono);font-weight:400;color:var(--accent)}
.codebox code{display:block;padding:0;border:0;background:none;font:13.5px/1.7 var(--mono);color:var(--fg);white-space:pre-wrap;overflow-wrap:anywhere;user-select:all;-webkit-user-select:all}
.tip summary{position:absolute;top:24px;right:64px;cursor:pointer;list-style:none}
.tip summary::-webkit-details-marker{display:none}
.tip-body{margin:12px 0 0;padding:12px 14px;border-radius:12px;box-shadow:inset 0 0 0 1px var(--line);font-size:13px;line-height:1.7;color:var(--dim)}
.tip-body a{color:var(--fg);text-decoration:underline;text-decoration-color:#ffffff3a;text-underline-offset:3px}
@media(max-width:47.99rem){.sidebar{position:static;width:auto;flex-direction:row;flex-wrap:nowrap;align-items:center;gap:8px;padding:10px 12px;border-right:0;border-bottom:1px solid var(--line)}.mark{width:32px;height:32px;border-radius:9px}.mark-svg{width:15px;height:15px}.wordmark{position:static;width:auto;height:auto;clip:auto;font-size:17px}.nav{flex-direction:row;gap:4px;width:auto;margin:0 0 0 auto}.nav-item{flex-direction:row;gap:6px;width:auto;min-height:36px;padding:0 12px 0 10px;border-radius:999px;font-size:13px;letter-spacing:0}.nav-item .icon{width:16px;height:16px}.nav-item.active{background:#ffffff0d;box-shadow:inset 0 0 0 1px var(--line)}.nav-dot{display:none}.sidebar-bottom{width:auto;margin:0}.publish{flex-direction:row;gap:6px;min-height:36px;padding:0 12px 0 8px;border-radius:999px;font-size:13px}.publish .icon{width:20px;height:20px;padding:3px;border-radius:7px}.main{margin:0;background:none}.content{padding:14px 14px 72px}.grid{gap:12px}a.tile{padding:5px;border-radius:14px}.shot{border-radius:10px}.tile-body{padding:10px 6px 6px}.tile-body h2{font-size:14px}.tile .summary{font-size:12.5px}.tile .meta{margin-top:8px;padding-top:8px}.cover.word b{font-size:46px}.cover.word i{display:none}.main>.drawer{margin:14px;padding:22px 18px 24px}.overlay{align-items:flex-end;padding:0}.sheet{width:100%;max-height:90dvh;padding:22px 20px calc(24px + env(safe-area-inset-bottom));border-radius:20px 20px 0 0}.dialog-head h2{font-size:20px}.tip summary{top:22px;right:60px}}
"#;

/// 页面外壳。`head` 放 OG 之类的额外元信息，`body` 放卡片里面的内容。
pub fn shell(title: &str, head: &str, body: &str) -> String {
    shell_hero(title, head, "", body)
}

/// 带一张封面的外壳（门禁页有封面时）：封面顶到卡片边，正文在它下面。
/// `hero` 是已经拼好的 HTML（调用方负责转义）；空字符串就是没有封面。
pub fn shell_hero(title: &str, head: &str, hero: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"zh-CN\">\n<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1,viewport-fit=cover\">\n\
<title>{title}</title>\n{head}<style>{BASE}{CARD}</style>\n\
</head>\n<body>\n<main class=\"card\">\n{hero}<div class=\"body\">\n{body}</div>\n</main>\n</body>\n</html>\n",
        title = esc(title),
    )
}

/// 不套卡片的整页外壳，广场与「我的」用：左边一条栏，右边一面墙。
pub fn page(title: &str, head: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"zh-CN\">\n<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1,viewport-fit=cover\">\n\
<title>{title}</title>\n{head}<style>{BASE}{PAGE}</style>\n\
</head>\n<body class=\"page\">\n{body}</body>\n</html>\n",
        title = esc(title),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_every_dangerous_char() {
        assert_eq!(
            esc("<img src=x onerror=\"a&b\" title='c'>"),
            "&lt;img src=x onerror=&quot;a&amp;b&quot; title=&#39;c&#39;&gt;"
        );
        assert_eq!(esc("《正常的名字》"), "《正常的名字》");
    }

    #[test]
    fn each_page_carries_only_its_two_layers_and_they_stay_small() {
        // 每一页都内联样式；它长一点，每个玩家的第一屏就慢一点（DESIGN §3.3「整页不超过几 KB」）。
        // 卡页 6 KB：门禁页整页（含样式）仍在 11 KB 以内，见 gate.rs 的尺寸测试。
        // 整页 14 KB：广场的墙、栏、卡、发布说明都在里面，没有一条卡页的规则。
        let card = BASE.len() + CARD.len();
        let page = BASE.len() + PAGE.len();
        assert!(card < 6 * 1024, "卡页样式 {card} 字节");
        assert!(page < 14 * 1024, "整页样式 {page} 字节");
        // 两层互不覆盖：整页那一层不该再出现「撤销卡页」的写法。
        assert!(!PAGE.contains("body.page"));
        assert!(!PAGE.contains("filter:none"));
        assert!(!PAGE.contains("transform:none;cursor"));
        // 每一层里没有注释——它们会随页面发给玩家。
        for layer in [BASE, CARD, PAGE] {
            assert!(!layer.contains("/*"));
        }
    }

    #[test]
    fn hero_sits_above_the_body() {
        let html = shell_hero("t", "", "<img class=\"hero\" src=\"/x\">", "<h1>x</h1>");
        let hero = html.find("class=\"hero\"").unwrap();
        let body = html.find("class=\"body\"").unwrap();
        assert!(hero < body);
        assert!(!shell("t", "", "<h1>x</h1>").contains("class=\"hero\""));
    }

    #[test]
    fn hue_is_stable_and_spread() {
        assert_eq!(hue("brisk-otter-41"), hue("brisk-otter-41"));
        assert!(hue("a") < 360);
        assert_ne!(hue("brisk-otter-41"), hue("wise-mink-28"));
    }
}
