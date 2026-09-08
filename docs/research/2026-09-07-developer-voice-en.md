# 英文世界目标用户的原话 · 2026-09-07

> 这一篇只做一件事：把英文社区里目标用户**自己说的话**挖出来、归类、计数。不做产品分析，产品含义单独放在最后一节。
> 引文全部保留英文原文。每一条都是本次抓取里实际读到的文本，没有一条是凭印象补写的。

---

## 0. 怎么抓的，能信到什么程度

**抓取路径**（2026-09-07 12:20–13:15 完成，材料落在 `/tmp`）：

| 来源 | 方式 | 覆盖 |
| --- | --- | --- |
| Reddit 搜索 | 各子版公开 RSS 搜索接口，29 组查询词 | r/gamedev、r/Unity3D、r/godot、r/playmygame、r/vibecoding、r/ClaudeAI、r/webdev |
| Reddit 单帖全评论 | 帖子 `.rss`（top 排序，上限 200 条） | 7 个帖子完整评论树 |
| Hacker News | Algolia 搜索 API + 单帖评论抓取 | 2 个完整帖子 + 1 条定点评论 |

**三个必须先说的限制**，否则下面的数字会被误读：

1. **Reddit 引文一律没有点赞数。** RSS 条目不含 score；今天 13:1x 直接请求 `reddit.com/.../comments/<id>.json` 返回的是反爬页面（`<body class=theme-beta>` 拦截页，不是 JSON）。所以每条 Reddit 引文我只标作者、日期、链接，点赞数写「没取到」。**没有编造任何一个数字。**
2. **HN 评论本身不公开分数**（HN 的产品设计如此），只有故事（story）有 points。所以 HN 引文标故事的 points，评论标 item id。
3. **样本有集中风险。** 主题 ④ 的 22 条引文里有 8 条来自同一个帖子（r/gamedev `1vpbhof`）。这个帖子内容高度贴合我们要找的东西，读起来几乎像是为这次调研准备的——我把它当作**一个**样本而不是八个来权衡，下面的频次表按「帖子数」计数正是为了这个。
4. **三条线没抓到**，在对应主题里写明了「没查到」：Playset / The Gaming Nest / PlayFeed / LocalhostVibe / SIMMER 的发布帖评论（HN Algolia 搜 "Playset" 113 条命中全是玩具和无关同名，没有 playset.games 的帖子）；「web build 比下载版被玩了多少倍」的量化 jam 数据；「免费档角标」的直接评价。

---

## 1. 主题频次

「帖子数」= 我实际读到正文的不同帖子/评论树；「引文数」= 下面列出的原话条数。

| 主题 | 帖子数 | 引文数 | 时间分布 | 密度感受 |
| --- | ---: | ---: | --- | --- |
| ① 怎么把 web build 给朋友 / 测试者 / 评委 | 12 | 14 | 2020–2026，2026 占一半 | 高，而且**答案高度一致**：给链接，别给文件 |
| ② 托管坑（黑屏 / MIME / 压缩头 / COOP-COEP / itch / Safari / 手机） | 14 | 17 | 2017–2026，连续十年不断 | **最高**，且是唯一一个每年都有新帖的主题 |
| ③ 隧道工具在游戏场景 | 3 | 14 | 2021、2025-12、2026-08 | 引文多但**帖子少**——集中在一个 HN 帖里 |
| ④ 反馈收集方式与抱怨 | 9 | 22 | 2025-06 – 2026-08，几乎全在近 15 个月 | 高，情绪最强 |
| ⑤ Jam 场景 | 5 | 5 | 2024–2026 | 低，且没有量化数据 |
| ⑥ 手机上试自己的作品 | 8 | 9 | 2013–2026 | 中低，且**大多不是我们设想的那个痛法** |
| ⑦ 对同类产品的直接评价 | 5 | 8 | 2020、2025-12、2026-06 | 低；点名的那几家一条都没抓到 |
| ⑧ 对付费的态度 | 4 | 10 | 2020、2023、2025-12、2026-03 | 中；集中在「可持续性」而非「值不值」 |
| ⑨ 用 AI 做了小游戏想给人玩 | 10 | 11 | 2025-10 – 2026-08，全部近一年 | 中高，**增速最快** |

一句话读这张表：**②（托管坑）是十年不愈的老病，④（不知道结果）是近一年爆发的新痛，⑨（AI 作者）是新增的人群**；③⑤⑥⑦ 在英文世界的证据比我们预期的薄。

---

## 2. 分主题原话

### ① 「我怎么把 web build 给朋友 / 测试者 / 评委」

**共同答案：给一个浏览器里能开的链接。** 不是因为链接优雅，是因为**别的方式对方不敢碰**。

> "Paid playtesting services exist, but that's studio money — I just want five friends to click around a web build."
> — /u/Edmand46，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/)，点赞数没取到

> "I understand asking someone to download and run an exe from a stranger on social media is a big ask, so is there a better place to ask for that kind of thing? Can I host my dev builds somewhere more trustworthy than a google drive link, distributed via a discord server?"
> — /u/Cudabear，2026-02-01，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1qszcrp/where_to_find_playtesters/)，点赞数没取到

> "In general, beware of any request that ask you install or run anything locally. If you really want to playtest a game, unless you trust the person, you should either: Ask to playtest on the developer's machine directly, or Ask them for a web build (for example, on itch.io) that you can directly play from the browser."
> — /u/Ok-Current-8786，2026-05-28，[r/gamedev · PSA: Beware of playtest request scam](https://www.reddit.com/r/gamedev/comments/1tpum8f/psa_beware_of_playtest_request_scam_that_may/)，点赞数没取到

> "I personally would not install a keylogger just to give feedback. What I would do is link to your html5 game to your friends that is easy and online. I would build a QA copy of your game(the link above). Simple but effective and has worked for me."
> — /u/-hellozukohere-，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wbo19/)，点赞数没取到

> "in the old days just share the executable and no problem. Now...security warnings go off if you try and do that." … "But...there's no way I can expect users of my games to do the same."
> — /u/Haunting_Art_6081（做了 30 年游戏给朋友玩的业余开发者，被 Windows SmartScreen 拦住），2026-04-23，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1stbrzc/im_a_hobbyist_game_developer_i_make_games_for_the/)，点赞数没取到

> "Thus I created a Unity WebGL game (so they don't have to download anything). I've created the game and it works fine when I run it from my localhost. But don't know how to expose it to everyone."
> — /u/DragonWarrior008，2020-04-23，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/g6jrom/any_suggestions_for_free_servers_i_can_host_my/)，点赞数没取到

> "Is there a site where I could upload the game and just get a link to only the game? Or where there's content curation to prevent adult/inappropriate-for-school content?"
> — /u/leftshoe18（给学校三年级学生做数学练习游戏），2023-02-05，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/10ueg8c/familyfriendly_place_to_upload_a_webgl_build/)，点赞数没取到

