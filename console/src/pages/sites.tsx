// 作品列表。进来第一眼看到的东西，一行一个作品，点进去是时间线。
//
// 广场（DESIGN §3.8）的开关也在这里：放不放到广场上、要不要标「正在找人测」、想让人看什么。
// 被举报撤下的作品要在这里明说，不能让开发者以为自己还在广场上（AGENTS 第 4 条）。

import { useState } from "preact/hooks";

import { api, type Listing, type Site } from "../api";
import { useLoad } from "../load";
import { href } from "../router";
import { moment } from "../words";
import { Empty, Failed, Loading } from "./status";

const SEEK_NOTE_MAX = 140;

export function SitesPage() {
  const { data, error, loading, reload } = useLoad(() => api.sites(), []);

  if (loading) return <Loading />;
  if (error) return <Failed error={error} onRetry={reload} />;
  if (!data || data.length === 0) {
    return (
      <Empty>
        <p>这个令牌名下还没有作品。</p>
        <p class="muted">在作品目录里运行 playtest，链接和第一版会一起出来。</p>
      </Empty>
    );
  }

  return (
    <>
      <header class="page-head">
        <h1>作品</h1>
        <p class="muted">每个作品一条链接，永远指向它最新的版本。</p>
      </header>
      <ul class="cards">
        {data.map((site) => (
          <SiteCard key={site.slug} site={site} />
        ))}
      </ul>
    </>
  );
}

function SiteCard({ site: initial }: { site: Site }) {
  const [site, setSite] = useState(initial);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const listing: Listing = site.listing ?? {
    public: false,
    seeking: false,
    hidden: false,
    has_cover: false,
  };
  const plazaUrl = plazaOf(site.url);

  async function change(request: Parameters<typeof api.updateSite>[1]) {
    setBusy(true);
    setProblem(null);
    try {
      setSite(await api.updateSite(site.slug, request));
    } catch (e) {
      setProblem(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <li class="card site">
      <div class="site-head">
        <div class="site-title-block">
          <a class="card-title" href={href({ name: "timeline", slug: site.slug })}>
            {site.title}
          </a>
          <p class="muted">
            <code>{site.slug}</code>
            {" · "}
            {site.current_version ? `最新 v${site.current_version}` : "还没上传过版本"}
            {" · "}
            建于 {moment(site.created_at)}
          </p>
        </div>
        <PlazaBadge listing={listing} />
      </div>

      {listing.summary ? <p class="site-summary">{listing.summary}</p> : null}

      <p class="row-actions">
        <a href={href({ name: "timeline", slug: site.slug })}>看结果</a>
        <a href={href({ name: "feedback", slug: site.slug })}>反馈</a>
        <a href={site.url} target="_blank" rel="noreferrer">
          玩家看到的链接
        </a>
      </p>

      <PlazaControls
        listing={listing}
        busy={busy}
        canPublish={site.current_version !== undefined}
        plazaUrl={plazaUrl}
        onChange={change}
      />

      {problem ? <p class="notice">{problem}</p> : null}
      {site.expires_at ? (
        <p class="muted">这个链接 {moment(site.expires_at)} 到期，到期自动从广场上下来。</p>
      ) : null}
    </li>
  );
}

function PlazaBadge({ listing }: { listing: Listing }) {
  if (!listing.public) return null;
  if (listing.hidden) return <span class="tag warn">已从广场撤下</span>;
  if (listing.seeking) return <span class="tag accent">广场 · 正在找人测</span>;
  return <span class="tag good">在广场上</span>;
}

function PlazaControls({
  listing,
  busy,
  canPublish,
  plazaUrl,
  onChange,
}: {
  listing: Listing;
  busy: boolean;
  canPublish: boolean;
  plazaUrl: string;
  onChange: (request: Parameters<typeof api.updateSite>[1]) => Promise<void>;
}) {
  const [note, setNote] = useState(listing.seek_note ?? "");
  const [editing, setEditing] = useState(false);

  if (!canPublish) {
    return <p class="muted">先发一版，才能放到广场上。</p>;
  }

  if (!listing.public) {
    return (
      <div class="plaza">
        <p class="muted">
          放到 <a href={plazaUrl} target="_blank" rel="noreferrer">广场</a> 上，路过的人点开就能玩；
          来的人玩成什么样，点名册里看得见。默认不放。
        </p>
        <p class="row-actions">
          <button class="button" type="button" disabled={busy} onClick={() => onChange({ public: true })}>
            放到广场上
          </button>
          <button
            class="button primary"
            type="button"
            disabled={busy}
            onClick={() => {
              setEditing(true);
              void onChange({ seeking: true });
            }}
          >
            放上去并标「正在找人测」
          </button>
        </p>
      </div>
    );
  }

  return (
    <div class="plaza">
      {listing.hidden ? (
        <p class="notice">
          这个作品 24 小时内被多人举报，已经从广场上撤下，等我们复核。它的链接照常能开，你发出去的人不受影响。
        </p>
      ) : (
        <p class="muted">
          在 <a href={plazaUrl} target="_blank" rel="noreferrer">广场</a> 上
          {listing.seeking ? "，标着「正在找人测」" : ""}。
          {listing.has_cover ? "" : " 还没有封面——上传时加 --cover 一张图，卡片会好看得多。"}
        </p>
      )}

      {listing.seeking && !editing ? (
        <p class="seek-note">
          <b>想让人看：</b>
          {listing.seek_note ? listing.seek_note : <span class="muted">（还没写）</span>}
          <button class="linkish" type="button" onClick={() => setEditing(true)}>
            改
          </button>
        </p>
      ) : null}

      {editing ? (
        <form
          class="seek-form"
          onSubmit={(event) => {
            event.preventDefault();
            void onChange({ seeking: true, seek_note: note.trim() }).then(() => setEditing(false));
          }}
        >
          <label class="field">
            <span>想让来的人重点看什么（{note.length}/{SEEK_NOTE_MAX}）</span>
            <input
              value={note}
              maxLength={SEEK_NOTE_MAX}
              placeholder="例如：新手引导看得懂吗？第三关难不难？"
              onInput={(event) => setNote((event.target as HTMLInputElement).value)}
            />
          </label>
          <p class="row-actions">
            <button class="button primary" type="submit" disabled={busy}>
              保存
            </button>
            <button class="button" type="button" onClick={() => setEditing(false)}>
              取消
            </button>
          </p>
        </form>
      ) : null}

      <p class="row-actions">
        {listing.seeking ? (
          <button class="button" type="button" disabled={busy} onClick={() => onChange({ seeking: false })}>
            人找够了
          </button>
        ) : (
          <button
            class="button"
            type="button"
            disabled={busy}
            onClick={() => {
              setEditing(true);
              void onChange({ seeking: true });
            }}
          >
            标「正在找人测」
          </button>
        )}
        <button class="button quiet" type="button" disabled={busy} onClick={() => onChange({ public: false })}>
          从广场上拿下来
        </button>
      </p>
    </div>
  );
}

/** 玩家链接去掉 slug 那一级就是广场：`https://brisk-otter-41.playtest.run` → `https://playtest.run/`。 */
function plazaOf(siteUrl: string): string {
  try {
    const url = new URL(siteUrl);
    const host = url.host.split(".").slice(1).join(".");
    return `${url.protocol}//${host}/`;
  } catch {
    return "/";
  }
}
