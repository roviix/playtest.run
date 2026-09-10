// 只在开发时用的假控制面。不是产品的一部分，不进构建产物。
//
// 为什么要它：控制面这几天正在长「名额 / 关注 / 公开反馈 / 推广」这几张表，
// 边缘的邀请卡也还没开始画。控制台这一侧要能独立看见每一块长什么样，
// 所以照着 common/src/ 的契约摆一份数据出来，形状对得上就够了。
//
//   node console/dev/mock-api.mjs            # 听 127.0.0.1:8799
//   PLAYTEST_API=http://127.0.0.1:8799 npm --prefix console run dev
//
// 令牌不校验，只要求带一个——粘贴令牌那一页的流程照旧要走一遍。
// 邀请卡返回的是一张 SVG 占位图（路径仍是 .png，浏览器按 Content-Type 认）：
// 真的那张由边缘按当前版本用 resvg 光栅化，这里只是让版式和失败态看得见。

import { createServer } from 'node:http';

const port = Number(process.env.PORT || 8799);
const origin = `http://brisk-otter-41.localhost:${port}`;

const sites = [
  {
    slug: 'brisk-otter-41',
    url: origin,
    title: '潮汐小镇',
    current_version: 7,
    created_at: '2026-08-30T02:10:00Z',
    listing: {
      public: true,
      seeking: true,
      seek_note: '新手引导看得懂吗？第三关难不难？',
      summary: '一个退潮时才能过河的小镇，十分钟能玩完。',
      hidden: false,
      has_cover: true,
      seats: 10,
      joined: 6,
      followers: 12,
      community_url: 'https://example.com/qun',
      feedback_public: true,
      boost: {
        id: 1,
        slug: 'brisk-otter-41',
        kind: 'days7',
        status: 'live',
        granted: true,
        starts_at: '2026-09-08T00:00:00Z',
        ends_at: '2026-09-15T00:00:00Z',
        created_at: '2026-09-07T09:00:00Z',
      },
    },
  },
  {
    slug: 'sage-lynx-30',
    url: `http://sage-lynx-30.localhost:${port}`,
    title: '一个小工具',
    current_version: 2,
    created_at: '2026-09-06T11:00:00Z',
    listing: {
      public: false,
      seeking: false,
      hidden: false,
      has_cover: false,
      joined: 0,
      followers: 0,
      feedback_public: false,
    },
  },
];

const results = {
  'brisk-otter-41': {
    slug: 'brisk-otter-41',
    title: '潮汐小镇',
    current_version: 7,
    followers: 12,
    versions: [
      {
        version: 7,
        created_at: '2026-09-05T14:20:00Z',
        note: '改了新手引导',
        opened: 8,
        entered: 8,
        dropped_before_first_frame: 2,
        returned: 2,
        dwell_median_s: 45,
        played_5min_plus: 3,
        errors: {
          distinct: 1,
          total: 3,
          top: [{ fingerprint: "TypeError: Cannot read 'x' of undefined @ main.js:412", count: 3 }],
        },
        load_failures: 0,
        feedback_count: 2,
        named: 4,
        sources: [
          { kind: 'card', count: 4 },
          { kind: 'plaza', count: 2 },
          { kind: 'wechat', count: 2 },
        ],
      },
      {
        version: 6,
        created_at: '2026-09-03T16:40:00Z',
        note: '加了音效',
        opened: 5,
        entered: 5,
        dropped_before_first_frame: 2,
        returned: 0,
        dwell_median_s: 25,
        played_5min_plus: 1,
        errors: {
          distinct: 1,
          total: 3,
          top: [{ fingerprint: "TypeError: Cannot read 'x' of undefined @ main.js:412", count: 3 }],
        },
        load_failures: 2,
        feedback_count: 1,
        named: 1,
        sources: [
          { kind: 'wechat', count: 3 },
          { kind: 'notice', count: 2 },
        ],
      },
      {
        // 没接 SDK、也没有任何来源可归类的一版：那两句都不该出现。
        version: 5,
        created_at: '2026-09-01T09:12:00Z',
        note: '第一版，能跑了',
        opened: 4,
        entered: 4,
        dropped_before_first_frame: null,
        returned: 0,
        dwell_median_s: 40,
        played_5min_plus: 1,
        errors: { distinct: 0, total: 0 },
        load_failures: 0,
        feedback_count: 0,
        named: 0,
        sources: [],
      },
    ],
  },
  'sage-lynx-30': {
    slug: 'sage-lynx-30',
    title: '一个小工具',
    current_version: 2,
    followers: 0,
    versions: [
      {
        version: 2,
        created_at: '2026-09-06T11:30:00Z',
        opened: 0,
        entered: 0,
        dropped_before_first_frame: null,
        returned: 0,
        dwell_median_s: null,
        played_5min_plus: 0,
        errors: { distinct: 0, total: 0 },
        load_failures: 0,
        feedback_count: 0,
        named: 0,
      },
    ],
  },
};

