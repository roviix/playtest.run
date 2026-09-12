// 作品页（DESIGN §3.13）：身份栏、「现在」那一行、五个分页签。
//
// 身份栏上的签是事实不是按钮：玩家现在看到哪一版、在不在广场上、几个人关注、名额到哪。
// 「现在」让这一页有当下——没有它，控制台就是一份报告。
// 分页签是地址的一部分（router.ts），刷新不丢。

import type { JSX } from "preact";
import { useEffect, useState } from "preact/hooks";

import { api, doorUrl, type FeedbackItem, type Site, type SiteResults, type VersionResults } from "../api";
import { useLoad, type Loaded } from "../load";
import { href, type Tab } from "../router";
import { ago, left, moment } from "../words";
import { CardTab } from "./card";
import { Cover } from "./cover";
import { FeedbackTab } from "./feedback";
import { ResultsTab } from "./results";
import { RosterTab } from "./roster";
import { SettingsTab } from "./settings";
import { Failed, Loading } from "./status";

export function SitePage({
  slug,
  initialSite,
  tab,
  version,
  plazaUrl,
  onSiteChanged,
}: {
  slug: string;
  initialSite?: Site;
  tab: Tab;
  version?: number;
  plazaUrl: string;
  onSiteChanged: () => void;
}) {
  const loaded = useLoad(() => api.site(slug), [slug]);
  const results = useLoad(() => api.results(slug), [slug]);
  // 设置改完 PATCH 会回一份最新的 Site，存在这里，身份栏上的签立刻跟着变。
  const [site, setSite] = useState<Site | null>(initialSite ?? null);
  useEffect(() => {
    if (loaded.data) setSite(loaded.data);
  }, [loaded.data]);

  if (loaded.loading && !site) return <Loading />;
  if (loaded.error && !site) return <Failed error={loaded.error} onRetry={loaded.reload} />;
  if (!site) return null;

  function changed(next: Site) {
    setSite(next);
    onSiteChanged();
  }

  function versionsChanged() {
    loaded.reload();
    results.reload();
    onSiteChanged();
  }

  const current = site.current_version;
  const rosterVersion = version ?? current;

  return (
    <>
      <a class="back-link" href={href({ name: "sites" })}>← 所有作品</a>
      <Identity site={site} />
      <a class="docs-context-link" href={href({ name: "docs", section: tab === "card" ? "share" : tab === "settings" ? "manage" : "publish" })}>
        {tab === "card" ? "邀请卡与分享用法" : tab === "settings" ? "下架、回滚与删除的区别" : "如何发布新版本"} →
      </a>
      {loaded.error ? <Failed error={loaded.error} onRetry={loaded.reload} /> : null}
      <Now site={site} results={results} />
      <nav class="tabs" aria-label="作品页">
        <TabLink slug={slug} tab="results" active={tab} text="结果" />
        <TabLink slug={slug} tab="roster" active={tab} text="点名册" version={rosterVersion} />
        <TabLink slug={slug} tab="feedback" active={tab} text="反馈" />
        <TabLink slug={slug} tab="card" active={tab} text="邀请卡" />
        <TabLink slug={slug} tab="settings" active={tab} text="设置" />
      </nav>
      <section class="tab-body" key={`${tab}-${version ?? "current"}`}>
        {tab === "results" ? (
          <ResultsTab slug={slug} site={site} results={results} onVersionsChanged={versionsChanged} />
        ) : null}
        {tab === "roster" ? (
          <RosterTab slug={slug} version={rosterVersion} versions={results.data?.versions ?? []} />
        ) : null}
        {tab === "feedback" ? <FeedbackTab slug={slug} site={site} /> : null}
        {tab === "card" ? <CardTab site={site} /> : null}
        {tab === "settings" ? (
          <SettingsTab site={site} plazaUrl={plazaUrl} onChanged={changed} onVersionsChanged={versionsChanged} />
        ) : null}
      </section>
    </>
  );
}

function TabLink({
  slug,
  tab,
  active,
  text,
  version,
}: {
  slug: string;
  tab: Tab;
  active: Tab;
  text: string;
  version?: number;
}) {
  return (
    <a
      class={`tab ${tab === active ? "active" : ""}`}
      href={href({ name: "site", slug, tab, version: tab === "roster" ? version : undefined })}
      aria-current={tab === active ? "page" : undefined}
    >
      {text}
    </a>
  );
}

// ------------------------------------------------------------------ 身份栏

