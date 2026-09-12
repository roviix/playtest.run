// 控制面的响应体类型在 ./generated/api.ts，由 common/src/*.rs 生成——那边改了这边跟着重新生成，
// 不再手抄。两个进程不会同时升级，所以带 `?` 的字段在旧控制面上可能不存在。

import { paths } from "./generated/api";
import type { Collection, CollectionDraft, EntryDraft } from "./generated/api";
export type { Collection, CollectionDraft, EntryDraft, CreationMethod } from "./generated/api";
import type {
  FeedbackItem,
  FeedbackList,
  LoginResponse,
  Me,
  RosterSort,
  Site,
  SiteResults,
  UpdateFeedbackRequest,
  UpdateSiteRequest,
  VersionList,
  VersionSessions,
} from "./generated/api";

export type {
  Boost,
  BoostKind,
  BoostStatus,
  ErrorSummary,
  ErrorTally,
  FeedbackItem,
  FeedbackList,
  FeedbackStatus,
  Listing,
  LoginResponse,
  Me,
  RosterSort,
  SessionEvent,
  SessionRow,
  Site,
  SiteResults,
  SourceTally,
  UpdateFeedbackRequest,
  UpdateSiteRequest,
  VersionInfo,
  VersionList,
  VersionResults,
  VersionSessions,
} from "./generated/api";

// 构建出来的静态站默认打同源的 /v1（控制台和 api 都在 playtest.roviix.com 下）。
// 开发时 Vite 把 /v1 代理到本机的控制面，见 vite.config.ts。
const API_BASE = import.meta.env.VITE_PLAYTEST_API ?? "";

const TOKEN_KEY = "playtest.token";

/** 邀请卡的地址，和 common/src/lib.rs 的 `CARD_PATH` / `CARD_WIDE_PATH` / `card_url()` 是同一份。 */
export const CARD_PATH = "/_playtest/card.png";
export const CARD_WIDE_PATH = "/_playtest/card-wide.png";
/** 封面由边缘按当前版本给（edge/src/app.rs 的 `cover`），带 `?v=` 绕开缓存。 */
export const COVER_PATH = "/_playtest/cover";

function under(siteUrl: string, path: string): string {
  const doorMatch = siteUrl.match(/^(https?:\/\/)([^/]+)\/p\/([^/?#]+)/);
  if (doorMatch) {
    const [, scheme, root, slug] = doorMatch;
    return `${scheme}${slug}.${root}${path}`;
  }
  return `${siteUrl.replace(/\/+$/, "")}${path}`;
}

/** 作品主域邀请函完整链接（Front Door，DESIGN §3.1 与 §3.3）。 */
export function doorUrl(siteUrl: string, slug: string): string {
  if (siteUrl.includes("/p/")) {
    return siteUrl;
  }
  const m = siteUrl.match(/^(https?:\/\/)(?:[^./]+\.)?([^/]+)/);
  if (!m) return `${siteUrl}/p/${slug}`;
  const [, scheme, root] = m;
  return `${scheme}${root}/p/${slug}`;
}

export function cardUrl(siteUrl: string): string {
  return under(siteUrl, CARD_PATH);
}

export function cardWideUrl(siteUrl: string): string {
  return under(siteUrl, CARD_WIDE_PATH);
}

export function coverUrl(siteUrl: string, version: number): string {
  return `${under(siteUrl, COVER_PATH)}?v=${version}`;
}

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

export async function revokeToken(): Promise<void> {
  try {
    await call<void>("/v1/me/token", { method: "DELETE" });
  } catch (error) {
    if (!(error instanceof ApiError && error.needsToken)) throw error;
  }
  forgetToken();
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

/** 浏览器直接访问这个地址就被送到 GitHub；回来时 GitHub 把 code 和 state 挂在控制台地址上。 */
export const githubLoginUrl = `${API_BASE}${paths.loginWebStart}`;

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
    response = await fetch(`${API_BASE}${paths.loginWebExchange}`, {
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
  collections: () => call<Collection[]>(paths.collections),
  openCollections: () => call<Collection[]>(paths.openCollections),
  collection: (slug: string) => call<Collection>(paths.collection(slug)),
  createCollection: (draft: CollectionDraft) => call<Collection>(paths.collections, { method: "POST", body: JSON.stringify(draft) }),
  updateCollection: (slug: string, draft: CollectionDraft) => call<Collection>(paths.collection(slug), { method: "PUT", body: JSON.stringify(draft) }),
  deleteCollection: (slug: string) => call<void>(paths.collection(slug), { method: "DELETE" }),
  submitCollection: (slug: string, draft: EntryDraft) => call<Collection>(paths.collectionEntries(slug), { method: "POST", body: JSON.stringify(draft) }),
  withdrawCollection: (slug: string, site: string) => call<void>(paths.collectionEntry(slug, site), { method: "DELETE" }),
  unblockCollection: (slug: string, site: string) => call<void>(paths.collectionBlock(slug, site), { method: "DELETE" }),
  me: () => call<Me>(paths.me),
  sites: () => call<Site[]>(paths.projects),
  site: (slug: string) => call<Site>(paths.project(slug)),
  // 名额传 0、群链接传空字符串就是清掉，见 common/src/api.rs 的 UpdateSiteRequest。
  updateSite: (slug: string, request: UpdateSiteRequest) =>
    call<Site>(paths.project(slug), { method: "PATCH", body: JSON.stringify(request) }),
  // 删除：链接立刻失效，版本与结果一起走。控制面回 204。
  deleteSite: (slug: string) => call<void>(paths.project(slug), { method: "DELETE" }),
  versions: (slug: string) => call<VersionList>(paths.projectVersions(slug)),
  results: (slug: string) => call<SiteResults>(paths.projectResults(slug)),
  // 回滚：把「当前版本」指回某一版。版本都在，边缘 1 秒内看到新指针（DESIGN §3.7）。
  activateVersion: (slug: string, version: number) =>
    call<Site>(paths.projectVersionActivate(slug, version), { method: "POST", body: "{}" }),
  sessions: (slug: string, version: number, sort: RosterSort) =>
    call<VersionSessions>(`${paths.projectVersionSessions(slug, version)}?sort=${sort}`),
  feedback: (slug: string) => call<FeedbackList>(paths.projectFeedback(slug)),
  updateFeedback: (slug: string, id: number, request: UpdateFeedbackRequest) =>
    call<FeedbackItem>(paths.projectFeedbackItem(slug, id), {
      method: "PATCH",
      body: JSON.stringify(request),
    }),
};