const person = (id, at, dwell, extra) => ({
  id,
  at,
  dwell_s: dwell,
  wechat: false,
  started: true,
  first_frame: true,
  entered: true,
  last_input_after_s: null,
  errors: 0,
  feedback: 0,
  is_return: false,
  events: [],
  ...extra,
});

const sessions = [
  person('7h2f1c9b4a', '2026-09-05T14:35:00Z', 7, {
    device: 'phone',
    browser: 'wechat',
    os: 'android',
    first_frame: false,
    entered: false,
    referrer_kind: 'card',
    wechat: true,
  }),
  person('7g8e2d1a05', '2026-09-05T14:28:00Z', 11, {
    name: '阿树',
    device: 'phone',
    browser: 'wechat',
    os: 'ios',
    first_frame: false,
    entered: false,
    referrer_kind: 'card',
    wechat: true,
  }),
  person('7f3c0b7e21', '2026-09-05T14:25:00Z', 30, {
    device: 'tablet',
    browser: 'safari',
    os: 'ios',
    referrer_kind: 'plaza',
    errors: 1,
  }),
  person('7a9d4e2f66', '2026-09-05T14:00:00Z', 45, {
    name: '小雨',
    device: 'phone',
    browser: 'safari',
    os: 'ios',
    referrer_kind: 'card',
    reached: '过了引导',
    feedback: 1,
  }),
  person('7e5b1a8c30', '2026-09-05T14:22:30Z', 100, {
    device: 'desktop',
    browser: 'chrome',
    os: 'macos',
    referrer_kind: 'notice',
    errors: 2,
    is_return: true,
  }),
  person('7d6f2c4b19', '2026-09-05T14:20:00Z', 660, {
    name: '老周',
    device: 'desktop',
    browser: 'firefox',
    os: 'windows',
    referrer_kind: 'plaza',
    reached: '第一关过了',
  }),
  person('7c1e8a5d72', '2026-09-05T14:05:00Z', 680, {
    name: '柚子',
    device: 'phone',
    browser: 'chrome',
    os: 'android',
    referrer_kind: 'direct',
    reached: '第二关过了',
    is_return: true,
  }),
  person('7b4a3f9e08', '2026-09-05T14:02:10Z', 720, {
    device: 'phone',
    browser: 'wechat',
    os: 'android',
    wechat: true,
    reached: '第一关过了',
    feedback: 1,
  }),
];

const feedback = [
  {
    id: 1,
    session_id: '7a9d4e2f66',
    version: 7,
    ts: '2026-09-05T14:02:57Z',
    text: '不知道要按哪个键',
    seconds_in: 47,
    device: 'phone',
    browser: 'safari',
    status: 'new',
    name: '小雨',
    public: true,
  },
  {
    id: 2,
    session_id: '7b4a3f9e08',
    version: 7,
    ts: '2026-09-05T14:12:20Z',
    text: '退潮那一下很好看，就是第三关的桥找不到',
    seconds_in: 320,
    device: 'phone',
    browser: 'wechat',
    status: 'seen',
    public: false,
  },
  {
    id: 3,
    session_id: '6c2b7d1e40',
    version: 6,
    ts: '2026-09-03T18:02:00Z',
    text: '音效太大了，进来吓一跳',
    seconds_in: 12,
    device: 'desktop',
    browser: 'chrome',
    status: 'done',
    name: '老周',
    public: true,
  },
];

const me = {
  kind: 'github',
  display_name: '小海',
  login: 'xiaohai',
  avatar_url: `http://127.0.0.1:${port}/mock/avatar.png`,
};

// ------------------------------------------------------------------ 两张占位图

const avatarSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
<rect width="64" height="64" fill="#2f6f4f"/>
<circle cx="32" cy="25" r="11" fill="#cfe8d8"/>
<circle cx="32" cy="58" r="19" fill="#cfe8d8"/></svg>`;

/** 邀请卡的版式占位：深底、一张图、一个署名、一个日期、一条撕票线（DESIGN §3.4）。 */
function cardSvg(version) {
  const squares = [];
  for (let i = 0; i < 64; i++) {
    if ((i * 7 + ((i / 8) | 0) * 3) % 3 === 0) continue;
    squares.push(
      `<rect x="${430 + (i % 8) * 26}" y="${1010 + (((i / 8) | 0) * 26)}" width="22" height="22" fill="#12141a"/>`,
    );
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1350" viewBox="0 0 1080 1350">
<rect width="1080" height="1350" fill="#14161c"/>
<rect x="60" y="60" width="960" height="520" rx="24" fill="#22303a"/>
<text x="540" y="330" fill="#5d7a8c" font-family="sans-serif" font-size="44" text-anchor="middle">封面</text>
<text x="60" y="700" fill="#f2f0ec" font-family="sans-serif" font-size="54">小海 邀请你试玩</text>
<text x="60" y="790" fill="#ffb224" font-family="sans-serif" font-size="66" font-weight="bold">《潮汐小镇》</text>
<text x="60" y="870" fill="#9aa2b1" font-family="sans-serif" font-size="38">一个退潮时才能过河的小镇，十分钟能玩完。</text>
<text x="60" y="950" fill="#9aa2b1" font-family="sans-serif" font-size="34">v${version} · 9 月 9 日 · 在找 10 位试玩者</text>
<rect x="420" y="1000" width="240" height="240" rx="12" fill="#f2f0ec"/>${squares.join('')}
<line x1="60" y1="1290" x2="1020" y2="1290" stroke="#3a3f4a" stroke-width="3" stroke-dasharray="14 14"/>
<text x="60" y="1330" fill="#6b7076" font-family="monospace" font-size="28">playtest.run</text></svg>`;
}

