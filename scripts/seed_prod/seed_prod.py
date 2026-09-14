#!/usr/bin/env python3
"""
seed_prod.py - 生产环境 (playtest.run) 演示数据注入脚本

执行逻辑：
1. 使用 boto3 将作品 Blob、清单 Manifest、current.json、live.json、plaza.json 写入 S3 存储桶 (playtest-assets-306903156946-hk)
2. 连接本地 SQLite (/home/ubuntu/playtest-data/api.sqlite)，写入用户、作品元数据、版本、点名册访问记录、玩家反馈、合集与挑战、推广位
"""

import hashlib
import json
import os
import sqlite3
import sys
from datetime import datetime, timezone, timedelta

try:
    import boto3
except ImportError:
    print("错误: 请先安装 boto3 (pip3 install boto3)")
    sys.exit(1)

DIR = os.path.dirname(os.path.abspath(__file__))
ASSETS_DIR = os.path.join(DIR, "assets")
DB_PATH = "/home/ubuntu/playtest-data/api.sqlite"

S3_BUCKET = "playtest-assets-306903156946-hk"
S3_REGION = "ap-east-1"
S3_AK = os.environ.get("PLAYTEST_S3_ACCESS_KEY_ID", os.environ.get("AWS_ACCESS_KEY_ID"))
S3_SK = os.environ.get("PLAYTEST_S3_SECRET_ACCESS_KEY", os.environ.get("AWS_SECRET_ACCESS_KEY"))

s3 = boto3.client(
    "s3",
    region_name=S3_REGION,
    aws_access_key_id=S3_AK,
    aws_secret_access_key=S3_SK
)

def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def now_iso(offset_hours: float = 0) -> str:
    dt = datetime.now(timezone.utc) + timedelta(hours=offset_hours)
    return dt.strftime("%Y-%m-%dT%H:%M:%SZ")

def put_s3_blob(data: bytes, content_type: str = "application/octet-stream") -> str:
    h = sha256(data)
    key = f"blobs/{h[:2]}/{h}"
    s3.put_object(
        Bucket=S3_BUCKET,
        Key=key,
        Body=data,
        ContentType=content_type
    )
    return h

def put_s3_json(key: str, data: dict):
    body = json.dumps(data, ensure_ascii=False, indent=2).encode("utf-8")
    s3.put_object(
        Bucket=S3_BUCKET,
        Key=key,
        Body=body,
        ContentType="application/json; charset=utf-8"
    )

