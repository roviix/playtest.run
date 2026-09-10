# 2026-09-09 · 「俱乐部」那一层在控制台和 SDK 上长出来了（对着假数据）

**结论**：DESIGN §3.3–§3.6 与 §3.11 里开发者要管的六件事——邀请卡、名额、开发者的群、
让玩家看到彼此的反馈、关注、推广——在控制台作品页上都有了；时间线那段话多了「来自：邀请卡 4 ·
广场 2 · 微信 2」和「N 位留了名字」，点名册有名字，反馈流能逐条隐藏；SDK 反馈提交完不再自动关掉，
而是落到 §4.6 那一屏（谢谢 / 留邮箱 / 看看别的作品）。

**这次是对着一份假控制面做的，没有和真控制面或真边缘联调过。** 控制面的 `seats` / `followers` /
`sources` / `named` / `feedback_public` / `boost` 这几张表和边缘的邀请卡、`/_playtest/follow`
这两天正由另一条线在写，所以这边照着 `common/src/` 的契约摆了一份形状对得上的数据，先把每一块的
版式、空态和失败态看清楚。第七节列了全部没验的地方。

机器：macOS 25.6，node v22.22.0 / npm 10.9.4，TypeScript 5.9，vite 7.3.6，
Chrome 152.0.7977.83（`--headless=new`，420 宽的手机视口）。

## 一、构建与类型检查

```
$ npm --prefix console run build            # = tsc --noEmit && vite build
dist/index.html                  0.54 kB │ gzip:  0.33 kB
dist/assets/index-CHfgDg9l.css   6.60 kB │ gzip:  2.17 kB
dist/assets/index-5lNWSYAd.js   43.77 kB │ gzip: 16.09 kB
✓ built in 159ms

$ npm --prefix sdk run check                # tsc --noEmit
$ npm --prefix sdk run build
dist/playtest.js: 6650 字节，gzip 后 3192 字节（上限 4096）
```

`sdk/dist/playtest.js` 已重建并进仓库（边缘 `include_bytes!` 的就是它）。控制台的 package.json
里只有 `dev` / `build` / `preview` 三条，没有测试脚本，所以「跑测试」这一项这次是空的。

## 二、假控制面长什么样

`console/dev/mock-api.mjs`（无依赖 Node，听 `127.0.0.1:8799`，只在开发时用，不进构建产物）：

```
$ node console/dev/mock-api.mjs
$ PLAYTEST_API=http://127.0.0.1:8799 npm --prefix console run dev
```

两个作品，专门做成一满一空，好让「0 的句子不说」这条规矩两边都能看见：

| | `brisk-otter-41`《潮汐小镇》 | `sage-lynx-30`《一个小工具》 |
| --- | --- | --- |
| `seats` / `joined` | 10 / 6 | 无 / 0 |
| `followers` | 12 | 0 |
| `community_url` | `https://example.com/qun` | 无 |
| `feedback_public` | true | false |
| `boost` | `days7` · `live` · 到 9-15 | 无 |
| 邀请卡 | 有 | 404 |

三个版本的 results：v7 有 `sources: [card 4, plaza 2, wechat 2]` 与 `named: 4`；v6 有
`[wechat 3, notice 2]` 与 `named: 1`；v5 是 `sources: []` / `named: 0` 且
`dropped_before_first_frame: null`（没接 SDK 的那一版）。点名册 8 行，其中 4 行有 `name`。
反馈 3 条，覆盖「有名字 / 没名字」× 「public true / false」。PATCH 按契约实现：`seats: 0`
和 `community_url: ""` 是清掉，不是设成 0 和空串。

邀请卡返回的是一张 **SVG 占位**（路径仍是 `/_playtest/card.png`，浏览器按 Content-Type 认），
照 DESIGN §3.4 的版式摆了封面位、「小海 邀请你试玩《潮汐小镇》」、一句简介、
「v7 · 9 月 9 日 · 在找 10 位试玩者」、二维码方块和一条撕票线。真的那张由边缘按当前版本光栅化，
这里只是让版式和失败态看得见。

## 三、控制台每一块