// ------------------------------------------------------------------ 路由

const json = (res, body, status = 200) => {
  const text = JSON.stringify(body);
  res.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store',
  });
  res.end(text);
};

const svg = (res, body) => {
  res.writeHead(200, { 'content-type': 'image/svg+xml; charset=utf-8', 'cache-control': 'no-store' });
  res.end(body);
};

const notFound = (res) => json(res, { code: 'not_found', message: '假控制面没有这条路径。' }, 404);

const body = (req) =>
  new Promise((resolve) => {
    let text = '';
    req.on('data', (chunk) => (text += chunk));
    req.on('end', () => {
      try {
        resolve(JSON.parse(text || '{}'));
      } catch {
        resolve({});
      }
    });
  });

createServer(async (req, res) => {
  const url = new URL(req.url, 'http://x');
  const path = url.pathname;
  const parts = path.split('/').filter(Boolean);

  if (path === '/mock/avatar.png') return svg(res, avatarSvg);
  if (path === '/_playtest/card.png') {
    // 只有第一个作品有卡。另一个故意 404，好让「邀请卡稍后可拿」那条失败态也看得见——
    // 边缘现在还没开始画卡，线上短期内两个作品都会是这个样子。
    const host = String(req.headers.host || '');
    if (!host.startsWith('brisk-otter-41.')) return notFound(res);
    return svg(res, cardSvg(url.searchParams.get('v') ?? '7'));
  }

  if (!path.startsWith('/v1/')) return notFound(res);
  if (!req.headers.authorization) {
    return json(res, { code: 'unauthorized', message: '还没有令牌。先粘贴一个。' }, 401);
  }

  if (path === '/v1/me') return json(res, me);
  if (path === '/v1/sites' && req.method === 'GET') return json(res, sites);

  // /v1/sites/<slug>/...
  const slug = parts[2];
  const site = sites.find((one) => one.slug === slug);
  if (parts[0] === 'v1' && parts[1] === 'sites' && site) {
    const tail = parts.slice(3);
    if (tail.length === 0) {
      if (req.method === 'PATCH') {
        const patch = await body(req);
        // 契约：seats 传 0、community_url 传空字符串就是清掉。
        if ('seats' in patch) {
          if (patch.seats === 0) delete site.listing.seats;
          else site.listing.seats = patch.seats;
        }
        if ('community_url' in patch) {
          if (patch.community_url === '') delete site.listing.community_url;
          else site.listing.community_url = patch.community_url;
        }
        for (const key of ['public', 'seeking', 'feedback_public']) {
          if (key in patch) site.listing[key] = patch[key];
        }
        if ('seek_note' in patch) site.listing.seek_note = patch.seek_note || undefined;
      }
      return json(res, site);
    }
    if (tail[0] === 'results') return json(res, results[slug]);
    if (tail[0] === 'versions' && tail[2] === 'sessions') {
      const sort = url.searchParams.get('sort') === 'time' ? 'time' : 'dwell';
      const rows = [...(slug === 'brisk-otter-41' && tail[1] === '7' ? sessions : [])];
      rows.sort((a, b) => (sort === 'time' ? b.at.localeCompare(a.at) : a.dwell_s - b.dwell_s));
      return json(res, { slug, version: Number(tail[1]), sort, sessions: rows });
    }
    if (tail[0] === 'feedback' && tail.length === 1) {
      const items = slug === 'brisk-otter-41' ? feedback : [];
      return json(res, { slug, items });
    }
    if (tail[0] === 'feedback' && req.method === 'PATCH') {
      const item = feedback.find((one) => one.id === Number(tail[1]));
      if (!item) return notFound(res);
      const patch = await body(req);
      if ('status' in patch) item.status = patch.status;
      if ('public' in patch) item.public = patch.public;
      return json(res, item);
    }
  }

  return notFound(res);
}).listen(port, '127.0.0.1', () => {
  console.log(`假控制面在 http://127.0.0.1:${port}`);
  console.log(`控制台：PLAYTEST_API=http://127.0.0.1:${port} npm --prefix console run dev`);
});
