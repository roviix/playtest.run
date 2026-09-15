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
import { ago, left, moment, workKind } from "../words";
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
  // 只保留本作品访问过的数据页；返回分页不再卸载重取，设置页仍按需装载。
  const [visited, setVisited] = useState<Tab[]>([tab]);
  useEffect(() => {
    setVisited((tabs) => tabs.includes(tab) ? tabs : [...tabs, tab]);
  }, [tab]);
  const shown = (name: Tab) => name === tab || visited.includes(name);
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
      <a class="back-link" href={href({ name: "sites" })}>← All Works</a>
      <Identity site={site} />
      {loaded.error ? <Failed error={loaded.error} onRetry={loaded.reload} /> : null}
      <Now site={site} results={results} />
      <nav class="tabs" aria-label="Work tabs">
        <TabLink slug={slug} tab="results" active={tab} text="Results" />
        <TabLink slug={slug} tab="roster" active={tab} text="Visitors" version={rosterVersion} />
        <TabLink slug={slug} tab="feedback" active={tab} text="Feedback" />
        <TabLink slug={slug} tab="card" active={tab} text="Invitation Card" />
        <TabLink slug={slug} tab="settings" active={tab} text="Settings" />
      </nav>
      {shown("results") ? <section class="tab-body" hidden={tab !== "results"} aria-label="Results">
        <ResultsTab slug={slug} site={site} results={results} onVersionsChanged={versionsChanged} />
      </section> : null}
      {shown("roster") ? <section class="tab-body" hidden={tab !== "roster"} aria-label="Visitors">
        <RosterTab slug={slug} version={rosterVersion} versions={results.data?.versions ?? []} />
      </section> : null}
      {shown("feedback") ? <section class="tab-body" hidden={tab !== "feedback"} aria-label="Feedback">
        <FeedbackTab slug={slug} site={site} />
      </section> : null}
      {tab === "card" ? <section class="tab-body" aria-label="Invitation Card"><CardTab site={site} /></section> : null}
      {tab === "settings" ? <section class="tab-body" aria-label="Settings">
        <SettingsTab site={site} plazaUrl={plazaUrl} onChanged={changed} onVersionsChanged={versionsChanged} />
      </section> : null}
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
      setCopied("Copied");
    } catch {
      // 剪贴板要安全上下文加权限；拿不到就让人自己选中地址。
      setCopied("Copy manually");
    }
    setTimeout(() => setCopied(null), 2500);
  }

  const chips: { text: string; tone: string }[] = [];
  chips.push({ text: workKind(site.kind), tone: "plain" });
  if (site.current_version !== undefined) chips.push({ text: `v${site.current_version} · Current`, tone: "good" });
  else chips.push({ text: "No version yet", tone: "plain" });
  if (listing?.public) {
    if (listing.hidden) chips.push({ text: "Unlisted", tone: "warn" });
    else chips.push({ text: listing.seeking ? "On Plaza · Seeking Testers" : "On Plaza", tone: "" });
  }
  if (listing && (listing.followers ?? 0) > 0) chips.push({ text: `${listing.followers} followers`, tone: "plain" });
  if (listing?.seats) chips.push({ text: `${listing.joined ?? 0} / ${listing.seats} seats`, tone: "plain" });
  const remaining = left(site.expires_at);
  if (remaining) chips.push({ text: `Anonymous · ${remaining}`, tone: "warn" });

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
            {copied ?? "Copy"}
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
 */
function Now({ site, results }: { site: Site; results: Loaded<SiteResults> }) {
  const current = site.current_version;
  const sessions = useLoad(
    () => (current === undefined ? Promise.resolve(null) : api.sessions(site.slug, current, "time")),
    [site.slug, current],
  );
  const feedback = useLoad(() => api.feedback(site.slug), [site.slug]);

  if (current === undefined) {
    return (
      <p class="now muted">
        <span class="now-dot idle" aria-hidden="true" />
        No version published yet.
      </p>
    );
  }
  if (sessions.loading || feedback.loading || results.loading) {
    return (
      <p class="now muted" role="status">
        <span class="now-dot idle" aria-hidden="true" />
        Loading recent activity…
      </p>
    );
  }
  if (sessions.error || feedback.error || results.error) {
    return (
      <p class="now muted">
        <span class="now-dot idle" aria-hidden="true" />
        Recent activity temporarily unavailable.
        <button class="button small" onClick={() => { sessions.reload(); feedback.reload(); results.reload(); }}>Retry</button>
      </p>
    );
  }

  const rows = sessions.data?.sessions ?? [];
  const now = Date.now();
  const recent = rows.filter((row) => now - new Date(row.at).getTime() < RECENT_WINDOW_MS).length;
  const latest = newest(feedback.data?.items ?? []);
  const version: VersionResults | undefined = results.data?.versions.find((one) => one.version === current);

  const hasErrors = (version?.errors.total ?? 0) > 0;
  const isLive = recent > 0;
  const dotState = hasErrors ? "warn" : isLive ? "live" : "idle";
  const containerClass = `now ${hasErrors ? "has-warn" : isLive ? "has-live" : ""}`.trim();

  const bits: JSX.Element[] = [];
  if (recent > 0) {
    bits.push(
      <span key="recent" class="now-lead">
        <b>{recent}</b> {recent === 1 ? "visitor" : "visitors"} in the last hour
      </span>,
    );
  } else if (rows.length > 0) {
    bits.push(<span key="last">Last opened {moment(rows[0].at)}</span>);
  } else if (version && version.opened > 0) {
    bits.push(<span key="last">{version.last_at ? `Last opened ${moment(version.last_at)}` : `${version.opened} visitors on v${current}`}</span>);
  } else {
    return (
      <p class="now muted">
        <span class="now-dot idle" aria-hidden="true" />
        No visits on v{current} yet. Share the door link to invite first testers.
      </p>
    );
  }

  if (latest) {
    bits.push(
      <span key="fb">
        Latest feedback {ago(latest.ts)}: "{clip(latest.text, 24)}"
      </span>,
    );
  }

  if (version && version.errors.total > 0) {
    const topError = version.errors.top?.[0]?.fingerprint;
    const errorSnippet = topError ? ` (${clip(topError, 28)})` : "";
    bits.push(
      <span key="err" class="now-warn">
        ⚠️ {version.errors.total} {version.errors.total === 1 ? "error" : "errors"}{errorSnippet}
      </span>,
    );
  }

  return (
    <p class={containerClass}>
      <span class={`now-dot ${dotState}`} aria-hidden="true" />
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
