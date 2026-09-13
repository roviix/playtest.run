#!/usr/bin/env python3
"""
seed_demo_data.py - 为本地开发与体验生成丰富真实的多样化演示数据

包含：
1. 开发者身份绑定 (永久有效 GitHub 开发者账户，绑定本地 CLI 令牌)
2. 7 个各具特色的作品（游戏、Web 工具、文章阅读、实机视频、微型游戏）
3. 真实文件与可游玩 Demo (Blob 存储、清单 Manifest、Live 状态)
4. 多种门禁卡片形态（4:3 凭证画框、物理打孔撕票线、高对比钛白按键、艺术封面 vs 几何星轨画框）
5. 控制台点名册（Sessions 多平台各操作系统真实访问记录）与玩家反馈（Feedback）
6. 2 个合集与创作挑战（Autumn Indie Showcase, 72h Microgame Jam）
7. 玩家关注中心数据（订阅广场周报与作品更新，一键登录 Token）
8. 广场 plaza.json 同步更新与推广位配置
"""

import hashlib
import json
import os
import shutil
import sqlite3
import time
from datetime import datetime, timezone, timedelta

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
DATA_DIR = os.path.join(ROOT, ".data")
STORE_DIR = os.path.join(DATA_DIR, "store")
BLOBS_DIR = os.path.join(STORE_DIR, "blobs")
SITES_DIR = os.path.join(STORE_DIR, "sites")
DB_PATH = os.path.join(DATA_DIR, "api.sqlite")
CLI_CONFIG_PATH = os.path.expanduser("~/.config/playtest/config.json")

def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def put_blob(data: bytes) -> str:
    h = sha256(data)
    prefix = h[:2]
    d = os.path.join(BLOBS_DIR, prefix)
    os.makedirs(d, exist_ok=True)
    p = os.path.join(d, h)
    with open(p, "wb") as f:
        f.write(data)
    return h

def now_iso(offset_hours: float = 0) -> str:
    dt = datetime.now(timezone.utc) + timedelta(hours=offset_hours)
    return dt.strftime("%Y-%m-%dT%H:%M:%SZ")

