// 首页是作品墙（DESIGN §3.13）：一张卡一个作品，和广场上的卡是同一种物件。
//
// 卡右下只放一件事实，规则和广场卡（§3.9）同一份：名额 → 到期 → 关注 → 最新版本。
// 没有作品时不是一句灰字，是一个设计好的起点：第一次登录的人面对的就是这一屏。

import { useState } from "preact/hooks";

import type { Me, Site } from "../api";
import type { Loaded } from "../load";
import { href } from "../router";
import { left, moment, workKind } from "../words";
import { Cover } from "./cover";
import { Empty, Failed, Loading } from "./status";

export function HomePage({ sites, me }: { sites: Loaded<Site[]>; me: Me | null }) {
  const data = sites.data;

  if (sites.loading) return <Loading />;
  if (sites.error) return <Failed error={errorOf(sites)} onRetry={sites.reload} />;
  const count = data?.length ?? 0;

  return (
    <>
      <header class="workspace-head">
        <h1>My Projects</h1>
        <span class="workspace-count">{count} {count === 1 ? "project" : "projects"}</span>
      </header>
      {!data || data.length === 0 ? (
        <EmptyHome me={me} />
      ) : (
        <ul class="wall">
          {data.map((site) => (
            <li key={site.slug}>
              <Tile site={site} />
            </li>
          ))}
        </ul>
      )}
    </>
  );
}

function errorOf(sites: Loaded<Site[]>) {
  return sites.error!;
}

function Tile({ site }: { site: Site }) {
  const tag = tagOf(site);
  const summary = (site.listing?.seeking ? site.listing.seek_note : undefined) || site.listing?.summary;

  return (
    <a class="tile" href={href({ name: "site", slug: site.slug, tab: "results" })}>
      <Cover site={site}>{tag ? <span class={`tag ${tag.tone}`}>{tag.text}</span> : null}</Cover>
      <div class="tile-body">
        <h2>{site.title}</h2>
        {summary ? <p class="summary">{summary}</p> : null}
        <p class="meta">
          <span class="tile-slug mono">{workKind(site.kind)} · {site.slug}</span>
          <span class="fact">{factOf(site)}</span>
        </p>
      </div>
    </a>
  );
}

/** 窗左上最多一个标：撤下 > 正在找人测 > 在广场上。私密作品没有标。 */
function tagOf(site: Site): { text: string; tone: string } | null {
  const listing = site.listing;
  if (!listing?.public) return null;
  if (listing.hidden) return { text: "Unlisted", tone: "warn" };
  if (listing.seeking) return { text: "Seeking Testers", tone: "" };
  return { text: "On Plaza", tone: "plain" };
}

/** 卡上只出现一件事实。 */
function factOf(site: Site): string {
  const listing = site.listing;
  if (listing?.seats) return `${listing.joined ?? 0} / ${listing.seats} seats`;
  const remaining = left(site.expires_at);
  if (remaining) return remaining;
  if (listing && (listing.followers ?? 0) > 0) return `${listing.followers} followers`;
  if (site.current_version !== undefined) return `v${site.current_version}`;
  return `Created ${moment(site.created_at)}`;
}

// ------------------------------------------------------------------ 起点

function EmptyHome({ me }: { me: Me | null }) {
  const [copied, setCopied] = useState(false);
  const command = "playtest ./dist";

  async function copy() {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // ignore
    }
  }

  const anonymous = me?.kind !== "github";

  return (
    <div class="works-empty-wrap">
      <Empty
        icon={
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
            <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
            <line x1="12" y1="22.08" x2="12" y2="12" />
          </svg>
        }
        title="No projects yet"
        description="Run one command in your project directory to create a playable build and collect feedback."
        action={
          <div class="empty-home-actions">
            <button class="copy-pill" type="button" onClick={copy} aria-label={`Copy command ${command}`}>
              <code>$ {command}</code>
              <span>{copied ? "Copied!" : "Copy"}</span>
            </button>
            <a class="button small primary" href={href({ name: "docs", section: "start" })}>
              Quickstart Guide →
            </a>
          </div>
        }
      >
        {anonymous ? (
          <p class="empty-anon-hint">
            Anonymous session. Sign in anytime to preserve projects permanently.
          </p>
        ) : null}
      </Empty>
    </div>
  );
}