> "anyone know of another website I could host my game on or how to fix the issue with itch or a way to run my game locally? also It has to be free and preferably on a website that is unlikely to get blocked so i can play it with my friends at school."
> — /u/MacAndCheesy3，2024-02-06，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/1ajxfpc/issues_with_hosting_webgl_build/)，点赞数没取到

> "I wanted a way to easily share my games with friends and family. So I made an Editor Plugin that automatically builds your game and hosts it on the internet with just one button click! It's free If you want to use it."
> — /u/Final_Parsec（帖子标题），2023-02-20，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/11789gv/i_wanted_a_way_to_easily_share_my_games_with/)，点赞数没取到

> "A friend and I have spent way too much time sharing builds so we built this free tool!"
> — /u/LevelPuzzleheaded339（帖子标题），2023-05-04，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/1377h8p/a_friend_and_i_have_spent_way_too_much_time/)，点赞数没取到

> "Frictionless try-before-install. No download, no 'trust me.' If it boots in your browser, you can judge it in 10 seconds." … "Sharable debugging. Repro links are the best bug reports ('open this URL, press R, see the glitch'), which accelerates iteration."
> — /u/PigeonCodeur，2025-08-10，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1mmy1bm/i_finally_shipped_inbrowser_wasm_demos_for_my_c/)，点赞数没取到

> "Share a demo/build for free in itch.io (if it's web is easier for people to play it and get some feedback)"
> — /u/girly_proggrammer，2026-02-01，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1qszcrp/where_to_find_playtesters/o2z5idt/)，点赞数没取到

> "One thing I've used for early prototypes is the password feature on Itch. You can upload builds behind a password if you don't want the general public to see/test it."
> — /u/laughinwhale，2026-02-02，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1qszcrp/where_to_find_playtesters/o330730/)，点赞数没取到

> "You should look at building a website with a domain name to host your game builds if you're not using Steam, they aren't that expensive."
> — /u/cyb_tachyon，2026-02-01，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1qszcrp/where_to_find_playtesters/o2z12v4/)，点赞数没取到

**归类**：14 条里有 5 条把「不用下载 / 不用装」直接说成理由，3 条明确提到**安全与信任**（陌生人 exe、SmartScreen、钓鱼），2 条是开发者忍不住自己造轮子。被推荐得最多的落点是 **itch.io**（4 次，其中 1 次带 password 用法），其次是自建站点、GitHub Pages、Steam Playtest。

---

### ② 托管坑

这是唯一一个 2017 到 2026 每年都能翻到新帖的主题。坑的形状十年没变：**本地能跑，传上去就白屏/黑屏/404**。

> "My WebGL games take forever to load the final 10%. My thought is that I should use brotli or gzip and set up the htaccess file to use that decompression method. Unfortunately so far I've not been able to get this to work and I always get, in my browser, the console warning message that the compression method is not being used correctly. If anyone has knowledge of this and would be able to help, please let me know. I can pay you by the hour."
> — /u/twinmatrix，2020-06-09，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/gzii19/looking_for_paid_help_with_brotligzip_compression/)，点赞数没取到

> "I've kicked a WebGL build of a project that works fine in a local browser, but when I've uploaded the requisite files to the server, I get: 'An error occurred running the Unity content on this page… The error was: Uncaught unknown compression method.'… Unfortunately, my client has his website hosted with Network Solutions, which expressly forbids MIME-editing because of security reasons."
> — /u/DarkSiegmeyer，2017-03-21，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/60q0k3/having_trouble_with_webgl_build_on_server/)，点赞数没取到

