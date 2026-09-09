// 控制面的响应体。字段和 common/src/results.rs、common/src/api.rs 一一对应，
// 那边改了这里跟着改——两个进程不会同时升级，所以新字段一律当作可能不存在。

// 构建出来的静态站默认打同源的 /v1（控制台和 api 都在 playtest.roviix.com 下）。
// 开发时 Vite 把 /v1 代理到本机的控制面，见 vite.config.ts。
const API_BASE = import.meta.env.VITE_PLAYTEST_API ?? "";

const TOKEN_KEY = "playtest.token";

/** 广场上的状态（DESIGN §3.8）。旧控制面不返回这一段，按「不公开」处理。 */
export type Listing = {
  public: boolean;
  seeking: boolean;
  seek_note?: string;
  summary?: string;
  /** 被举报到阈值或我们手工撤下：开发者勾着公开，但广场上没有它。必须告诉他。 */
  hidden: boolean;
  has_cover: boolean;
};

export type Site = {
  slug: string;
  url: string;
  title: string;
  current_version?: number;
  created_at: string;
  expires_at?: string;
  listing?: Listing;
};

export type UpdateSiteRequest = {
  public?: boolean;
  seeking?: boolean;
  /** 空字符串表示清掉。 */
  seek_note?: string;
};

export type ErrorTally = { fingerprint: string; count: number };

export type ErrorSummary = { distinct: number; total: number; top?: ErrorTally[] };

export type VersionResults = {
  version: number;
  created_at?: string;
  note?: string;
  opened: number;
  entered: number;
  /** null = 这一版没人报过首帧，多半是没接 SDK，我们不知道，不是 0。 */
  dropped_before_first_frame: number | null;
  returned: number;
  dwell_median_s: number | null;
  played_5min_plus: number;
  errors: ErrorSummary;
  load_failures: number;
  feedback_count: number;
  first_at?: string;
  last_at?: string;
};

export type SiteResults = {
  slug: string;
  title: string;
  current_version?: number;
  versions: VersionResults[];
};

export type SessionEvent = {
  ts: string;
  source: string;
  kind: string;
  name?: string;
  data?: unknown;
};

export type SessionRow = {
  id: string;
  at: string;
  device?: string;
  browser?: string;
  os?: string;
  wechat: boolean;
  referrer_kind?: string;
  started: boolean;
  first_frame: boolean;
  entered: boolean;
  dwell_s: number;
  last_input_after_s: number | null;
  reached?: string;
  errors: number;
  feedback: number;
  is_return: boolean;
  events?: SessionEvent[];
  more_events?: boolean;
};

export type RosterSort = "dwell" | "time";

export type VersionSessions = {
  slug: string;
  version: number;
  sort: RosterSort;
  sessions: SessionRow[];
};

export type FeedbackStatus = "new" | "seen" | "done";

export type FeedbackItem = {
  id: number;
  session_id: string;
  version: number;
  ts: string;
  text: string;
  seconds_in?: number;
  device?: string;
  browser?: string;
  screenshot_hash?: string;
  status: FeedbackStatus;
};

export type FeedbackList = { slug: string; items: FeedbackItem[] };

export class ApiError extends Error {
  status: number;
  code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.status = status;
    this.code = code;
  }

  /** 令牌不对或过期了，界面要把人送回粘贴令牌那一页。 */
  get needsToken(): boolean {
    return this.status === 401;
  }
}

export function readToken(): string {
  return localStorage.getItem(TOKEN_KEY) ?? "";
}

export function saveToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token.trim());
}

export function forgetToken(): void {
  localStorage.removeItem(TOKEN_KEY);
}