function Identity({ site }: { site: Site }) {
  const [copied, setCopied] = useState<string | null>(null);
  const listing = site.listing;
  const link = doorUrl(site.url, site.slug);

  async function copy() {
    try {
      await navigator.clipboard.writeText(link);
      setCopied("已复制");
    } catch {
      // 剪贴板要安全上下文加权限；拿不到就让人自己选中地址。
      setCopied("请手动复制");
    }
    setTimeout(() => setCopied(null), 2500);
  }

  const chips: { text: string; tone: string }[] = [];
  if (site.current_version !== undefined) chips.push({ text: `v${site.current_version} · 当前`, tone: "good" });
  else chips.push({ text: "还没有版本", tone: "plain" });
  if (listing?.public) {
    if (listing.hidden) chips.push({ text: "已从广场撤下", tone: "warn" });
    else chips.push({ text: listing.seeking ? "在广场上 · 正在找人测" : "在广场上", tone: "" });
  }
  if (listing && (listing.followers ?? 0) > 0) chips.push({ text: `${listing.followers} 人关注`, tone: "plain" });
  if (listing?.seats) chips.push({ text: `${listing.joined ?? 0} / ${listing.seats} 位`, tone: "plain" });
  const remaining = left(site.expires_at);
  if (remaining) chips.push({ text: `匿名链接 ${remaining}`, tone: "warn" });

  return (
    <header class="identity">
      <Cover site={site} small />
      <div class="identity-body">
        <h1>{site.title}</h1>
        <p class="link-row">
          <a class="mono player-link" href={link} target="_blank" rel="noreferrer">
            {link.replace(/^https?:\/\//, "")}
          </a>
          <button class="button small" type="button" onClick={copy}>
            {copied ?? "复制"}
          </button>
        </p>
        <p class="chips">
          {chips.map((chip) => (
            <span key={chip.text} class={`chip ${chip.tone}`}>
              {chip.text}
            </span>
          ))}
        </p>
      </div>
    </header>
  );
}

// ------------------------------------------------------------------ 现在

const RECENT_WINDOW_MS = 60 * 60 * 1000;

/**
 * 此刻这一行。「近一小时 3 个人打开 · 最新一条反馈 12 分钟前：『……』 · 这一版 3 个错误」。
 *
 * 控制面还没有专门的「近况」字段，这里从最新一版的会话里在浏览器侧数（DESIGN §3.13 如实写了）。
 * 没有近况就退到「最近一次打开是什么时候」；一个人都没来过就说下一步该做什么。
 */
function Now({ site, results }: { site: Site; results: Loaded<SiteResults> }) {
  const current = site.current_version;
  const sessions = useLoad(
    () => (current === undefined ? Promise.resolve(null) : api.sessions(site.slug, current, "time")),
    [site.slug, current],
  );
  const feedback = useLoad(() => api.feedback(site.slug), [site.slug]);

  if (current === undefined) {
    return <p class="now muted">还没有版本。</p>;
  }
  if (sessions.loading || feedback.loading || results.loading) return <p class="now muted" role="status">正在获取最近动态…</p>;
  if (sessions.error || feedback.error || results.error) return <p class="now muted">最近动态暂时无法获取。<button class="button small" onClick={() => { sessions.reload(); feedback.reload(); results.reload(); }}>重试</button></p>;

  const rows = sessions.data?.sessions ?? [];
  const now = Date.now();
  const recent = rows.filter((row) => now - new Date(row.at).getTime() < RECENT_WINDOW_MS).length;
  const latest = newest(feedback.data?.items ?? []);
  const version: VersionResults | undefined = results.data?.versions.find((one) => one.version === current);

  const bits: JSX.Element[] = [];
  if (recent > 0) {
    bits.push(
      <span key="recent" class="now-lead">
        近一小时 <b>{recent}</b> 人打开
      </span>,
    );
  } else if (rows.length > 0) {
    bits.push(<span key="last">上次打开 {moment(rows[0].at)}</span>);
  } else if (version && version.opened > 0) {
    bits.push(<span key="last">{version.last_at ? `上次打开 ${moment(version.last_at)}` : `这一版 ${version.opened} 人打开`}</span>);
  } else {
    return <p class="now muted">这一版还没有人打开。</p>;
  }
  if (latest) {
    bits.push(
      <span key="fb">
        最新反馈 {ago(latest.ts)}「{clip(latest.text, 24)}」
      </span>,
    );
  }
  if (version && version.errors.total > 0) {
    bits.push(
      <span key="err" class="now-warn">
        {version.errors.total} 个错误
      </span>,
    );
  }

  return (
    <p class="now">
      <span class="now-dot" aria-hidden="true" />
      {bits.map((bit, index) => (
        <span key={index} class="now-bit">
          {index > 0 ? <span class="now-sep"> · </span> : null}
          {bit}
        </span>
      ))}
    </p>
  );
}

function newest(items: FeedbackItem[]): FeedbackItem | null {
  if (items.length === 0) return null;
  return items.reduce((best, one) => (one.ts > best.ts ? one : best), items[0]);
}

function clip(text: string, max: number): string {
  const chars = Array.from(text);
  return chars.length > max ? `${chars.slice(0, max).join("")}…` : text;
}