| 图 | 看到的 |
| --- | --- |
| [作品列表](img/console-club-1-sites.png) | 第一张卡多一行「12 人关注 · 名额 6 / 10 位」；第二张卡两个都是 0，整行不出现 |
| [邀请卡](img/console-club-2-invite-card.png) | 卡的预览（`?v=7` 防缓存）、「保存图片」「复制链接」、「发到群里，别人长按识别就能玩」 |
| [名额与群](img/console-club-3-seats-community.png) | 「在找 10 位，已有 6 位加入。」「留了名字的人算加入；到齐之后玩家仍然可以玩。」；群的输入框校验 http(s) |
| [公开反馈 / 关注 / 推广](img/console-club-4-public-followers-boost.png) | 「12 人关注着这个作品，下一版发出去他们会收到通知。」「信由我们代发，你看不到他们的邮箱。」；推广那一节显示「推广 7 天 · 推广中，到 9 月 15 日」 |
| [时间线](img/console-club-5-timeline.png) | 三个版本的那段话，见下 |
| [点名册](img/console-club-6-roster.png) | 有名字的加粗（阿树 / 小雨 / 老周 / 柚子），没名字的仍是会话 id 头 6 位；来源是中文 |
| [反馈流](img/console-club-7-feedback.png) | 每条署名（没有则「一位试玩者」），带「公开中 / 已隐藏」和「隐藏 / 恢复」 |
| [隐藏一条之后](img/console-club-8-feedback-hidden.png) | 第一条从「公开中」变「已隐藏」，按钮变「恢复」 |
| [什么都还没设的作品](img/console-club-9-nothing-yet.png) | 邀请卡失败态、名额空、群空、公开关着、「还没有人关注。」、推广那段「尚未开放」 |

时间线三个版本的原话（从 DOM 里读出来的，不是手打的）：

```
v7  8 个人打开，6 个人进到游戏（2 个在加载时走了）。
    4 位留了名字，3 个人玩了 5 分钟以上，2 个人回来过，停留中位数 45 秒；来自：邀请卡 4 · 广场 2 · 微信 2。
    1 个错误撞了 3 次（TypeError: Cannot read 'x' of undefined @ main.js:412）。
    2 条反馈。

v6  1 位留了名字，1 个人玩了 5 分钟以上，停留中位数 25 秒；来自：微信 3 · 通知 2。

v5  4 个人打开，4 个人点了开始。
    这一版没接 playtest.js，加载时走掉几个人看不出来。
    1 个人玩了 5 分钟以上，停留中位数 40 秒。
```

v5 那一版 `named` 是 0、`sources` 是空，两句都没出现——「0 的句子不说」在新加的两处也成立。
「N 人关注着这个作品」放在页头而不是紧贴「关注」那一节：截图时发现挨在一起像同一句话说了两遍。

点名册里出现了「来自邀请卡」「来自广场」「来自通知」「来自直接打开」，以及邀请卡 + 微信里打开
同时出现在一行上——微信里长按识别二维码进来的人两个标都该有。

**推广那一节没有任何按钮。** 没有 `boost` 时只有一段字：「推广位尚未开放：付款通道要等主体落地。
开放后可以买 3 天或 7 天，作品会出现在广场顶部并标『推广』，免费流不受影响。」

**关注那一节不显示任何邮箱**，只有数字和一句「信由我们代发，你看不到他们的邮箱」（DESIGN §3.6）。

## 四、SDK 玩后那一屏

假边缘：一个 Node 静态服务器，服务 `fixtures/phaser-jump/export`，在 `</body>` 前插一行
`<script src="/_playtest/sdk.js">`，并实现 `/_playtest/me`、`/v1/ingest/session`、
`/_playtest/follow` 三条（脚本在 `/tmp/`，没进仓库）。

| 图 | 看到的 |
| --- | --- |
| [反馈按钮](img/sdk-club-1-button.png) | 右下角，和以前一样 |
| [反馈面板](img/sdk-club-2-panel.png) | 输入框 + 「发出去」 |
| [提交之后](img/sdk-club-3-landing.png) | 「谢谢，开发者会看到。」+ 邮箱输入「有新版本时告诉我」+「告诉我」；底下小字「看看别的作品」和「关掉」 |
| [整页 POST 之后](img/sdk-club-4-follow-posted.png) | 假边缘渲染的结果页 |