def main():
    print("🌱 正在初始化 Playtest 本地全套体验数据...")

    os.makedirs(BLOBS_DIR, exist_ok=True)
    os.makedirs(SITES_DIR, exist_ok=True)

    # 1. 检查或读取 CLI 配置里的 Token
    token = None
    if os.path.exists(CLI_CONFIG_PATH):
        try:
            with open(CLI_CONFIG_PATH, "r", encoding="utf-8") as f:
                cfg = json.load(f)
                token = cfg.get("token")
        except Exception:
            pass

    if not token:
        token = "demo-developer-token-2026-secret"

    token_hash = sha256(token.encode("utf-8"))
    user_id = "286ee20e-5418-4096-bbfa-d1402d51f82a"

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()

    # 确保 user_id 存在且为 Github 开发者身份
    cur.execute("SELECT id FROM users WHERE id = ?", (user_id,))
    row = cur.fetchone()
    if row:
        cur.execute("""
            UPDATE users SET 
                kind = 'github',
                display_name = 'Zhongshang Wu',
                login = 'zhongshangwu',
                avatar_url = 'https://avatars.githubusercontent.com/u/1024025?v=4',
                expires_at = NULL
            WHERE id = ?
        """, (user_id,))
    else:
        cur.execute("""
            INSERT INTO users (id, kind, display_name, created_at, expires_at, github_id, login, avatar_url)
            VALUES (?, 'github', 'Zhongshang Wu', ?, NULL, 1024025, 'zhongshangwu', 'https://avatars.githubusercontent.com/u/1024025?v=4')
        """, (user_id, now_iso(-24)))

    # 确保 token 存在且永不失效
    cur.execute("DELETE FROM tokens WHERE token_hash = ?", (token_hash,))
    cur.execute("""
        INSERT INTO tokens (token_hash, user_id, created_at, expires_at)
        VALUES (?, ?, ?, NULL)
    """, (token_hash, user_id, now_iso(-24)))

    print(f"👤 开发者账号已就绪：Zhongshang Wu (@zhongshangwu)")

    # 准备基础文件 Blobs
    # 鹈鹕单车 HTML
    pelican_html_path = os.path.join(ROOT, "fixtures/pelican-bicycle/export/index.html")
    with open(pelican_html_path, "rb") as f:
        pelican_html = f.read()
    pelican_hash = put_blob(pelican_html)

    # Phaser Jump
    phaser_html_path = os.path.join(ROOT, "fixtures/phaser-jump/export/index.html")
    with open(phaser_html_path, "rb") as f:
        phaser_html = f.read()
    phaser_hash = put_blob(phaser_html)

    # Vite Vanilla
    vite_html_path = os.path.join(ROOT, "fixtures/vite-vanilla/export/index.html")
    with open(vite_html_path, "rb") as f:
        vite_html = f.read()
    vite_hash = put_blob(vite_html)

    # 头像工作室 (ODD FOLK)
    odd_folk_html = """<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>ODD FOLK — 原创头像工作室</title>
<style>
body { margin: 0; background: #09090b; color: #f4f4f5; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; }
.card { background: #18181b; border: 1px solid #27272a; border-radius: 16px; padding: 32px; text-align: center; max-width: 400px; box-shadow: 0 10px 25px -5px rgba(0,0,0,0.5); }
.avatar-box { width: 160px; height: 160px; margin: 0 auto 20px; background: #27272a; border-radius: 50%; display: flex; align-items: center; justify-content: center; border: 3px solid #75cdb5; overflow: hidden; }
svg { width: 120px; height: 120px; }
h2 { margin: 0 0 8px; font-size: 20px; letter-spacing: -0.02em; }
p { color: #a1a1aa; font-size: 14px; margin: 0 0 24px; line-height: 1.5; }
.btn-group { display: flex; gap: 12px; justify-content: center; }
button { background: #ffffff; color: #09090b; border: none; padding: 10px 18px; border-radius: 8px; font-weight: 600; font-size: 14px; cursor: pointer; transition: transform 0.1s; }
button:active { transform: scale(0.96); }
button.sec { background: #27272a; color: #f4f4f5; }
</style>
</head>
<body>
<div class="card">
  <div class="avatar-box" id="av">
    <svg viewBox="0 0 100 100">
      <circle cx="50" cy="50" r="45" fill="#1e293b"/>
      <circle cx="35" cy="45" r="5" fill="#75cdb5"/>
      <circle cx="65" cy="45" r="5" fill="#75cdb5"/>
      <path d="M 35 68 Q 50 80 65 68" stroke="#f8fafc" stroke-width="4" fill="none" stroke-linecap="round"/>
      <rect x="42" y="20" width="16" height="12" rx="4" fill="#38bdf8"/>
    </svg>
  </div>
  <h2>ODD FOLK 像素工坊</h2>
  <p>轻触换一组特征，拼出专属你的独立创作者头像。</p>
  <div class="btn-group">
    <button onclick="randomize()">🎲 随机特征</button>
    <button class="sec" onclick="alert('已复制 SVG 矢量代码！')">导出 SVG</button>
  </div>
</div>
<script>
const colors = ['#75cdb5', '#38bdf8', '#fb7185', '#fbbf24', '#a78bfa'];
function randomize() {
  const c = colors[Math.floor(Math.random() * colors.length)];
  document.getElementById('av').querySelector('svg').innerHTML = `
    <circle cx="50" cy="50" r="45" fill="#1e293b"/>
    <circle cx="35" cy="45" r="${Math.random()>0.5?5:7}" fill="${c}"/>
    <circle cx="65" cy="45" r="${Math.random()>0.5?5:7}" fill="${c}"/>
    <path d="M 32 65 Q 50 ${Math.random()>0.5?78:55} 68 65" stroke="#f8fafc" stroke-width="4" fill="none" stroke-linecap="round"/>
    <circle cx="50" cy="22" r="6" fill="${c}"/>
  `;
}
</script>
</body>
</html>""".encode("utf-8")
    odd_folk_hash = put_blob(odd_folk_html)

    # 极简轨道游戏 (Tiny Orbit)
    tiny_orbit_html = """<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Tiny Orbit</title>
<style>
body { margin: 0; background: #050608; overflow: hidden; display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100vh; font-family: monospace; color: #a1a1aa; }
canvas { background: #090a0f; border: 1px solid #1f242d; border-radius: 12px; box-shadow: 0 0 40px rgba(0,0,0,0.8); }
.hint { margin-top: 16px; font-size: 13px; color: #71717a; }
</style>
</head>
<body>
<canvas id="c" width="380" height="380"></canvas>
<div class="hint">按住鼠标 / 触屏：施加引力推进 · 松开：惯性离心</div>
<script>
const c = document.getElementById('c'), ctx = c.getContext('2d');
let angle = 0, r = 110, vr = 0, pressing = false;
let score = 0, alive = true;
window.addEventListener('mousedown', () => pressing = true);
window.addEventListener('mouseup', () => pressing = false);
window.addEventListener('touchstart', e => { e.preventDefault(); pressing = true; }, {passive:false});
window.addEventListener('touchend', () => pressing = false);
function loop() {
  ctx.fillStyle = 'rgba(9, 10, 15, 0.2)';
  ctx.fillRect(0, 0, 380, 380);
  ctx.beginPath();
  ctx.arc(190, 190, 110, 0, Math.PI * 2);
  ctx.strokeStyle = '#1e2430';
  ctx.stroke();

  // 中心恒星
  ctx.beginPath();
  ctx.arc(190, 190, 24, 0, Math.PI * 2);
  ctx.fillStyle = '#75cdb5';
  ctx.shadowColor = '#75cdb5';
  ctx.shadowBlur = 15;
  ctx.fill();
  ctx.shadowBlur = 0;

  if (alive) {
    if (pressing) vr -= 0.25; else vr += 0.2;
    vr *= 0.96;
    r += vr;
    angle += 0.04;
    score++;
    if (r < 30 || r > 175) alive = false;
  }

  const x = 190 + Math.cos(angle) * r;
  const y = 190 + Math.sin(angle) * r;
  ctx.beginPath();
  ctx.arc(x, y, 5, 0, Math.PI * 2);
  ctx.fillStyle = alive ? '#ffffff' : '#ef4444';
  ctx.fill();

  ctx.fillStyle = '#f4f4f5';
  ctx.font = '14px sans-serif';
  ctx.fillText('航行周期: ' + Math.floor(score / 10), 20, 30);
  if (!alive) {
    ctx.fillStyle = '#fb7185';
    ctx.fillText('偏离轨道！点击刷新重新发射', 95, 230);
  }
  requestAnimationFrame(loop);
}
loop();
</script>
</body>
</html>""".encode("utf-8")
    tiny_orbit_hash = put_blob(tiny_orbit_html)

    # 文章 Safe HTML Artifact
    article_html = """<h2>从扁平圆角卡到实体入场券的演变</h2>
<p>在早期的移动端原型设计中，我们采用了比较普遍的宽屏 20:9 封面搭配窄条信息。但在真实设备上手把玩时，我们立刻发现了严重的视觉疲劳与节奏压抑：</p>
<blockquote>
<p>“扁平的圆角矩形削弱了入场与试玩的物理隐喻，让本应具备收藏价值与仪式感的 Playtest Pass 退化成了冰冷的弹窗。”</p>
</blockquote>
<h3>黄金比例与物理细节</h3>
<ul>
<li><strong>4:3 凭证视窗</strong>：将头图比例规范为 4:3（400x300），为场景构图与氛围原画留出充裕的呼吸空间。</li>
<li><strong>微型半圆打孔槽（Ticket Punch）</strong>：在主画框与副券正文之间设计了两侧对齐的 16px 凹坑，配合 <code>1px dashed</code> 虚线，还原出实体门票被撕下后的断裂质感。</li>
<li><strong>纯钛白高对比按键</strong>：在深黑曜背景（<code>#09090b</code> ~ <code>#121215</code>）上，纯钛白底面（<code>#ffffff</code>）配合墨黑文字带来高达 18:1 的确定性触感。</li>
</ul>
<p>这种纯粹中性高对比的设定，不仅让视线瞬间聚焦于第一核心行动，更让冷萃绿（<code>#75cdb5</code>）退回到了真正珍贵的焦点准星，形成了耐看、沉稳的专业工业质感。</p>"""
    article_artifact_hash = put_blob(article_html.encode("utf-8"))

    # 小说连载 3 章节 Safe HTML Blobs
    ch1_text = """<h2>第一章：沉睡的三百年</h2>
<p>低温休眠舱的气阀在零下四十度的空气中喷吐出一缕霜白色的气雾。</p>
<p>陆巡睁开眼时，视神经花了足足三秒钟才重新建立起微弱的色彩感度。耳边没有蜂鸣，没有告警，只有「深空信标 07 号」核心堆芯那熟悉而沉闷的次声频脉动——每四点二秒震颤一次，像一颗被冰封在金属胸膛里的重型心脏。</p>
<blockquote>
<p>“主脑日志显示：最后一次对地通信握手记录在九万七千天之前。握手协议中断，重连尝试次数：已达上限。”</p>
</blockquote>
<p>舷窗外并不是想象中耀眼的星海，而是死寂的暗区。在太阳系柯伊伯带边缘的外侧，行星与太阳都已缩略成了背景噪声中不易察觉的光点。陆巡扶着舱壁站直身体，肌肉里残存的冷冻液让指尖微微颤抖。</p>
<p>工作台上的老式全息投显跳动了一下，闪烁出一行橙红色的物理字符：</p>
<pre><code>BEACON-07 // ANOMALOUS ECHO DETECTED AT 14.8 AU</code></pre>
<p>十四点八个天文单位以外，正有什么东西，逆着太阳风的吹拂，以绝对均匀的加速度向着信标站的方向逼近。</p>"""
    ch1_bytes = ch1_text.encode("utf-8")
    ch1_hash = put_blob(ch1_bytes)

    ch2_text = """<h2>第二章：奥尔特云的谐波</h2>
<p>分析仪的示波器屏幕上，那道引力波回声展现出令人不安的对称性。</p>
<p>自然天体的引力扰动往往伴随着杂乱的高频毛刺，而面前这条波形却平滑得像用高阶多项式拟合出来的正弦曲线。</p>
<p>“这不是彗星分裂，也不是小行星摄动。”陆巡咬碎了一粒合成咖啡胶囊，苦涩的提神剂让他的思维迅速冷凝，“它的频率是 1420.405 MHz——中性氢的辐射线，但它上面加载了三重相位翻转。”</p>
<p>那是人类在二十世纪中叶向深空发出的第一个坐标握手协议。</p>
<p>有人在三百年后，把人类自己扔向虚空的火把，原封不动地掷了回来。或者说……有人在用人类的母语，向整个奥尔特云广播一个求救代码。</p>
<p>陆巡启动了信标站的高增益抛物面天线。巨大的冷凝器开始加压，金属骨架在极端温差下发出轻微的呻吟。如果这一束确认脉冲发射出去，信标站的储能将直接跌入临界阈值，但如果保持沉默，这个飞掠而过的谜题将在三十六小时后彻底坠入未知的星际荒原。</p>"""
    ch2_bytes = ch2_text.encode("utf-8")
    ch2_hash = put_blob(ch2_bytes)

    ch3_text = """<h2>第三章：未知的引力回声</h2>
<p>天线充能进度：98%……99%……发射。</p>
<p>一道肉眼无法看见的高相干微波束刺破了柯伊伯带的万古死寂。微弱的蓝光在离子推进喷口边缘一闪而过。</p>
<p>随后是长达七分钟的等待。在光速都要行走漫长尺度的深空里，沉默就是最漫长的凌迟。</p>
<p>七分十四秒。</p>
<p>信标站的舱壁突然剧烈地震颤起来。这不是无线电波的接收，而是整个局部空间的微度扭曲——引力透镜效应在距离信标站仅三十公里的真空中撕开了一条肉眼可见的微弱亮线。</p>
<p>暗区中，一艘通体覆盖着黑色吸光晶体的梭形构装体滑出了虚空。它的表面没有任何焊接缝隙，只有一道由浅蓝色流光构成的信标指示灯，正以一秒一次的频率，与陆巡胸口的心跳监视器发生着完全一致的同频共振。</p>
<p>主控屏上的自检日志跳出了最终解密行：</p>
<pre><code>WELCOME BACK, EXPLORER.</code></pre>"""
    ch3_bytes = ch3_text.encode("utf-8")
    ch3_hash = put_blob(ch3_bytes)

    # 封面图片（选用现成的高质量 PNG 截图作为封面）
    cover_bike_src = os.path.join(ROOT, "docs/spikes/img/2026-09-13-finesse-card-detail.png")
    with open(cover_bike_src, "rb") as f:
        cover_bike_bytes = f.read()
    cover_bike_hash = put_blob(cover_bike_bytes)

    cover_rain_src = os.path.join(ROOT, "docs/spikes/img/2026-09-10-plaza-night-desktop.png")
    with open(cover_rain_src, "rb") as f:
        cover_rain_bytes = f.read()
    cover_rain_hash = put_blob(cover_rain_bytes)

    cover_avatar_src = os.path.join(ROOT, "docs/spikes/img/2026-09-12-avatar-native-characters.png")
    with open(cover_avatar_src, "rb") as f:
        cover_avatar_bytes = f.read()
    cover_avatar_hash = put_blob(cover_avatar_bytes)

    # 视频演示（生成一段微型有效 MP4 或使用现有视频文件）
    # 创建一个轻量的模拟 MP4 字节或使用现成的视频文件
    fake_video_bytes = b"\x00\x00\x00\x18ftypmp42\x00\x00\x00\x00isommp42\x00\x00\x00\x08free\x00\x00\x00\x10mdat\x00\x00\x00\x00\x00\x00\x00\x00"
    video_hash = put_blob(fake_video_bytes)

    # 2. 定义 7 个精选作品数据
    projects = [
        {
            "slug": "lucky-robin-21",
            "title": "鹈鹕单车",
            "summary": "一辆穿梭在海港小镇的复古单车，收集风与信件。",
            "note": "重点测试第 2 关的惯性物理手感与雨水反光",
            "kind": "web",
            "engine": "godot",
            "seats": 10,
            "joined": 7,
            "seeking": True,
            "public": True,
            "boosted": False,
            "players": 28,
            "followers": 14,
            "cover": {"hash": cover_bike_hash, "size": len(cover_bike_bytes), "mime": "image/png"},
            "version": 2,
            "files": [{"path": "index.html", "hash": pelican_hash, "size": len(pelican_html)}],
            "feedbacks": [
                ("过弯物理惯性手感很扎实！希望能加个手刹键漂移", "macOS · Chrome", "阿遥", "done"),
                ("音乐和雨夜海港画面太治愈了，已加入愿望单！", "iOS · Safari", "Nori", "starred"),
                ("手机 Safari 全屏触摸进入非常丝滑，无卡顿", "Android · Chrome", "小满", "seen"),
            ]
        },
        {
            "slug": "rainy-store",
            "title": "雨夜便利店",
            "summary": "一间只在下雨时营业的街角便利店，倾听来往客人的心事与故事。",
            "note": "背景环境白噪音与收音机电台交互测试",
            "kind": "web",
            "engine": "phaser",
            "seats": 5,
            "joined": 3,
            "seeking": True,
            "public": True,
            "boosted": False,
            "players": 16,
            "followers": 9,
            "cover": {"hash": cover_rain_hash, "size": len(cover_rain_bytes), "mime": "image/png"},
            "version": 1,
            "files": [{"path": "index.html", "hash": phaser_hash, "size": len(phaser_html)}],
            "feedbacks": [
                ("关门风铃的声音清脆，便利店货架交互很自然", "macOS · Safari", "鹿白", "seen"),
                ("期待加入更多下雨天的随机客人对话！", "Windows · Edge", "木川", "new"),
            ]
        },
        {
            "slug": "paper-plane",
            "title": "纸飞机",
            "summary": "用指尖滑动的风，把折叠的纸飞机送到最远的海边。",
            "note": "调整了触控灵敏度与迎风阻力模型，欢迎尝试大回旋动作",
            "kind": "web",
            "engine": "canvas",
            "seats": None,
            "joined": 0,
            "seeking": False,
            "public": True,
            "boosted": True, # 广场头牌推荐
            "players": 56,
            "followers": 23,
            "cover": None, # 体验 4:3 自动几何星轨画框！
            "version": 1,
            "files": [{"path": "index.html", "hash": vite_hash, "size": len(vite_html)}],
            "feedbacks": [
                ("4:3 卡片撕票线太有实体质感了！按键对比度非常舒服", "macOS · Chrome", "Leo", "starred"),
                ("风向变化时的气流粒子做得很有代入感", "iOS · Safari", "小树", "seen"),
                ("在 iPad 上用 Apple Pencil 划风体验绝了", "iPadOS · Safari", "云端客", "done"),
                ("滑行超过 500 米后背景音乐升调很惊艳", "Windows · Chrome", "Tester_01", "new"),
            ]
        },
        {
            "slug": "ivory-beaver-33",
            "title": "ODD FOLK — 原创头像工作室",
            "summary": "几百种手绘几何特征，拼出一张独一无二的像素头像。",
            "note": "新增冷萃绿调色板与高分辨率 SVG 矢量导出，看看导出按钮是否顺手",
            "kind": "web",
            "engine": "vanilla",
            "seats": None,
            "joined": 0,
            "seeking": False,
            "public": True,
            "boosted": False,
            "players": 42,
            "followers": 18,
            "cover": {"hash": cover_avatar_hash, "size": len(cover_avatar_bytes), "mime": "image/png"},
            "version": 1,
            "files": [{"path": "index.html", "hash": odd_folk_hash, "size": len(odd_folk_html)}],
            "feedbacks": [
                ("太可爱了！随机到的戴小蓝帽狐狸直接拿来当头像了", "macOS · Chrome", "七月", "done"),
                ("希望能支持自定义十六进制颜色代码", "Linux · Firefox", "CoderX", "new"),
            ]
        },
        {
            "slug": "design-notes",
            "title": "黑曜石与冷萃绿：Playtest 视觉设计手记",
            "summary": "关于 4:3 黄金凭证画框、物理撕票虚线与纯钛白高对比主按键的质感重构实践。",
            "note": "分享设计系统从 20:9 扁平压抑到实体收藏卡（Playtest Pass）的演进过程",
            "kind": "article",
            "engine": None,
            "seats": None,
            "joined": 0,
            "seeking": False,
            "public": True,
            "boosted": False,
            "players": 31,
            "followers": 12,
            "cover": None,
            "version": 1,
            "article": {
                "hash": article_artifact_hash,
                "size": len(article_html.encode("utf-8")),
            },
            "files": [{"path": "index.md", "hash": article_artifact_hash, "size": len(article_html.encode("utf-8"))}],
            "feedbacks": [
                ("文章排版阅读体验极度舒适，字重和行距控制得非常克制严谨", "macOS · Safari", "设计观察员", "starred"),
                ("钛白高对比按钮确实彻底解决了暗色模式下的置灰禁用错觉！", "Windows · Edge", "前端小王", "done"),
            ]
        },
        {
            "slug": "tiny-orbit",
            "title": "Tiny Orbit",
            "summary": "用一根手指，把微型人造卫星留在引力轨道上。",
            "note": "测试引力弹弓与近地轨道减速手感",
            "kind": "web",
            "engine": "godot",
            "seats": 8,
            "joined": 5,
            "seeking": True,
            "public": True,
            "boosted": False,
            "players": 24,
            "followers": 7,
            "cover": None,
            "version": 1,
            "files": [{"path": "index.html", "hash": tiny_orbit_hash, "size": len(tiny_orbit_html)}],
            "feedbacks": [
                ("按压加速的手感很有魔性，不知不觉玩了十几分钟", "iOS · Safari", "星际流浪者", "new"),
                ("近地轨道的吸力稍微有点强，新手容易撞星", "Android · Chrome", "航天爱好者", "seen"),
            ]
        },
        {
            "slug": "deep-sea-echo",
            "title": "深海回响：实机演示片段",
            "summary": "水下声纳与弱光环境渲染实机 Demo 演示片段，感受低频声纳脉冲。",
            "note": "建议佩戴耳机全屏观看，重点倾听低频声纳回响",
            "kind": "video",
            "engine": None,
            "seats": None,
            "joined": 0,
            "seeking": False,
            "public": True,
            "boosted": False,
            "players": 19,
            "followers": 6,
            "cover": None,
            "version": 1,
            "entry": "sonar.mp4",
            "files": [{"path": "sonar.mp4", "hash": video_hash, "size": len(fake_video_bytes)}],
            "feedbacks": [
                ("声纳水下扩散的音效做得很真实，期待正式完整试玩版发布", "macOS · Chrome", "音频发烧友", "new")
            ]
        },
        {
            "slug": "deep-space-beacon",
            "title": "深空信标",
            "summary": "在柯伊伯带边缘的第 07 号深空信标站，倾听来自奥尔特云彼端的神秘脉冲。",
            "note": "关于节奏与科幻设定的取舍，哪一段还没说清楚？欢迎在各章末尾留言探讨。",
            "kind": "article",
            "engine": None,
            "seats": None,
            "joined": 0,
            "seeking": False,
            "public": True,
            "boosted": False,
            "players": 35,
            "followers": 19,
            "cover": None,
            "version": 1,
            "chapters": [
                {"id": "c1", "title": "第一章：沉睡的三百年", "path": "01-sleep.md", "hash": ch1_hash, "size": len(ch1_bytes)},
                {"id": "c2", "title": "第二章：奥尔特云的谐波", "path": "02-harmonics.md", "hash": ch2_hash, "size": len(ch2_bytes)},
                {"id": "c3", "title": "第三章：未知的引力回声", "path": "03-gravity-echo.md", "hash": ch3_hash, "size": len(ch3_bytes)},
            ],
            "article": {
                "hash": ch1_hash,
                "size": len(ch1_bytes),
            },
            "entry": "01-sleep.md",
            "files": [
                {"path": "01-sleep.md", "hash": ch1_hash, "size": len(ch1_bytes)},
                {"path": "02-harmonics.md", "hash": ch2_hash, "size": len(ch2_bytes)},
                {"path": "03-gravity-echo.md", "hash": ch3_hash, "size": len(ch3_bytes)},
            ],
            "feedbacks": [
                ("[c1] 低温休眠舱被唤醒的气氛渲染得太棒了，有太空史诗感！", "macOS · Safari", "科幻迷老陈", "starred"),
                ("[c2] 1420MHz中性氢翻转相位的设定很有考究，硬科幻味正", "Windows · Edge", "天文爱好者", "done"),
                ("[c3] 第三章结尾断章太狠了！什么时候更第四章？求追更！", "iOS · Safari", "星海游民", "new"),
            ]
        }
    ]

    plaza_items = []

    for idx, p in enumerate(projects):
        slug = p["slug"]
        site_dir = os.path.join(SITES_DIR, slug)
        manifests_dir = os.path.join(site_dir, "manifests")
        os.makedirs(manifests_dir, exist_ok=True)

        # 写入数据库 sites 表
        cur.execute("DELETE FROM sites WHERE slug = ?", (slug,))
        cur.execute("""
            INSERT INTO sites (
                slug, user_id, title, created_at, expires_at, current_version,
                public, seeking, seek_note, summary,
                cover_hash, cover_mime, engine, updated_at, seats,
                community_url, feedback_public, work_kind
            ) VALUES (?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?)
        """, (
            slug, user_id, p["title"], now_iso(-72 + idx * 6), p["version"],
            1 if p["public"] else 0,
            1 if p["seeking"] else 0,
            p["note"], p["summary"],
            p["cover"]["hash"] if p["cover"] else None,
            p["cover"]["mime"] if p["cover"] else None,
            p["engine"],
            now_iso(-idx * 3),
            p["seats"],
            "https://playtest.roviix.com" if idx % 2 == 0 else None,
            p["kind"]
        ))

        # 写入 versions 表
        cur.execute("DELETE FROM versions WHERE slug = ?", (slug,))
        for v in range(1, p["version"] + 1):
            cur.execute("""
                INSERT INTO versions (slug, version, created_at, note, file_count, total_bytes)
                VALUES (?, ?, ?, ?, ?, ?)
            """, (
                slug, v, now_iso(-72 + v * 12),
                p["note"] if v == p["version"] else "初始发布版本",
                len(p["files"]),
                sum(f["size"] for f in p["files"])
            ))

        # 写入 manifests/{v}.json
        manifest_obj = {
            "schema": 1,
            "slug": slug,
            "version": p["version"],
            "title": p["title"],
            "developer": "Zhongshang Wu",
            "summary": p["summary"],
            "note": p["note"],
            "cover": p["cover"],
            "created_at": now_iso(-72),
            "badge": False,
            "gate": "once",
            "isolated": False,
            "spa": False,
            "engine": p["engine"],
            "kind": p["kind"],
            "files": p["files"]
        }
        if p.get("article"):
            manifest_obj["article"] = p["article"]
            manifest_obj["entry"] = "index.md"
        if p.get("chapters"):
            manifest_obj["chapters"] = p["chapters"]
        if p.get("entry"):
            manifest_obj["entry"] = p["entry"]

        with open(os.path.join(manifests_dir, f"{p['version']}.json"), "w", encoding="utf-8") as mf:
            json.dump(manifest_obj, mf, ensure_ascii=False, indent=2)

        # 写入 current.json
        current_obj = {
            "version": p["version"],
            "updated_at": now_iso(-idx * 3)
        }
        with open(os.path.join(site_dir, "current.json"), "w", encoding="utf-8") as cf:
            json.dump(current_obj, cf, ensure_ascii=False, indent=2)

        # 写入 live.json
        public_feedback_list = []
        for text, dev, author, status in p["feedbacks"]:
            public_feedback_list.append({
                "name": author,
                "text": text,
                "version": p["version"],
                "at": now_iso(-idx * 4 - 2),
            })

        live_obj = {
            "schema": 1,
            "slug": slug,
            "generated_at": now_iso(),
            "seats": p["seats"],
            "joined": p["joined"],
            "followers": p["followers"],
            "feedback_public": True,
            "public_feedback": public_feedback_list[:3],
            "avatar_url": "https://avatars.githubusercontent.com/u/1024025?v=4",
            "listed": p["public"],
            "seeking": p["seeking"]
        }
        with open(os.path.join(site_dir, "live.json"), "w", encoding="utf-8") as lf:
            json.dump(live_obj, lf, ensure_ascii=False, indent=2)

        # 写入 sessions (点名册数据)
        cur.execute("DELETE FROM sessions WHERE slug = ?", (slug,))
        cur.execute("DELETE FROM session_events WHERE slug = ?", (slug,))
        devices_pool = [
            ("macOS · Chrome 128", "desktop", "chrome", "macos", "阿遥"),
            ("iOS 18 · Safari", "phone", "safari", "ios", "Nori"),
            ("Windows 11 · Edge", "desktop", "chrome", "windows", "老木"),
            ("Android 15 · Chrome", "phone", "chrome", "android", "小满"),
            ("iPadOS 18 · Safari", "tablet", "safari", "ios", "鹿白"),
            ("macOS · Safari 18", "desktop", "safari", "macos", "七月"),
            ("Windows 10 · Firefox", "desktop", "firefox", "windows", "Tester_Neo"),
        ]
        for s_idx in range(min(p["players"], 6)):
            d_ua, d_dev, d_browser, d_os, d_name = devices_pool[s_idx % len(devices_pool)]
            sess_id = f"sess-{slug}-{s_idx+1:03d}"
            t_first = now_iso(-s_idx * 5 - 1)
            t_last = now_iso(-s_idx * 5 + 1)
            cur.execute("""
                INSERT INTO sessions (
                    id, slug, version, first_seen_at, last_seen_at, ua, device,
                    browser, os, referrer_kind, wechat, gate_view_at, start_at,
                    load_ms, is_return, name
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'direct', 0, ?, ?, ?, ?, ?)
            """, (
                sess_id, slug, p["version"], t_first, t_last, d_ua, d_dev,
                d_browser, d_os, t_first, t_first, 420 + s_idx * 45,
                1 if s_idx % 2 == 1 else 0,
                d_name
            ))

        # 写入 feedback 表
        cur.execute("DELETE FROM feedback WHERE slug = ?", (slug,))
        for f_idx, (f_text, f_dev, f_author, f_status) in enumerate(p["feedbacks"]):
            cur.execute("""
                INSERT INTO feedback (
                    session_id, slug, version, ts, text, seconds_in,
                    device, browser, status, hidden
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0)
            """, (
                f"sess-{slug}-{f_idx+1:03d}", slug, p["version"],
                now_iso(-f_idx * 6 - 3), f_text, 120 + f_idx * 80,
                f_dev, "chrome" if "Chrome" in f_dev else "safari",
                "done" if f_status == "starred" else f_status
            ))

        # 组装 plaza item
        is_game = p["engine"] is not None or p["kind"] == "web" and "单车" in p["title"] or "跳" in p["title"]
        plaza_items.append({
            "slug": slug,
            "url": f"http://{slug}.localhost:8443",
            "title": p["title"],
            "developer": "Zhongshang Wu",
            "avatar_url": "https://avatars.githubusercontent.com/u/1024025?v=4",
            "summary": p["summary"],
            "note": p["note"],
            "engine": p["engine"],
            "kind": p["kind"],
            "is_game": is_game,
            "cover_hash": p["cover"]["hash"] if p["cover"] else None,
            "version": p["version"],
            "updated_at": now_iso(-idx * 3),
            "players": p["players"],
            "seeking": p["seeking"],
            "seats": p["seats"],
            "joined": p["joined"],
            "followers": p["followers"],
            "boosted": p["boosted"]
        })

    # 3. 写入 Boosts (推广位)
    cur.execute("DELETE FROM boosts WHERE slug = 'paper-plane'")
    cur.execute("""
        INSERT INTO boosts (
            slug, kind, status, granted, starts_at, ends_at, created_at, reason
        ) VALUES ('paper-plane', 'days3', 'live', 1, ?, ?, ?, '金秋精选热门作品')
    """, (now_iso(-12), now_iso(60), now_iso(-12)))

    # 4. 写入 Collections (合集与挑战)
    cur.execute("DELETE FROM collections WHERE slug IN ('autumn-showcase-2026', 'microgame-72h')")
    cur.execute("DELETE FROM collection_entries WHERE collection_slug IN ('autumn-showcase-2026', 'microgame-72h')")

    cur.execute("""
        INSERT INTO collections (
            slug, user_id, title, summary, kind, prompt, rules, closes_at,
            public, hidden, created_at, updated_at
        ) VALUES (
            'autumn-showcase-2026', ?, '2026 金秋独立作品试玩展',
            '精选本季由独立开发者带来的原创试玩作品，感受细腻的视听与交互设计。',
            'collection', '', '', NULL, 1, 0, ?, ?
        )
    """, (user_id, now_iso(-48), now_iso(-12)))

    for c_slug in ["lucky-robin-21", "rainy-store", "paper-plane", "ivory-beaver-33", "deep-space-beacon"]:
        cur.execute("""
            INSERT INTO collection_entries (
                collection_slug, site_slug, user_id, submitted_version, submitted_at,
                note, blocked
            ) VALUES ('autumn-showcase-2026', ?, ?, 1, ?, '', 0)
        """, (c_slug, user_id, now_iso(-24)))

    cur.execute("""
        INSERT INTO collections (
            slug, user_id, title, summary, kind, prompt, rules, closes_at,
            public, hidden, created_at, updated_at
        ) VALUES (
            'microgame-72h', ?, '72小时极简微型游戏极限挑战',
            '只用一根手指或一个按键，做出让人停不下来的微型游戏。',
            'challenge', '主题：连锁反应与引力',
            '1. 单指触控操作；2. 文件体积 < 5MB；3. 首次加载耗时 < 1s。',
            ?, 1, 0, ?, ?
        )
    """, (user_id, now_iso(120), now_iso(-36), now_iso(-6)))

    for c_slug in ["paper-plane", "tiny-orbit"]:
        cur.execute("""
            INSERT INTO collection_entries (
                collection_slug, site_slug, user_id, submitted_version, submitted_at,
                note, blocked
            ) VALUES ('microgame-72h', ?, ?, 1, ?, '一键微型游戏物理引擎与手感调优', 0)
        """, (c_slug, user_id, now_iso(-18)))

    # 5. 写入 Player 与关注数据 (/me 订阅中心)
    player_id = "demo-player-2026-uuid"
    player_email = "demo@playtest.run"
    me_token_plain = "pt_me_secret_token_2026"
    me_token_hash = sha256(me_token_plain.encode("utf-8"))
    confirm_token_plain = "confirm-token-auto-login-2026"
    confirm_token_hash = sha256(confirm_token_plain.encode("utf-8"))

    cur.execute("DELETE FROM players WHERE id = ? OR email = ?", (player_id, player_email))
    cur.execute("""
        INSERT INTO players (
            id, email, email_verified_at, me_token_hash, created_at
        ) VALUES (?, ?, ?, ?, ?)
    """, (player_id, player_email, now_iso(-72), me_token_hash, now_iso(-72)))

    cur.execute("DELETE FROM follows WHERE player_id = ?", (player_id,))
    # 关注广场周报
    cur.execute("""
        INSERT INTO follows (player_id, target_kind, target_slug, source, created_at)
        VALUES (?, 'plaza', NULL, 'plaza', ?)
    """, (player_id, now_iso(-70)))
    # 关注作品
    for target in ["lucky-robin-21", "rainy-store", "ivory-beaver-33", "deep-space-beacon"]:
        cur.execute("""
            INSERT INTO follows (player_id, target_kind, target_slug, source, created_at)
            VALUES (?, 'site', ?, 'gate', ?)
        """, (player_id, target, now_iso(-50)))

    # 插入确认令牌 (方便在浏览器通过 GET 直接激活会话 cookie)
    cur.execute("DELETE FROM follow_tokens WHERE player_id = ?", (player_id,))
    cur.execute("""
        INSERT INTO follow_tokens (
            token_hash, player_id, purpose, target_kind, target_slug, source, created_at, expires_at
        ) VALUES (?, ?, 'confirm', 'plaza', NULL, 'me', ?, ?)
    """, (confirm_token_hash, player_id, now_iso(), now_iso(240)))

    conn.commit()
    conn.close()

    # 6. 生成并写出 store/plaza.json
    plaza_data = {
        "schema": 1,
        "generated_at": now_iso(),
        "club_followers": 38,
        "items": plaza_items
    }
    with open(os.path.join(STORE_DIR, "plaza.json"), "w", encoding="utf-8") as f:
        json.dump(plaza_data, f, ensure_ascii=False, indent=2)

    # 7. 更新本地 ~/.config/playtest/config.json 映射
    if os.path.exists(CLI_CONFIG_PATH):
        try:
            with open(CLI_CONFIG_PATH, "r", encoding="utf-8") as f:
                cfg = json.load(f)
            cfg["token"] = token
            cfg["token_expires_at"] = None
            if "sites" not in cfg:
                cfg["sites"] = {}
            cfg["sites"][os.path.join(ROOT, "fixtures/pelican-bicycle/export")] = "lucky-robin-21"
            cfg["sites"][os.path.join(ROOT, "fixtures/phaser-jump/export")] = "rainy-store"
            cfg["sites"][os.path.join(ROOT, "fixtures/vite-vanilla/export")] = "paper-plane"
            with open(CLI_CONFIG_PATH, "w", encoding="utf-8") as f:
                json.dump(cfg, f, ensure_ascii=False, indent=2)
        except Exception as e:
            print(f"⚠️ 更新 CLI 配置文件警告: {e}")

    print("✨ 本地全套数据充斥完成！")
    print(f"   · 开发者：Zhongshang Wu (CLI Token 永久绑定)")
    print(f"   · 作品总数：{len(projects)} 个（含游戏、Web 工具、文章、实机视频、微型游戏）")
    print(f"   · 合集与挑战：2 组（2026金秋独立作品展、72h微型游戏挑战）")
    print(f"   · 玩家关注：已预置 demo@playtest.run 关注广场及 3 个作品")
    print(f"   · 玩家登录确认链接：http://localhost:8443/me/confirm/{confirm_token_plain}")

if __name__ == "__main__":
    main()