> "when we export the web build and host it on Itch.io, we see a 404 missing page error on Safari and Firefox. It works perfectly on Chrome browsers." … "if I export a version without Thread Support, it works fine in Safari but takes 30-40 seconds to load (for everyone), so that isn't ideal either." … "But I worry a lot of people give up before that on the first error."
> — /u/Abandon22，2026-01-09，[r/godot](https://www.reddit.com/r/godot/comments/1q8dkvr/godot_web_export_to_itch_results_in_404_missing/)，点赞数没取到

> "It's such a hybrid bug between Godot, Html5, Itch.io and Safari/Firefox, all working together to break :)"
> — /u/Abandon22（同帖，三天后），2026-01-12，[r/godot](https://www.reddit.com/r/godot/comments/1q8dkvr/godot_web_export_to_itch_results_in_404_missing/nz6laqu/)，点赞数没取到

> "I make many game exclusively to export for the web. With the release of V4 I tried it out and quickly found that there is a new system using SharedArrayBuffers and it just doesnt seem to work. Some quick googling found that the it may work if I uploaded to Itch. I am a teacher and make activities for my students and as you could guess Itch.io is blocked for the school."
> — /u/kylamon1，2023-03-02，[r/godot](https://www.reddit.com/r/godot/comments/11gf3rg/future_of_godot_4_web_export/)，点赞数没取到

> "Github pages don't support many things. Itch does. Sounds like a case of getting itch unblocked, or hosting your own webserver."
> — /u/TheDuriel（同帖回复），2023-03-02，[r/godot](https://www.reddit.com/r/godot/comments/11gf3rg/future_of_godot_4_web_export/jaodi91/)，点赞数没取到

> "That really sucks, and honestly sounds like it should be fixed on Godot's side. If Godot is supposed to run everywhere, this is not it."
> — /u/Lucrecious，2023-03-02，[r/godot](https://www.reddit.com/r/godot/comments/11gf3rg/future_of_godot_4_web_export/jaosftl/)，点赞数没取到

> "Hosting the game on itich.io works, I'd just like a more convenient option for players. The following features required to run Godot projects on the Web are missing: Cross Origin Isolation - Check web server configuration (send correct headers) / SharedArrayBuffer - Check web server configuration (send correct headers)"
> — /u/BreakfastGun（想传到 Ludum Dare 站点），2024-04-17，[r/godot](https://www.reddit.com/r/godot/comments/1c5xynb/error_running_html_export_on_ludum_dare_site/)，点赞数没取到

> "this guide covers using coi to get around a couple of errors (Cross Origin Isolation and SharedArrayBuffer) you get when hosting on GitHub Pages."
> — /u/kirbycope，2024-02-12，[r/godot](https://www.reddit.com/r/godot/comments/1apc1s2/host_your_game_on_github_pages/)，点赞数没取到

> "TLDR: Is it possible to iframe a web exported game from a different domain?… I can access the game at secondarywebsite.com, but whenever trying from mywebsite.com, I get an error: … not-set cross-origin-resource-policy"
> — /u/fluento-team，2023-01-29，[r/godot](https://www.reddit.com/r/godot/comments/10o8typ/export_to_web_and_iframe_from_different_website/)，点赞数没取到

> "Edit: oof, nevermind guys. So there seems to be something wrong with previewing the webgl. If I just accept the black screen after it loads and continue to publish, it works fine. I can't believe I spent 4 hours on this."
> — /u/ItsNotBigBrainTime（Unity Play / play.unity），2025-03-28，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/1jlxlqp/is_there_something_wrong_with_playunity_or_am_i/)，点赞数没取到

> "when I load the game I get the popup 'JavaScript from html.itch.zone … WebGL context lost, please reload the page.' In all the googling I've done, all I can seem to find is 'It doesn't work on Macs,'… 'Use another hosting solution,' which all either are paid or also don't work, and one person saying 'It fixed itself.'"
> — /u/Fake4091（学校作业必须上线），2024-05-11，[r/godot](https://www.reddit.com/r/godot/comments/1cps80y/itchio_webgl_context_lost_please_reload_the_page/)，点赞数没取到

> "When I uploaded my WebGL build, it initially kept telling me the game was too large… Then, the game would not load on Itch.io. It would get to the Unity loading screen, but the loading progress bar would be stuck at the beginning… This was probably because was zipping the folder that the index.html and data folder were in, when you are supposed to zip the files themselves"
> — /u/Ophashias，2025-06-16，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/1lcnw37/june_2025_working_solution_for_unity_webgl_export/)，点赞数没取到

> "I set up an ubuntu server and uploaded my WebGL for beta-testing - it runs! I am trying to improve the performance and it is getting worse. AI (I tried 2!) and me circulating between the index.html, decompressing, loading issues and overload in Safari (Console). Please scroll down directly to 'Problem: Unity's Decompression Fallback vs. server configuration'"
> — /u/Schaever，2025-11-25，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/1p68504/webgl_deployment_fixing_safari_crashes_and_nginx/)，点赞数没取到

> "One was live: two of my games had shipped silent on every iPhone, because touchstart grants no user activation and the resume call was failing quietly. No player ever reported it. iPhone players just assume web games have no sound."
> — /u/MDawg74（11 个浏览器小游戏的作者），2026-08-13，[r/ClaudeAI](https://www.reddit.com/r/ClaudeAI/comments/1vmzdtk/my_dice_game_tenkay_has_12_rivals_with_different/)，点赞数没取到

> "Why mobile web export does not register character input?"
> — /u/Secret_Selection_473（帖子标题），2026-07-26，[r/godot](https://www.reddit.com/r/godot/comments/1v71baw/why_mobile_web_export_does_not_register_character/)，点赞数没取到

> "WebGL is a black screen on some mobile phones?"
> — /u/twinmatrix（帖子标题），2017-12-25，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/7m0q7e/webgl_is_a_black_screen_on_some_mobile_phones/)，点赞数没取到

**归类**：17 条里，压缩头/MIME 4 条、COOP-COEP/SharedArrayBuffer 5 条、黑屏且原因不明 4 条、Safari/移动端专属 4 条。有两条透露了**愿意花钱解决**（"I can pay you by the hour"）。有两条透露了**放弃成本**：`I can't believe I spent 4 hours on this`、`I worry a lot of people give up before that on the first error`。

---

### ③ 隧道工具在游戏场景

**证据结构和我们预期不一样**：引文很多（14 条），但**只有 3 个帖子**，而且几乎全是通用隧道讨论，不是游戏场景。真正「游戏 + 隧道」的抱怨我只抓到 1 条（2021 年的 ngrok 延迟）。

主帖：**Tunnl.gg**，[HN 46145902](https://news.ycombinator.com/item?id=46145902)，**258 points**，113 条评论，2025-12-04。（HN 评论无公开分数，下面标 item id。）

> "Built another localhost tunneling tool because I kept forgetting my ngrok auth token. What it does: Expose localhost to the internet (HTTP/TCP/WebSockets) / Zero signup – just works immediately / Free"
> — klipitkas（作者），2025-12-04，[id=46145903](https://news.ycombinator.com/item?id=46145903)

> "That's really cool. I guess this is an alternative to ngrok (which I like but hate due to having to sign in)."
> — canopi，2025-12-04，[id=46145942](https://news.ycombinator.com/item?id=46145942)

> "This is a great idea but I'm a bit concerned about your bandwidth costs and illegal/malicious content being hosted used under your domain. For the second point, you might want to implement some kind of browser warning similar to what Ngrok does."
> — rany_，2025-12-04，[id=46146384](https://news.ycombinator.com/item?id=46146384)

> "An interstitial warning page is probably the happy medium, and what ngrok already uses now."
> — ronsor，2026-08-05，[id=49190168](https://news.ycombinator.com/item?id=49190168)（另一个帖子里的定点命中）

> "this will attract all kind of bad and malicious users who want nothing more than a 'clean' IP to funnel their badness through. serveo.net [2] tried it 8 years ago, but when I wanted to use it I at some point I found it was no longer working, as I remember the author said there was too much abuse for him to maintain it as a free service" … "Even the ones where you have to register like cloudflare tunnels and ngrok are full of malware, which is not a risk to you as a user but means they are often blocked."
> — gnyman，2025-12-04，[id=46146836](https://news.ycombinator.com/item?id=46146836)

> "tailscale has their own one also called funnel. It has the benefit of being end-to-end encrypted (in theory) but the downside that you are announcing your service to the world through the certificate transparency logs. So your little dev project will have bots hammering on it (and trying to take your .git folder) within seconds from you activating the funnel."
> — gnyman（同上），2025-12-04，[id=46146836](https://news.ycombinator.com/item?id=46146836)

> "That is wrong (and I need to update any docs that mention this), the traffic is not encrypted end to end, we do TLS termination on our side… However I would in any case not suggest to host any production applications using this service. It is mostly for local dev testing."
> — klipitkas，2025-12-04，[id=46146714](https://news.ycombinator.com/item?id=46146714)

> "If you keep this up you'll want to add yourself to the public suffix list: https://publicsuffix.org/ You should also consider grouping your random hostnames under a dedicated subdomain. e.g. 'xxx-xxx-xxx.users.tunnl.gg', that separates out cookies and suchlike."
> — stevekemp，2025-12-04，[id=46146527](https://news.ycombinator.com/item?id=46146527)

> "I run a similar site (https://pico.sh) with public urls and thought the same thing for us. The public suffix has some fuzzy limits on usage size before they will add domains (e.g. on the scale of thousands of active users). I don't have tunnl.gg usage numbers but I'm going to guess they are no where near the threshold — we were also rejected."
> — qudat，2025-12-04，[id=46146844](https://news.ycombinator.com/item?id=46146844)

> "I run playit.gg. Abuse is a big problem on our free tier. I'd get https://github.com/projectdiscovery/nuclei setup to scan your online endpoints and autoban detections of c2 servers."
> — patricklorio，2025-12-04，[id=46153916](https://news.ycombinator.com/item?id=46153916)

> "I had done some account filtering for origins coming out of Tor, VPN networks, data centers, etc. but I recently dropped those and added an portal page for free accounts, similar to what ngrok does. It was very effective at preventing abuse."
> — jborak（packetriot.com 运营者），2025-12-05，[id=46162293](https://news.ycombinator.com/item?id=46162293)

> "Why not just buy trial or cheap VM? Are devs that lazy now? Or is this aimed on vibe 'devs'? :D"
> — Fokamul，2025-12-04，[id=46146860](https://news.ycombinator.com/item?id=46146860)
> 回应："To some people (students, people in low income countries) there are no cheap hosted VMs." — Zambyte，[id=46151991](https://news.ycombinator.com/item?id=46151991)
> 作者回应："Agreed and even devs who have the money, most of the times don't have the time." — klipitkas，[id=46153101](https://news.ycombinator.com/item?id=46153101)

> "Ngrok (https://ngrok.com/) is a pretty good way to do, but there is an insane lag when I try it. Are there any other alternatives for this?"
> — /u/CodeEasyYT（Unity Mirror 联机，想免端口转发），2021-04-05，[r/gamedev](https://www.reddit.com/r/gamedev/comments/mkff21/hosting_with_mirror_but_what_about_port_forwarding/)，点赞数没取到

**归类**：14 条里，**滥用与信誉**占 6 条（这是这个品类被讨论最多的东西，超过速度和易用性）、**免登录**2 条、**可持续性**2 条、**PSL / cookie 隔离**2 条、**游戏场景**1 条。

---

### ④ 反馈收集方式与抱怨

情绪最强、时间最新（22 条里 20 条在 2025-06 之后）。核心不是「收不到反馈」，是**「收到的话没有信息量，而真正有信息量的东西我看不到」**。

> "Every playtest goes the same way: I send the build, they play it, and a few days later I get 'pretty fun, found one bug.' What I actually want to know is whether they understood the mechanic, where they hesitated, what they clicked that wasn't a button. I never find out."
> — /u/Edmand46，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/)，点赞数没取到

> "I tried asking people to screen record. Turns out nobody wants to install OBS and upload a 1GB file just to do me a favor."
> — /u/Edmand46（同上）

> "This. What they tell you is nearly irrelevant. What they do is important."
> — /u/earlyworm，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wgfyy/)，点赞数没取到

> "All of my friends/family play testers told me the game is awesome. The longest session was 11 minutes."
> — /u/Salty_Dig8574，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wbewz/)，点赞数没取到

> "'pretty fun, found one bug.' basically means 'not fun'. Just to let you know."
> — /u/Sycopatch，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wrwth/)，点赞数没取到

> "friends and family are worse than useless for feedback as they will lie to you"
> — /u/ZoopeeDoopeeDoo，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wg4p8/)，点赞数没取到

> "On web builds I just stopped asking friends to record stuff. I log a few events (first click, time to first real action, where ppl drop) and look at that. 5 testers is enough if you dont ask 'was it fun' and ask like 3 real questions instead, like where did you get lost. And yeah everyone ignores the tutorial button."
> — /u/quiet_scope，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3xglws/)，点赞数没取到

> "If it's a web game. Something like Sentry could be super nice. It'll track literally everything they do on the page while on it. Also for your needs you could definitely use the free version"
> — /u/madmelonxtra，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3xh797/)，点赞数没取到

> "The fix that worked for me was to stop asking and start watching. Friends say 'it's fun' because they're being kind, and because when they get stuck they ask you a question and you answer it — which quietly deletes the exact thing you needed to see." … "My most useful playtest was an App Review rejection: the reviewer tapped the board with nothing selected, got no response because I'd never handled that case, and concluded the game was broken. Nobody who knew me ever hit it, because they all knew to pick a piece first."
> — /u/Rainicy925，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3x4wx5/)，点赞数没取到

> "I personally would not install a keylogger just to give feedback."
> — /u/-hellozukohere-，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wbo19/)，点赞数没取到
> 作者解释「不装东西、只在标签页内、开始前告知」之后，对方仍然回：
> "Ya fair enough sounds like it was like download this. Trust me bro lol"
> — /u/-hellozukohere-，2026-08-15，[同帖](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wvkn4/)

> "Give up on Google Forms. Our previous playtests used Google Forms, and we got significantly less feedback. This time, we just posted a message in Discord explaining what we'd like to know… Moving the feedback to Discord probably felt less formal, and whatever the reason, it worked much better for us."
> — /u/RiftedSkies（VR 团队，60 人申请 / 40 人真玩 / 约一半留了反馈），2026-08-28，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1w0wl0u/how_many_people_do_you_actually_need_for_a_useful/)，点赞数没取到

> "Online playtests are usually more promotion than anything else and if you're looking for anything actionable from it stick to analytics more than feedback forms. Track how long players play and what they do in the game, like if they are losing to a particular fight or level at a different rate than you expect, time spent in a UI that's more/less than what you pictured, if people quit after a particular moment and things like that."
> — /u/MeaningfulChoices，2026-08-28，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1w0wl0u/how_many_people_do_you_actually_need_for_a_useful/p6gdg0x/)，点赞数没取到

> "You don't get good playtesting feedback by posting builds online. Playtests are best conducted privately and in person, because you learn so much more by watching someone play your game than any comment they might leave after."
> — /u/MeaningfulChoices，2026-02-01，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1qszcrp/where_to_find_playtesters/o2z4gh1/)，点赞数没取到
> （同一段被另一位用户完整引用后反驳：
> "I upvote this every time I see it because I know it is correct, but at the same time I feel like it's generally untenable for a majority of smaller indie devs… it's pretty common for people to say things like 'no one I know cares about what I'm making'"
> — /u/disgustipated234，2026-02-01，[同帖](https://www.reddit.com/r/gamedev/comments/1qszcrp/where_to_find_playtesters/o2z7vwy/)）

> "Like people don't have the words to describe what's happening and yes, watching people use your software will humble you haha, but it's insights you could NEVER get by having them type in their thoughts to a form. 1000x maybe 100000x as valuable."
> — /u/icpooreman，2026-08-29，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1w0wl0u/how_many_people_do_you_actually_need_for_a_useful/p6lmmcu/)，点赞数没取到

> "I'm prototyping a new game and had around 10 playtesters. 8 of them mentioned the game taking them X amount of time to complete, when the logs showed that they were still working on finishing the game for much longer than that. For example, the most egregious example was one player saying that it took them 15 min to finish the playtest when they were clearly playing for 45 minutes."
> — /u/HostOfRats，2026-07-16，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1uxz92c/playtesters_thought_the_game_was_significantly/)，点赞数没取到

> "The feedback was rough. Every single time: 'feels too early.' 'completely broken.' 'lacks depth.' But 'early' and 'depth' is so vague. We spent countless hours brainstorming just to figure out what testers were actually trying to tell us. Translating a feeling into a concrete problem is honestly the hardest part."
> — /u/ExhaustedBonfire，2026-05-10，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1t96nyr/how_do_you_handle_vague_playtester_feedback/)，点赞数没取到

> "Launched the playtest with 350 wishlists, reached 850 wishlists after a month. 800 people signed up to play. 270 people actually loaded it up. 31 minutes average play time, 10 minutes median play time."
> — /u/jak12329（Steam Playtest 复盘），2025-11-10，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1otocey/steam_playtest_postmortem_everyone_should_do_them/)，点赞数没取到

> "It's should be easy for playtester no extra work, otherwise people will not do it. Rewards are great too"
> — /u/Sunslap-Kristina，2025-06-20，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1lfvmsz/how_do_you_collect_playtest_feedback_when_your/myrj9uo/)，点赞数没取到

> "I almost always push for recordings. The amount of times I've had people playtest my game without recording and then they just say something like 'It looked good' or 'Great Game' annoyed me. On the other hand when I've had people play it on a stream or recorded I often see things that they didn't even know is wrong"
> — /u/RiskyBiscuitGames，2025-06-21，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1lfvmsz/how_do_you_collect_playtest_feedback_when_your/mz0x4le/)，点赞数没取到

> "What's the best Game Analytics stack (lightweight and easy to integrate) to run on a WebGL/PC build on itch.io? (Considering data privacy and compliance)."
> — /u/Ok-Quail-7896，2026-05-23，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1tlmih7/what_is_the_best_approach_to_conducting_a_playtest/)，点赞数没取到

> "And is it normal when the game automatically opens a feedback form for the player after the game session ends?"
> — /u/MonthFragrant7804，2026-02-27，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1rg1c89/how_to_collect_feedback_correctly/)，点赞数没取到

> "We have people using our analytics platform (in beta) for play testing… I think I under estimated (I am the product person), how important that would be as an early use case for indie teams."
> — /u/gamerslab_official，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wdw4g/)，点赞数没取到

> "Try to get feedback during gameplay. It has helped me gather feedback to be like 1000% less work."
> — /u/-hellozukohere-，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wbo19/)，点赞数没取到

**归类**：22 条里，「说的话没用」9 条、「主张改看行为数据」6 条、「表单/录屏摩擦太大」4 条、「隐私反弹」2 条、「远程不如线下」2 条（其中 1 条被同帖用户当场反驳为对独立开发者不现实）。

---

### ⑤ Jam 场景

**这个主题最弱，而且弱得有信息量**：抓到的 jam 相关痛点全都是**导出与托管**，没有一条是关于「评委打不开」或「web 版比下载版多多少人玩」的。后者**没查到**。

> "Hosting the game on itich.io works, I'd just like a more convenient option for players."（想传 Ludum Dare 官方站，撞 COOP/COEP）
> — /u/BreakfastGun，2024-04-17，[r/godot](https://www.reddit.com/r/godot/comments/1c5xynb/error_running_html_export_on_ludum_dare_site/)，点赞数没取到

> "We participated in the GMTK 2024 game jam, and our game was all about sending and receiving data from an online database… I decided to export my game as soon as possible and upload it to itch.io to see if it would work. Spoiler: it didn't. I spent around four hours trying to figure out a solution"
> — /u/Powerful_Case_9542，2024-08-27，[r/godot](https://www.reddit.com/r/godot/comments/1f2kezo/how_we_successfully_used_the_httprequest_node_in/)，点赞数没取到

> "So I made an all-in-1 video to share with my game jam partners on how to set up c# web export with the legacy engine version."（Godot 4 的 C# 还不能 web 导出，只能退回 Godot 3）
> — /u/enigma2728，2026-05-03，[r/godot](https://www.reddit.com/r/godot/comments/1t2pevj/tutorial_using_godot3_c_for_web_export_for_game/)，点赞数没取到

> "I just uploaded my tech demo, that is actually a game that I made for a game jam in 48H with some bug fixes!"
> — /u/PigeonCodeur，2025-08-11，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1mmy1bm/i_finally_shipped_inbrowser_wasm_demos_for_my_c/n86xzac/)，点赞数没取到

> "Free tier for hobby/game jam projects, $19/mo for indie devs shipping commercially"（Godot CI/CD 服务的定价设想，把 jam 当免费档的典型场景）
> — /u/LimpOutlandishness87，2026-03-06，[r/godot](https://www.reddit.com/r/godot/comments/1rm3lrb/w4_build_is_dead_im_building_the_managed_cicd/)，点赞数没取到

---

### ⑥ 手机上试自己的作品

**痛法和 DESIGN M1 的设想只对上一半。** 找到的抱怨集中在「怎么把本地服务弄到手机上看」和「手机上表现不一样」，**没有一条**明确抱怨「WebXR / 陀螺仪 / 摄像头需要 HTTPS，局域网 IP 是 HTTP」——这一条**没查到**（不等于不存在，只是本次抓取里没有）。

> "Just chat, download the HTML file, playtest on my phone, screenshot what feels wrong, paste it back, iterate."
> — /u/Dapper_Dingo4617，2026-03-17，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1rwm9gb/i_prototyped_and_built_a_logic_puzzle_game/)，点赞数没取到

> "Working on a site that I'm building with Astro and am looking for something that will allow me to emulate the localhost server on a mobile device."
> — /u/danielrosehill，2024-06-27，[r/webdev](https://www.reddit.com/r/webdev/comments/1dpmdk4/mobile_emulator_for_testing_site_on_mobile/)，点赞数没取到

> "The bottom line is how to access a WordPress site locally (via MAMP) on a different computer. I can see the (MAMP) root directory of the localhost (by typing in 'mylocal_ip_address:8888'), but I cannot access any of the WordPress site in the directory."
> — /u/MidtownBlue，2023-10-22，[r/webdev](https://www.reddit.com/r/webdev/comments/17dw708/how_to_access_a_localhost_wordpress_site_on/)，点赞数没取到

> "Each of this method is a huge time cost for me! Is there any chance a program can help not wasting my time with all this step? Maybe there is something allowing us to put png on a app folder and directly test it on the device? Or something like putting your computer screen directly on your mobile screen?"
> — /u/falsuss，2013-12-09，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1si07z/how_to_easily_test_my_drawable_icons/)，点赞数没取到

> "The real test with an ECS game is whether or not it's performant… it runs very smoothly on my phone in a web browser."
> — /u/Kitae，2026-03-30，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1s7wann/knights_vs_archers_unit_combat_simulator/)，点赞数没取到

> "The lag: I don't know what it is - are the channels backlogged or what - But I would say 20-30% of devices are _unplayable_. It's not like the inputs are delayed, it's that they're fully unpredictable about whether they will register (and for how long). Same browsers, same Android OEM, same Wifi network and just... lag."
> — OsrsNeedsf2P，2025-12-28，HN [id=46408316](https://news.ycombinator.com/item?id=46408316)（故事 [Show HN: Gaming Couch](https://news.ycombinator.com/item?id=46344573)，**437 points**，121 条评论，2025-12-21）

> "There's actually a demo implementation with few games that support fully remote gaming but the problem I've been running into is some memory limitations on iOS devices causing crashes. On Android everything seems to work a-ok."
> — ChaosOp（Gaming Couch 作者），2025-12-26，HN [id=46390562](https://news.ycombinator.com/item?id=46390562)

> "Why mobile web export does not register character input?"
> — /u/Secret_Selection_473，2026-07-26，[r/godot](https://www.reddit.com/r/godot/comments/1v71baw/why_mobile_web_export_does_not_register_character/)，点赞数没取到

> "WebGL is a black screen on some mobile phones?"
> — /u/twinmatrix，2017-12-25，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/7m0q7e/webgl_is_a_black_screen_on_some_mobile_phones/)，点赞数没取到

---

### ⑦ 对同类产品的直接评价

**点名的六家（Playset / The Gaming Nest / PlayFeed / tunr / LocalhostVibe / SIMMER）的发布帖评论，一条都没查到。** HN Algolia 搜 "Playset" 得到 113 条命中，全部是玩具、NSA Playset、Playmobil 之类的同名内容，没有 playset.games。下面是抓到的**同类别但非点名**的评价。

> "The product: The accessibility is key. Telling people to 'scan the QR code' is great for getting it going."
> — OsrsNeedsf2P（用 Gaming Couch 和约 12 个人玩了两天后的反馈），2025-12-28，HN [id=46408316](https://news.ycombinator.com/item?id=46408316)

> "Instead of a central screen, what about just having the first client get a URL or code they can share in a group chat. I think this will help remove a lot of friction in getting this setup by piggybacking on top of existing platform's groups."
> — charcircuit，2025-12-26，HN [id=46391193](https://news.ycombinator.com/item?id=46391193)

> "people are making tiny games with Claude/Cursor/Replit, but then the games end up as local files, random deploy links, or dead demos. So I built a very small publish/play flow: upload a single HTML game, or publish from Claude through MCP / get an instant playable link / share it with friends / games run sandboxed on a separate origin"
> — /u/Ok-Goal1907（Clawcade，clawcade.gg），2026-06-19，[r/vibecoding](https://www.reddit.com/r/vibecoding/comments/1uafzsp/i_made_a_tiny_publishplay_layer_for_vibecoded/)，点赞数没取到
> ——**这是本次调研里离 playtest.run 最近的一个东西，DESIGN §2 没有它。**

> "I builded my game as WebGL, but it's not working… I ignored it and tried to upload it on simmer.io web site, well it accepted my folder and it displayed my game in the preview, it was working fine. But when I saved and updated, my game page only shows a black screen"
> — /u/kops182（SIMMER 唯一一条实际使用记录），2020-02-04，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/eyh6lc/games_not_loading_on_localhost_after_building_it/)，点赞数没取到

> "I recommend one where I do a lot of playtesting under the name 'Nahnachi'. I think there is an option to put a game there without paying at first, and there is also a rating feature for the developers to rate the reviews, so everyone is incentivized to take it seriously"
> — /u/Nah_Nachi（推荐 play2review.com），2026-08-16，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p41wfip/)，点赞数没取到

> "Since no one wants to play my prototype (especially for more than 10 minutes of the tutorial), I went to Fiverr and hired 'testers' there, lol. It cost me $200 for 7 people."
> — /u/OmiNya，2025-01-29，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1iclsg3/how_i_went_to_fiverr_because_nobody_wanted_to/)，点赞数没取到

> "I gave a GDC talk about this earlier this year. It's all about how to run 1 on 1 synchronous playtests on Discord/Google Meet etc… And you don't need to spend money to do it if you don't want."
> — /u/MurphyAt5BrainDamage，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wldby/)，点赞数没取到

> "Yes discord screenshare, I don't let my friends play without it :D"
> — /u/W0RKABLE，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wb6im/)，点赞数没取到

---

### ⑧ 对付费的态度

讨论几乎不围绕「值多少钱」，而围绕**「你会不会跑路」和「我信不信你」**。「免费档角标」的直接评价**没查到**，最接近的是运营者视角说 ngrok 式门户页对反滥用有效。

> "The question is, how is it sustainable? Nobody likes being rug pulled. Why not charge money for it? I'd rather pay a few dollars for a service that will be around 5 years from now, than pay nothing and have to deal with churn."
> — zarzavat，2025-12-04，HN [id=46146694](https://news.ycombinator.com/item?id=46146694)

> "To keep this up and running for 2-3 years, you probably do need to be rich, or to find a way to monetize. It's possible when it gets to be a drain, even charging pennies for the service could drive off the bad actors making it unsustainable though."
> — pcthrowaway，2025-12-04，HN [id=46147947](https://news.ycombinator.com/item?id=46147947)

> "Fair enough from a business standpoint, but seeing as there are massive privacy/security risks involved in exposing your data to an opaque service, the open source component is probably a non-optional aspect of the value prop."
> — popalchemist，2025-12-04，HN [id=46151975](https://news.ycombinator.com/item?id=46151975)

> "how come? just because it's open source doesn't mean that they run that exact binary on their servers. ngrok does pretty well without open sourcing."
> — rgbrgb（反驳上一条），2025-12-04，HN [id=46153366](https://news.ycombinator.com/item?id=46153366)

> "The locus of trust moves, if you have the source, and trust is a factor for you, because you can simply self-host and know what you're running."
> — popalchemist，2025-12-05，HN [id=46159491](https://news.ycombinator.com/item?id=46159491)

> "I can pay you by the hour. It should be fairly easy too."（为搞定 Brotli/gzip 响应头愿意按小时付费）
> — /u/twinmatrix，2020-06-09，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/gzii19/looking_for_paid_help_with_brotligzip_compression/)，点赞数没取到

> "Unable to Build Game Through Batch mode Only (Willing to pay for help)"（标题）… "willing to give you a few bucks to help me out!"
> — /u/VizeKarma，2023-11-05，[r/Unity3D](https://www.reddit.com/r/Unity3D/comments/17o6k41/unable_to_build_game_through_batch_mode_only/)，点赞数没取到

> "Free tier for hobby/game jam projects, $19/mo for indie devs shipping commercially" … "Would you use a free tier with 100 build minutes/month?"
> — /u/LimpOutlandishness87，2026-03-06，[r/godot](https://www.reddit.com/r/godot/comments/1rm3lrb/w4_build_is_dead_im_building_the_managed_cicd/)，点赞数没取到

> "And yeah, I know the community has strong opinions about commercial services in the Godot ecosystem. I respect that. The core build runner will be open source. The managed service (dashboard, deploy integrations, team features) is what you'd pay for. Same model as Supabase, GitLab, etc."
> — /u/LimpOutlandishness87（同上），2026-03-06

> "I am paying for it out of pocket. Its free for you to use, but not for me to host it :)"
> — klipitkas，2025-12-04，HN [id=46146481](https://news.ycombinator.com/item?id=46146481)

---

### ⑨ 「用 AI 做了个小游戏想给人玩」

11 条**全部在最近一年**（2025-10 至 2026-08）。他们的产物形态高度一致：**单个 HTML 文件或一个浏览器链接**；他们的困扰也高度一致：**做出来了，但没有一个体面的地方放，也不知道有没有人真的玩**。

> "people are making tiny games with Claude/Cursor/Replit, but then the games end up as local files, random deploy links, or dead demos."
> — /u/Ok-Goal1907，2026-06-19，[r/vibecoding](https://www.reddit.com/r/vibecoding/comments/1uafzsp/i_made_a_tiny_publishplay_layer_for_vibecoded/)，点赞数没取到

> "(Full disclosure: I built it heavily with AI coding tools — side project, no way I'd have shipped it otherwise.)"
> — /u/Edmand46，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/)，点赞数没取到

> "Feedback is really hard to get, and it's even harder to get when your game is ai generated. I'm guessing it's because you get the kind of effort they think you give. Even if it's a great game, they don't know how much you leaned on AI, and typically assume the worst."
> — /u/luke_skippy，2026-08-15，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1vpbhof/sent_my_game_to_friends_for_playtesting_all_i_got/p3wmq8l/)，点赞数没取到

> "Everything lives in a single self-contained HTML file. No dependencies, no build step. Open it in a browser and play. The development loop was: Claude writes the code, I download the file, playtest it, screenshot anything that feels off, paste the screenshot back into chat, fix, repeat."
> — /u/Dapper_Dingo4617，2026-03-17，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1rwm9gb/i_prototyped_and_built_a_logic_puzzle_game/)，点赞数没取到

> "Free, no signup, no ads, plays on your phone… The numbers: one hand-editable HTML file, 434KB, canvas and Web Audio, zero dependencies, zero assets loaded from anywhere. 25 versions to get here. It's game number 11 in a browser arcade I run the same way."
> — /u/MDawg74，2026-08-13，[r/ClaudeAI](https://www.reddit.com/r/ClaudeAI/comments/1vmzdtk/my_dice_game_tenkay_has_12_rivals_with_different/)，点赞数没取到

> "1 vs Computer (fully local, no server) and 1 vs 1 with a friend over a shareable URL - one player hosts, the other just clicks a link in their browser. No accou[nts]"
> — /u/walm00，2026-07-05，[r/vibecoding](https://www.reddit.com/r/vibecoding/comments/1unzsoq/made_a_browseronly_desert_strikestyle_lane/)，点赞数没取到

> "Looking for honest feedback. This is my first ever attempt at coding. Please consider giving this a try. It's free."
> — /u/NYTD-Blue，2026-08-05，[r/vibecoding](https://www.reddit.com/r/vibecoding/comments/1vgncya/i_vibe_coded_a_social_prediction_game_and_need/)，点赞数没取到

> "One random comment I left on another post sent 1,000+ new players to the web version over the weekend… I did not have enough analytics around CTA clicks and funnel behavior"
> — /u/MightyBig-Dev，2026-05-04，[r/vibecoding](https://www.reddit.com/r/vibecoding/comments/1t31wvm/i_got_1000_users_from_one_reddit_comment_and/)，点赞数没取到

> "It's called HistoryClue.com (still polishing and playtesting). Let me know what your honest opinions are please!"（Next.js + Supabase，部署在 Vercel）
> — /u/freeman9235，2025-10-06，[r/ClaudeAI](https://www.reddit.com/r/ClaudeAI/comments/1nzrwc5/i_built_a_full_web_based_historical_detective/)，点赞数没取到

> "All of the games have been developed by me and two of my friends. We do use Cursor and models like Gemini & Claude for generic debugging as part of everyday development tools… However it's not what non-programmers usually think, that you can just write one prompt and generate a game :D"
> — ChaosOp，2025-12-26，HN [id=46392014](https://news.ycombinator.com/item?id=46392014)

> "Vibe-coding full projects is largely a myth and today's models and agents aren't able to build anything more than prototypes."（反面证据，避免只挑顺耳的）
> — /u/lpshred，2026-04-23，[r/gamedev](https://www.reddit.com/r/gamedev/comments/1sty2af/postmortem_i_tried_and_failed_vibe_coding_a/)，点赞数没取到

---

## 3. 意外发现

**（1）「不要下载」在英文社区已经不是偏好，是安全共识。**
我原以为「web build 更方便」是主要理由，实际上翻出来的是三条独立的、非偏好性的理由：r/gamedev 的钓鱼 PSA 直接教人「让对方给你一个浏览器里能玩的 web build」；一个做了 30 年游戏的业余开发者被 Windows SmartScreen 拦到无法把 exe 给朋友；`asking someone to download and run an exe from a stranger on social media is a big ask`。也就是说，**浏览器链接正在变成「唯一还能递出去的东西」**，这比「方便」硬得多，也更不可逆。

**（2）录制与埋点在开发者社区会被当场叫成 keylogger。**
`I personally would not install a keylogger just to give feedback` —— 而且作者解释完「不装东西、只在标签页内、事先告知」之后，对方仍然回 `Trust me bro lol`。**这是对 Playset 那条录像路线最直接的社会成本证据**，也说明我们做 SDK 和会话记录时，「解释清楚」不够，得让**玩家看得见**（DESIGN §3.3 门禁页告知 + §3.4 不收集身份，方向是对的，但强度可能还不够）。

**（3）进 Public Suffix List 是会被拒的。**
DESIGN §4.1 把「把 `playtest.run` 提交进 PSL」写成一个步骤。pico.sh 的运营者在 HN 上说：`The public suffix has some fuzzy limits on usage size before they will add domains (e.g. on the scale of thousands of active users)… we were also rejected.` **我们在有几千活跃用户之前大概率进不去。** 这不影响两域名分离（那一半是我们自己能做到的），但影响「子域之间彻底隔离」这句话的成立时间。

**（4）离我们最近的对手不在 DESIGN §2 的名单上，而且是 2026 年 6 月才出现的。**
Clawcade（clawcade.gg）：`upload a single HTML game, or publish from Claude through MCP / get an instant playable link / share it with friends / games run sandboxed on a separate origin`。**「通过 MCP 从 Claude 里直接发布」这个入口我们完全没想过**，而且它对「用 AI 写小东西的人」这一类的贴合度比 CLI 更高——那类人未必开终端，但一定在跟模型对话。

**（5)「知道结果」最强的证据不是开发者要一个 dashboard，是他们已经在土法自建。**
`I log a few events (first click, time to first real action, where ppl drop)`；`the logs showed that they were still working on finishing the game for much longer than that`（用日志校正玩家自述的 15 分钟 vs 实际 45 分钟）；`two of my games had shipped silent on every iPhone… No player ever reported it`（靠自己审计才发现，玩家从不上报）。**这三条是同一件事：玩家说的和发生的对不上，而开发者已经在用自己写的最粗糙的手段去补这个差。**

**（6）一条真实的逆风：社区里最权威的声音在劝人别做远程 playtest。**
`You don't get good playtesting feedback by posting builds online. Playtests are best conducted privately and in person`——这条被同一个人在两个帖子里说了，还被第三个人主动引用并说 `I upvote this every time I see it because I know it is correct`。**我们卖的正是「远程」。** 值得庆幸的是紧接着的反驳同样有力：`it's generally untenable for a majority of smaller indie devs`。这说明我们的定位不该是「替代线下观察」，而是「你线下看不了的那些人，至少别是黑箱」。

---

## 4. 对 playtest.run 的含义

### 4.1 支持现有结论的

| DESIGN 里的判断 | 支持它的原话 |
| --- | --- |
| §0.1 起点是场景不是隧道；上传占多数 | ③ 只有 3 个帖子、1 条真正的游戏隧道抱怨；①②④ 加起来 53 条全是「静态构建 + 链接」的世界 |
| §1.2 M2「zip 对方不会跑」 | SmartScreen 那条把它升级成了「exe 根本递不出去」；钓鱼 PSA 把它升级成「递出去也没人敢开」 |
| §3.3 门禁页 = 用户手势（音频） | `two of my games had shipped silent on every iPhone, because touchstart grants no user activation` |
| §3.3 门禁页 = 反滥用 | packetriot 运营者：`added an portal page for free accounts, similar to what ngrok does. It was very effective at preventing abuse.` |
| §3.2 终端二维码 | `Telling people to "scan the QR code" is great for getting it going.` |
| §4.2 wasm / 预压缩 / COOP-COEP / `--isolated` | ② 的 17 条，跨 2017–2026，压缩头 4 条、COOP-COEP 5 条 |
| §4.1 两个域名分开 | `You should also consider grouping your random hostnames under a dedicated subdomain… that separates out cookies and suchlike.` |
| §7 开源 | `the open source component is probably a non-optional aspect of the value prop`（有对立意见 `ngrok does pretty well without open sourcing`，说明这是取舍不是定论） |
| §3.4 不收集玩家身份 | keylogger 那一串对话 |

### 4.2 与 DESIGN 冲突或需要改的（建议，不动 DESIGN）

1. **§3.4 的措辞「谁打开了」建议改成「有多少人打开、玩了多久、在哪儿断」。**
   22 条反馈类原话里，**没有一条**表达「我想知道是哪个人玩的」。全部指向行为：卡在哪、玩多久、有没有加载失败、在哪儿退出。「谁」这个字在英文社区会直接触发隐私反弹（见意外发现 2）。产品能力不用变，**这是文案问题，但是个会决定第一印象的文案问题**。

2. **§1.2 建议补第六个时刻，或把 M2 拆开：「让他敢点这个链接」。**
   这一格的证据强度超出预期：钓鱼 PSA（2026-05）、SmartScreen（2026-04）、`a big ask`（2026-02）、ngrok 警告页（2025-12 与 2026-08 两处）、学校/公司把 itch.io 封了（2023、2024 两处）。现在 DESIGN 把门禁页的价值写成「用户手势 + 会话起点 + 版本告示牌 + 举报入口」，**唯独没写它最值钱的那一条：它是这条链接的信任凭证**——一个写着真人名字和作品名的页面，正是 exe 和裸隧道链接给不了的东西。

3. **§4.1 的 PSL 建议改成「申请，但不依赖」。**
   pico.sh 被拒，门槛大约在「数千活跃用户」量级。v0.1 不会有。

4. **§2 建议补 Clawcade（2026-06，clawcade.gg），并单列一条「MCP / Agent 入口」的观察。**
   它不是威胁在体量上，而在**入口位置**：`publish from Claude through MCP`。对「用 AI 写小东西的人」，从对话里一句话发布，比记住一个 CLI 命令更短。DESIGN §3.2 说「CLI 之外的形态开源后希望社区做」——MCP 这一条可能不该等社区。

5. **§8 的 T6（每版反馈数 ≥ 1）可能偏乐观，建议把「第一层数据被用」也设成通过条件之一。**
   原话里对文字反馈的态度是普遍悲观的（`Give up on Google Forms`、`'It looked good' or 'Great Game' annoyed me`、`vague`），而对行为数据的态度是主动的（`stick to analytics more than feedback forms`、`What's the best Game Analytics stack…`）。如果 T6 为零但开发者天天看第一层数据，那不该触发「砍掉 SDK」，该触发「SDK 从反馈按钮改成事件」。

6. **隧道的证据比 DESIGN 假设的薄，T4 更重要了。**
   英文世界里「隧道 + 游戏」我只找到一条真实抱怨（2021 年 ngrok 延迟）。隧道社区自己在讨论的是滥用、封禁、可持续性——**这些是运营成本，不是用户需求**。建议 §8 把 T4 的判据往前提：私测第一周就看隧道占比，低于 10% 就别在 v0.1 里做 QUIC 的准备工作。

### 4.3 三个问题的直接回答

**(a) 最高频的痛点是什么？**

**「送出去之后什么都不知道，而收回来的话没有信息量。」** 这是唯一一个 22 条引文、9 个帖子、且几乎全部集中在最近 15 个月的主题，也是情绪最强的一个（`I never find out`、`worse than useless`、`basically means "not fun"`）。

紧随其后的是 **②托管坑**（17 条、14 帖、跨越十年）。两者的关系值得注意：**托管坑是「频率最高」的，反馈黑箱是「最痛」的**。前者决定用户装不装我们，后者决定用户留不留下来。DESIGN §0.4 说「名字里的 playtest 兑现在知道结果」，这个判断被数据支持；但 §0 没说清楚的是——**入口是托管，不是结果**。

**(b) 有没有一个我们没在 §1.2 五个时刻里写的？**

**有：「让对方敢点开」这一格，即链接的可信度。**
它现在被拆散在 M2（zip 不会跑）和 M5（ngrok 弹警告页）里，但它其实是独立的一格，而且是唯一一个**在 2026 年还在恶化**的（SmartScreen 收紧、Discord 钓鱼、隧道域名被批量封）。它的形状是：开发者递出去的东西必须让对方相信「点这个不会中毒、不会被公司网络拦、不需要我装什么」。这一格正好是门禁页能兑现的，但 DESIGN 目前没把它写成门禁页的价值主张。

次要的一格是「**反馈要对得上版本和会话**」——`Translating a feeling into a concrete problem is honestly the hardest part`、`the logs showed that they were still working on finishing the game for much longer`。DESIGN §3.5 已经做了版本归档，只是没把它当成一个「时刻」。

**(c)「知道谁玩了」是开发者主动要的，还是我们想给的？**

**「玩成了什么样」是他们主动要的，有明确证据；「谁」是我们想给的，且有反向证据。**

主动要的一侧，四条独立证据：

- `On web builds I just stopped asking friends to record stuff. I log a few events (first click, time to first real action, where ppl drop) and look at that.` —— 已经自己动手做了第一层数据。
- `stick to analytics more than feedback forms. Track how long players play and what they do in the game` —— 行业老手给新人的默认建议。
- `What's the best Game Analytics stack (lightweight and easy to integrate) to run on a WebGL/PC build on itch.io?` —— 主动开帖找工具，而且关键词就是 **lightweight and easy to integrate**，正好是 DESIGN §2.3「比 Playset 更轻」的那条路。
- `800 people signed up to play. 270 people actually loaded it up… 10 minutes median play time.` —— 开发者自己会去算这三个数，说明「多少人真的打开了」是他们的原生指标，不是我们发明的。

反向的一侧：**没有任何一条原话要求知道玩家是谁**，而且有两条明确的隐私反弹（keylogger / Trust me bro）。

结论：DESIGN §3.4 的**能力清单是对的**（打开数、独立人数、设备、来源、加载失败、错误、一句话反馈），**但第一句话「谁打开了」应该换掉**。他们要的是一面镜子，不是一份名单。