async function call<T>(path: string, init: RequestInit = {}): Promise<T> {
  const token = readToken();
  if (!token) {
    throw new ApiError(401, "unauthorized", "还没有令牌。先粘贴一个。");
  }

  const headers = new Headers(init.headers);
  headers.set("Authorization", `Bearer ${token}`);
  if (init.body) headers.set("Content-Type", "application/json");

  let response: Response;
  try {
    response = await fetch(`${API_BASE}${path}`, { ...init, headers });
  } catch {
    // 断网、控制面没起、代理没配都会走到这里。说清楚下一步做什么。
    throw new ApiError(0, "offline", "连不上控制面。确认它在跑，或者检查一下网络。");
  }

  if (response.status === 204) return undefined as T;

  const text = await response.text();
  let parsed: unknown;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    throw new ApiError(response.status, "internal", `控制面返回了看不懂的东西：${text.slice(0, 200)}`);
  }

  if (!response.ok) {
    const body = parsed as { code?: string; message?: string } | null;
    throw new ApiError(
      response.status,
      body?.code ?? "internal",
      body?.message ?? `控制面返回了 ${response.status}。`,
    );
  }
  return parsed as T;
}

/** `GET /v1/me`：这个令牌是谁。 */
export type Me = {
  kind: "anon" | "github";
  display_name: string;
  login?: string;
  expires_at?: string;
};

export type LoginResponse = {
  token: string;
  login: string;
  display_name: string;
  migrated_sites: number;
};

/** 浏览器直接访问这个地址就被送到 GitHub；回来时 GitHub 把 code 和 state 挂在控制台地址上。 */
export const githubLoginUrl = `${API_BASE}/v1/login/github/start`;

/**
 * 网页登录的最后一步：把 GitHub 回传的 code 与 state 交给控制面换令牌。
 * 手里若有一个匿名令牌就一起带上——那个身份下的作品会归到账号里。这里不走 call()，
 * 因为没有令牌也得能调。
 */
export async function exchangeGitHubCode(code: string, state: string): Promise<LoginResponse> {
  const headers = new Headers({ "Content-Type": "application/json" });
  const anon = readToken();
  if (anon) headers.set("Authorization", `Bearer ${anon}`);
  let response: Response;
  try {
    response = await fetch(`${API_BASE}/v1/login/github/exchange`, {
      method: "POST",
      headers,
      body: JSON.stringify({ code, state }),
    });
  } catch {
    throw new ApiError(0, "offline", "连不上控制面。确认它在跑，或者检查一下网络。");
  }
  const body = (await response.json().catch(() => null)) as
    | (LoginResponse & { code?: string; message?: string })
    | null;
  if (!response.ok || !body?.token) {
    throw new ApiError(
      response.status,
      body?.code ?? "internal",
      body?.message ?? `控制面返回了 ${response.status}。`,
    );
  }
  return body;
}

export const api = {
  me: () => call<Me>("/v1/me"),
  sites: () => call<Site[]>("/v1/sites"),
  updateSite: (slug: string, request: UpdateSiteRequest) =>
    call<Site>(`/v1/sites/${encodeURIComponent(slug)}`, {
      method: "PATCH",
      body: JSON.stringify(request),
    }),
  results: (slug: string) => call<SiteResults>(`/v1/sites/${encodeURIComponent(slug)}/results`),
  // 回滚：把「当前版本」指回某一版。清单都在，边缘 1 秒内看到新指针（DESIGN §3.5）。
  activateVersion: (slug: string, version: number) =>
    call<Site>(`/v1/sites/${encodeURIComponent(slug)}/versions/${version}/activate`, {
      method: "POST",
      body: "{}",
    }),
  sessions: (slug: string, version: number, sort: RosterSort) =>
    call<VersionSessions>(
      `/v1/sites/${encodeURIComponent(slug)}/versions/${version}/sessions?sort=${sort}`,
    ),
  feedback: (slug: string) => call<FeedbackList>(`/v1/sites/${encodeURIComponent(slug)}/feedback`),
  markFeedback: (slug: string, id: number, status: FeedbackStatus) =>
    call<FeedbackItem>(`/v1/sites/${encodeURIComponent(slug)}/feedback/${id}`, {
      method: "PATCH",
      body: JSON.stringify({ status }),
    }),
};