def main():
    print(f"🚀 开始向生产环境 (S3: {S3_BUCKET}, DB: {DB_PATH}) 灌入测试数据...")

    # 1. 准备文件与 Blobs
    with open(os.path.join(ASSETS_DIR, "pelican.html"), "rb") as f:
        pelican_html = f.read()
    pelican_hash = put_s3_blob(pelican_html, "text/html; charset=utf-8")

    with open(os.path.join(ASSETS_DIR, "rainy_store.html"), "rb") as f:
        rainy_store_html = f.read()
    rainy_store_hash = put_s3_blob(rainy_store_html, "text/html; charset=utf-8")

    with open(os.path.join(ASSETS_DIR, "paper_plane.html"), "rb") as f:
        paper_plane_html = f.read()
    paper_plane_hash = put_s3_blob(paper_plane_html, "text/html; charset=utf-8")

    with open(os.path.join(ASSETS_DIR, "cover_bike.png"), "rb") as f:
        cover_bike_bytes = f.read()
    cover_bike_hash = put_s3_blob(cover_bike_bytes, "image/png")

    with open(os.path.join(ASSETS_DIR, "cover_rain.png"), "rb") as f:
        cover_rain_bytes = f.read()
    cover_rain_hash = put_s3_blob(cover_rain_bytes, "image/png")

    with open(os.path.join(ASSETS_DIR, "cover_avatar.png"), "rb") as f:
        cover_avatar_bytes = f.read()
    cover_avatar_hash = put_s3_blob(cover_avatar_bytes, "image/png")

    # 像素头像工坊 HTML
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
    odd_folk_hash = put_s3_blob(odd_folk_html, "text/html; charset=utf-8")

    # 微型轨道小游戏 HTML
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
    tiny_orbit_hash = put_s3_blob(tiny_orbit_html, "text/html; charset=utf-8")

    # 设计手记 HTML
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
    article_hash = put_s3_blob(article_html.encode("utf-8"), "text/html; charset=utf-8")

    # 科幻小说 3 章节
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
    ch1_hash = put_s3_blob(ch1_text.encode("utf-8"), "text/html; charset=utf-8")

    ch2_text = """<h2>第二章：奥尔特云的谐波</h2>
<p>分析仪的示波器屏幕上，那道引力波回声展现出令人不安的对称性。</p>
<p>自然天体的引力扰动往往伴随着杂乱的高频毛刺，而面前这条波形却平滑得像用高阶多项式拟合出来的正弦曲线。</p>
<p>“这不是彗星分裂，也不是小行星摄动。”陆巡咬碎了一粒合成咖啡胶囊，苦涩的提神剂让他的思维迅速冷凝，“它的频率是 1420.405 MHz——中性氢的辐射线，但它上面加载了三重相位翻转。”</p>
<p>那是人类在二十世纪中叶向深空发出的第一个坐标握手协议。</p>
<p>有人在三百年后，把人类自己扔向虚空的火把，原封不动地掷了回来。或者说……有人在用人类的母语，向整个奥尔特云广播一个求救代码。</p>
<p>陆巡启动了信标站的高增益抛物面天线。巨大的冷凝器开始加压，金属骨架在极端温差下发出轻微的呻吟。如果这一束确认脉冲发射出去，信标站的储能将直接跌入临界阈值，但如果保持沉默，这个飞掠而过的谜题将在三十六小时后彻底坠入未知的星际荒原。</p>"""
    ch2_hash = put_s3_blob(ch2_text.encode("utf-8"), "text/html; charset=utf-8")

    ch3_text = """<h2>第三章：未知的引力回声</h2>
<p>天线充能进度：98%……99%……发射。</p>
<p>一道肉眼无法看见的高相干微波束刺破了柯伊伯带的万古死寂。微弱的蓝光在离子推进喷口边缘一闪而过。</p>
<p>随后是长达七分钟的等待。在光速都要行走漫长尺度的深空里，沉默就是最漫长的凌迟。</p>
<p>七分十四秒。</p>
<p>信标站的舱壁突然剧烈地震颤起来。这不是无线电波的接收，而是整个局部空间的微度扭曲——引力透镜效应在距离信标站仅三十公里的真空中撕开了一条肉眼可见的微弱亮线。</p>
<p>暗区中，一艘通体覆盖着黑色吸光晶体的梭形构装体滑出了虚空。它的表面没有任何焊接缝隙，只有一道由浅蓝色流光构成的信标指示灯，正以一秒一次的频率，与陆巡胸口的心跳监视器发生着完全一致的同频共振。</p>
<p>主控屏上的自检日志跳出了最终解密行：</p>
<pre><code>WELCOME BACK, EXPLORER.</code></pre>"""
    ch3_hash = put_s3_blob(ch3_text.encode("utf-8"), "text/html; charset=utf-8")

    # 2. 数据库连接与用户准备
    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()

    users = [
        {
            "id": "usr-zhongshang",
            "name": "Zhongshang Wu",
            "login": "zhongshangwu",
            "avatar": None,
            "github_id": 1024025
        },
        {
            "id": "usr-ayao",
            "name": "阿遥",
            "login": "ayao_studio",
            "avatar": None,
            "github_id": 1024026
        },
        {
            "id": "usr-noriko",
            "name": "Noriko",
            "login": "noriko_pixel",
            "avatar": None,
            "github_id": 1024027
        },
        {
            "id": "usr-star",
            "name": "星轨工坊",
            "login": "star_orbit",
            "avatar": None,
            "github_id": 1024028
        }
    ]

    for u in users:
        cur.execute("DELETE FROM users WHERE id = ?", (u["id"],))
        cur.execute("""
            INSERT INTO users (id, kind, display_name, created_at, expires_at, github_id, login, avatar_url)
            VALUES (?, 'github', ?, ?, NULL, ?, ?, ?)
        """, (u["id"], u["name"], now_iso(-240), u["github_id"], u["login"], u["avatar"]))

    # 3. 7 个各具特色的作品定义
    projects = [
        {
            "slug": "lucky-robin-21",
            "user_id": "usr-zhongshang",
            "developer": "Zhongshang Wu",
            "avatar": None,
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
            "players": 32,
            "followers": 16,
            "cover": {"hash": cover_bike_hash, "size": len(cover_bike_bytes), "mime": "image/png"},
            "version": 2,
            "files": [{"path": "index.html", "hash": pelican_hash, "size": len(pelican_html)}],
            "feedbacks": [
                ("过弯物理惯性手感很扎实！希望能加个手刹键漂移", "macOS · Chrome", "阿遥", "done"),
                ("音乐和雨夜海港画面太治愈了，已加入愿望单！", "iOS · Safari", "Noriko", "starred"),
                ("手机 Safari 全屏触摸进入非常丝滑，无卡顿", "Android · Chrome", "小满", "seen"),
            ]
        },
        {
            "slug": "rainy-store",
            "user_id": "usr-noriko",
            "developer": "Noriko",
            "avatar": None,
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
            "players": 18,
            "followers": 11,
            "cover": {"hash": cover_rain_hash, "size": len(cover_rain_bytes), "mime": "image/png"},
            "version": 1,
            "files": [{"path": "index.html", "hash": rainy_store_hash, "size": len(rainy_store_html)}],
            "feedbacks": [
                ("关门风铃的声音清脆，便利店货架交互很自然", "macOS · Safari", "鹿白", "seen"),
                ("期待加入更多下雨天的随机客人对话！", "Windows · Edge", "木川", "new"),
            ]
        },
        {
            "slug": "paper-plane",
            "user_id": "usr-star",
            "developer": "星轨工坊",
            "avatar": None,
            "title": "纸飞机",
            "summary": "用指尖滑动的风，把折叠的纸飞机送到最远的海边。",
            "note": "调整了触控灵敏度与迎风阻力模型，欢迎尝试大回旋动作",
            "kind": "web",
            "engine": "canvas",
            "seats": None,
            "joined": 0,
            "seeking": False,
            "public": True,
            "boosted": True, # 推广位置顶
            "players": 68,
            "followers": 29,
            "cover": None, # 纯粹展示 4:3 几何星轨画框
            "version": 1,
            "files": [{"path": "index.html", "hash": paper_plane_hash, "size": len(paper_plane_html)}],
            "feedbacks": [
                ("4:3 卡片撕票线太有实体质感了！按键对比度非常舒服", "macOS · Chrome", "Leo", "starred"),
                ("风向变化时的气流粒子做得很有代入感", "iOS · Safari", "小树", "seen"),
                ("在 iPad 上用 Apple Pencil 划风体验绝了", "iPadOS · Safari", "云端客", "done"),
            ]
        },
        {
            "slug": "ivory-beaver-33",
            "user_id": "usr-ayao",
            "developer": "阿遥",
            "avatar": None,
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
            "players": 45,
            "followers": 22,
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
            "user_id": "usr-zhongshang",
            "developer": "Zhongshang Wu",
            "avatar": None,
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
            "players": 38,
            "followers": 15,
            "cover": None,
            "version": 1,
            "article": {
                "hash": article_hash,
                "size": len(article_html.encode("utf-8")),
            },
            "files": [{"path": "index.md", "hash": article_hash, "size": len(article_html.encode("utf-8"))}],
            "feedbacks": [
                ("文章排版阅读体验极度舒适，字重和行距控制得非常克制严谨", "macOS · Safari", "设计观察员", "starred"),
                ("钛白高对比按钮确实彻底解决了暗色模式下的置灰禁用错觉！", "Windows · Edge", "前端小王", "done"),
            ]
        },
        {
            "slug": "tiny-orbit",
            "user_id": "usr-star",
            "developer": "星轨工坊",
            "avatar": None,
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
            "players": 27,
            "followers": 9,
            "cover": None,
            "version": 1,
            "files": [{"path": "index.html", "hash": tiny_orbit_hash, "size": len(tiny_orbit_html)}],
            "feedbacks": [
                ("按压加速的手感很有魔性，不知不觉玩了十几分钟", "iOS · Safari", "星际流浪者", "new"),
                ("近地轨道的吸力稍微有点强，新手容易撞星", "Android · Chrome", "航天爱好者", "seen"),
            ]
        },
        {
            "slug": "deep-space-beacon",
            "user_id": "usr-star",
            "developer": "星轨工坊",
            "avatar": None,
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
            "players": 41,
            "followers": 23,
            "cover": None,
            "version": 1,
            "chapters": [
                {"id": "c1", "title": "第一章：沉睡的三百年", "path": "01-sleep.md", "hash": ch1_hash, "size": len(ch1_text.encode('utf-8'))},
                {"id": "c2", "title": "第二章：奥尔特云的谐波", "path": "02-harmonics.md", "hash": ch2_hash, "size": len(ch2_text.encode('utf-8'))},
                {"id": "c3", "title": "第三章：未知的引力回声", "path": "03-gravity-echo.md", "hash": ch3_hash, "size": len(ch3_text.encode('utf-8'))},
            ],
            "article": {
                "hash": ch1_hash,
                "size": len(ch1_text.encode("utf-8")),
            },
            "entry": "01-sleep.md",
            "files": [
                {"path": "01-sleep.md", "hash": ch1_hash, "size": len(ch1_text.encode('utf-8'))},
                {"path": "02-harmonics.md", "hash": ch2_hash, "size": len(ch2_text.encode('utf-8'))},
                {"path": "03-gravity-echo.md", "hash": ch3_hash, "size": len(ch3_text.encode('utf-8'))},
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
        print(f"  📦 处理作品: {slug} ({p['title']})")

        # 写入 blobs 表 (避免 API 误判 blob 不存在)
        for f in p["files"]:
            cur.execute("INSERT OR REPLACE INTO blobs (hash, size, created_at) VALUES (?, ?, ?)",
                        (f["hash"], f["size"], now_iso(-100)))
        if p.get("cover"):
            cur.execute("INSERT OR REPLACE INTO blobs (hash, size, created_at) VALUES (?, ?, ?)",
                        (p["cover"]["hash"], p["cover"]["size"], now_iso(-100)))

        # 写入 sites 表
        cur.execute("DELETE FROM sites WHERE slug = ?", (slug,))
        cur.execute("""
            INSERT INTO sites (
                slug, user_id, title, created_at, expires_at, current_version,
                public, seeking, seek_note, summary,
                cover_hash, cover_mime, engine, updated_at, seats,
                community_url, feedback_public, work_kind
            ) VALUES (?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?)
        """, (
            slug, p["user_id"], p["title"], now_iso(-120 + idx * 10), p["version"],
            1 if p["public"] else 0,
            1 if p["seeking"] else 0,
            p["note"], p["summary"],
            p["cover"]["hash"] if p["cover"] else None,
            p["cover"]["mime"] if p["cover"] else None,
            p["engine"],
            now_iso(-idx * 4 - 1),
            p["seats"],
            "https://playtest.run" if idx % 2 == 0 else None,
            p["kind"]
        ))

        # 写入 versions 表
        cur.execute("DELETE FROM versions WHERE slug = ?", (slug,))
        for v in range(1, p["version"] + 1):
            cur.execute("""
                INSERT INTO versions (slug, version, created_at, note, file_count, total_bytes)
                VALUES (?, ?, ?, ?, ?, ?)
            """, (
                slug, v, now_iso(-120 + v * 20),
                p["note"] if v == p["version"] else "初始测试版本",
                len(p["files"]),
                sum(f["size"] for f in p["files"])
            ))

        # 写入 sessions 表
        cur.execute("DELETE FROM sessions WHERE slug = ?", (slug,))
        cur.execute("DELETE FROM session_events WHERE slug = ?", (slug,))
        devices_pool = [
            ("macOS · Chrome 128", "desktop", "chrome", "macos", "阿遥"),
            ("iOS 18 · Safari", "phone", "safari", "ios", "Noriko"),
            ("Windows 11 · Edge", "desktop", "chrome", "windows", "老木"),
            ("Android 15 · Chrome", "phone", "chrome", "android", "小满"),
            ("iPadOS 18 · Safari", "tablet", "safari", "ios", "鹿白"),
            ("macOS · Safari 18", "desktop", "safari", "macos", "七月"),
        ]
        for s_idx in range(min(p["players"], 6)):
            d_ua, d_dev, d_browser, d_os, d_name = devices_pool[s_idx % len(devices_pool)]
            sess_id = f"sess-{slug}-{s_idx+1:03d}"
            t_first = now_iso(-s_idx * 5 - 1)
            cur.execute("""
                INSERT INTO sessions (
                    id, slug, version, first_seen_at, last_seen_at, ua, device,
                    browser, os, referrer_kind, wechat, gate_view_at, start_at,
                    load_ms, is_return, name
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'direct', 0, ?, ?, ?, ?, ?)
            """, (
                sess_id, slug, p["version"], t_first, t_first, d_ua, d_dev,
                d_browser, d_os, t_first, t_first, 420 + s_idx * 50,
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
                now_iso(-f_idx * 8 - 4), f_text, 120 + f_idx * 90,
                f_dev, "chrome" if "Chrome" in f_dev else "safari",
                "done" if f_status == "starred" else f_status
            ))

        # 写入 S3: manifests/{version}.json
        manifest_obj = {
            "schema": 1,
            "slug": slug,
            "version": p["version"],
            "title": p["title"],
            "developer": p["developer"],
            "summary": p["summary"],
            "note": p["note"],
            "cover": p["cover"],
            "created_at": now_iso(-120),
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

        put_s3_json(f"sites/{slug}/manifests/{p['version']}.json", manifest_obj)

        # 写入 S3: current.json
        current_obj = {
            "version": p["version"],
            "updated_at": now_iso(-idx * 4 - 1)
        }
        put_s3_json(f"sites/{slug}/current.json", current_obj)

        # 写入 S3: live.json
        public_feedback_list = []
        for text, dev, author, status in p["feedbacks"]:
            public_feedback_list.append({
                "name": author,
                "text": text,
                "version": p["version"],
                "at": now_iso(-idx * 5 - 3),
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
            "avatar_url": p["avatar"],
            "listed": p["public"],
            "seeking": p["seeking"]
        }
        put_s3_json(f"sites/{slug}/live.json", live_obj)

        # 广场 item 数据
        is_game = p["engine"] is not None or (p["kind"] == "web" and ("单车" in p["title"] or "跳" in p["title"]))
        plaza_items.append({
            "slug": slug,
            "url": f"https://{slug}.playtest.run",
            "title": p["title"],
            "developer": p["developer"],
            "avatar_url": p["avatar"],
            "summary": p["summary"],
            "note": p["note"],
            "engine": p["engine"],
            "kind": p["kind"],
            "is_game": is_game,
            "cover_hash": p["cover"]["hash"] if p["cover"] else None,
            "version": p["version"],
            "updated_at": now_iso(-idx * 4 - 1),
            "players": p["players"],
            "seeking": p["seeking"],
            "seats": p["seats"],
            "joined": p["joined"],
            "followers": p["followers"],
            "boosted": p["boosted"]
        })

    # 4. 写入 Boosts (推广位)
    cur.execute("DELETE FROM boosts WHERE slug = 'paper-plane'")
    cur.execute("""
        INSERT INTO boosts (
            slug, kind, status, granted, starts_at, ends_at, created_at, reason
        ) VALUES ('paper-plane', 'days3', 'live', 1, ?, ?, ?, '金秋精选热门作品')
    """, (now_iso(-12), now_iso(72), now_iso(-12)))

    # 5. 写入 Collections (合集与挑战)
    collections_data = [
        {
            "slug": "autumn-showcase-2026",
            "user_id": "usr-zhongshang",
            "creator": "Zhongshang Wu",
            "title": "2026 金秋独立作品试玩展",
            "summary": "精选本季由独立开发者带来的原创试玩作品，感受细腻的视听与交互设计。",
            "kind": "collection",
            "prompt": "",
            "rules": "",
            "closes_at": None,
            "entries": [
                {"slug": "lucky-robin-21", "title": "鹈鹕单车", "note": "海港微风与雨夜骑行手感"},
                {"slug": "rainy-store", "title": "雨夜便利店", "note": "白噪音与暖光故事"},
                {"slug": "paper-plane", "title": "纸飞机", "note": "指尖划风的物理飞行"},
                {"slug": "ivory-beaver-33", "title": "ODD FOLK — 原创头像工作室", "note": "像素头像矢量生成"},
                {"slug": "deep-space-beacon", "title": "深空信标", "note": "科幻硬核短篇阅读"}
            ]
        },
        {
            "slug": "microgame-72h",
            "user_id": "usr-star",
            "creator": "星轨工坊",
            "title": "72小时极简微型游戏极限挑战",
            "summary": "只用一根手指或一个按键，做出让人停不下来的微型游戏。",
            "kind": "challenge",
            "prompt": "主题：连锁反应与引力",
            "rules": "1. 单指触控操作；2. 文件体积 < 5MB；3. 首次加载耗时 < 1s。",
            "closes_at": now_iso(168), # 7 天后截止
            "entries": [
                {"slug": "paper-plane", "title": "纸飞机", "note": "极简滑翔手感"},
                {"slug": "tiny-orbit", "title": "Tiny Orbit", "note": "按压引力弹弓机制"}
            ]
        }
    ]

    for col in collections_data:
        c_slug = col["slug"]
        cur.execute("DELETE FROM collections WHERE slug = ?", (c_slug,))
        cur.execute("DELETE FROM collection_entries WHERE collection_slug = ?", (c_slug,))
        cur.execute("""
            INSERT INTO collections (
                slug, user_id, title, summary, kind, prompt, rules, closes_at,
                public, hidden, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, 0, ?, ?)
        """, (
            c_slug, col["user_id"], col["title"], col["summary"], col["kind"],
            col["prompt"], col["rules"], col["closes_at"],
            now_iso(-48), now_iso(-6)
        ))

        for entry in col["entries"]:
            cur.execute("""
                INSERT INTO collection_entries (
                    collection_slug, site_slug, user_id, submitted_version, submitted_at,
                    note, blocked
                ) VALUES (?, ?, ?, 1, ?, ?, 0)
            """, (c_slug, entry["slug"], col["user_id"], now_iso(-24), entry["note"]))

    conn.commit()
    conn.close()
    print("✅ 数据库 SQLite 注入完成！")

    # 6. 生成并上传 plaza.json 到 S3
    def plaza_sort_key(item):
        return (
            0 if item["boosted"] else 1,
            0 if item["seeking"] else 1,
            -int(datetime.fromisoformat(item["updated_at"].replace("Z", "+00:00")).timestamp())
        )
    sorted_items = list(plaza_items)
    sorted_items.sort(key=plaza_sort_key)

    s3_collections = []
    for col in collections_data:
        s3_collections.append({
            "slug": col["slug"],
            "title": col["title"],
            "summary": col["summary"],
            "kind": col["kind"],
            "prompt": col["prompt"],
            "rules": col["rules"],
            "closes_at": col["closes_at"],
            "public": True,
            "hidden": False,
            "creator": col["creator"],
            "created_at": now_iso(-48),
            "updated_at": now_iso(-6),
            "entries": [
                {
                    "slug": e["slug"],
                    "title": e["title"],
                    "submitted_version": 1,
                    "submitted_at": now_iso(-24),
                    "note": e["note"]
                }
                for e in col["entries"]
            ]
        })

    plaza_data = {
        "schema": 1,
        "generated_at": now_iso(),
        "items": sorted_items,
        "club_followers": 42,
        "collections": s3_collections
    }
    put_s3_json("plaza.json", plaza_data)
    print("✅ S3 plaza.json 上传完成！")

    print("\n🎉 全部数据写入就绪！")
    print(f"   · 作品数: {len(projects)}")
    print(f"   · 合集数: {len(collections_data)}")
    print("   · 涵盖作品类型: Godot/Phaser 游戏、Canvas 益智、SVG 头像工坊、设计手记文章、硬核科幻连载")

if __name__ == "__main__":
    main()