假边缘收到的两笔：

```
反馈收到：{"session":"a1b2…","slug":"brisk-otter-41","text":"退潮那一下很好看，第三关的桥找不到","seconds_in":1}
关注表单：{"target":"site:brisk-otter-41","from":"sdk","to":"/","email":"someone@example.com"}
```

四个字段名和 `common/src/follow.rs` 的 `form::*` 对得上，`target` 是 `site:<slug>` 的形状。
表单是原生 `<form method="post" action="/_playtest/follow">`，整页 POST 到同源边缘——
SDK 不发 fetch、不读响应、不种 cookie，边缘渲染结果页再给一条「返回」链接回作品。

**只做邮箱，不做浏览器通知**：作品域上注册我们的 Service Worker 会顶掉开发者自己的 SW
（一个作用域只能有一个），Phaser / Unity 这类导出经常自带一个。理由写在 `sdk/src/playtest.ts`
的注释里。边缘的 SW 只注册在根域，和这条不冲突。

「看看别的作品」的根域是去掉当前主机名的第一个标签算出来的：`brisk-otter-41.playtest.run`
→ `playtest.run`，本机 `xxx.localhost:8443` → `localhost:8443`；算不出来（主机名只有一段）
就不显示这条链接。SDK 里没有硬编码域名。

## 五、对控制面 API 的假设

TS 那边是照着 `common/src/` 手抄的，下面几条是这次新加、还没被真的响应打过的：

- `GET /v1/sites/{slug}` 返回单个 `Site`（控制台原先只有列表；设置那一块要在 PATCH 之后拿回最新值）。
- `PATCH /v1/sites/{slug}` 接 `seats` / `community_url` / `feedback_public`；`seats: 0` 清掉名额，
  `community_url: ""` 清掉群。
- `PATCH /v1/sites/{slug}/feedback/{id}` 的 body 现在是 `{status?, public?}`，两个都可选。
- `POST` `SITE_COVER_FROM_FEEDBACK`（`/v1/sites/{slug}/cover/from-feedback/{id}`）。
- `Boost.reason`：控制台在 `rejected` 时会显示 `boost.reason`，但 `common/src/boost.rs` 的 `Boost`
  **没有**这个字段（只有 `ReviewBoostRequest` 有）。TS 里写成可选，注释说明了：控制面补上就显示，
  没有就只说「没通过」。

## 六、「设为封面」现在不会出现

`FeedbackItem.screenshot_hash` 有值才渲染那个按钮，而现在没有任何一条路径会给它赋值
（SDK 不截图），所以真实数据里这个按钮永远不出现——这是有意的：DESIGN §3.9 要这个能力，
代码留着接口，界面上不放一个点了没用的按钮。假数据里也没造 `screenshot_hash`，
所以上面的反馈流截图里看不到它。

## 七、没验的

1. **没和真控制面联调过**。第五节那五条假设全部只对着 `console/dev/mock-api.mjs` 跑过。
   控制面这几天正在长这些表，等它起来要再走一遍：新字段的名字、`seats: 0` / `community_url: ""`
   的清除语义、`SITE_COVER_FROM_FEEDBACK` 的方法和返回。
2. **没和真边缘联调过**。邀请卡用的是假控制面画的 SVG 占位，`/_playtest/follow` 用的是
   `/tmp/` 里的假边缘。真卡的尺寸、真表单的字段校验和结果页都没看过。
3. **跨源 `<a download>` 会被浏览器忽略**：邀请卡在作品域上，控制台在
   `playtest.roviix.com`，`download` 属性对跨源资源不生效，多半会变成在新标签页打开。
   所以加了 `target="_blank"`，说明里也写了「手机上长按这张图也能存下来」。这条只在无头 Chrome
   里推理过，没在真手机上试。
4. **只在无头 Chrome 里看过**：没上真手机、Safari、微信内置浏览器。420 宽的视口能看，
   但微信里那一层的字号和长按存图没验。
5. `vite preview` 不带 `/v1` 代理，所以截图走的是 `npm run dev`——和 2026-09-07 那条 spike
   记的缺口是同一个，没修。
6. 控制台没有测试脚本，这次只有 `tsc --noEmit` + `vite build` 把关。
