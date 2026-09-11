// 首页是作品墙（DESIGN §3.13）：一张卡一个作品，和广场上的卡是同一种物件。
//
// 卡右下只放一件事实，规则和广场卡（§3.9）同一份：名额 → 到期 → 关注 → 最新版本。
// 只有一个作品时直接进它——墙上只有一张卡时，墙本身没有信息。
// 没有作品时不是一句灰字，是一个设计好的起点：第一次登录的人面对的就是这一屏。

import { useEffect, useRef } from "preact/hooks";

import type { Me, Site } from "../api";
import type { Loaded } from "../load";
import { go, href } from "../router";
import { left, moment } from "../words";
import { Cover } from "./cover";
import { Failed, Loading } from "./status";

/** 单作品直进只在这一次页面加载里做一次：之后点「作品」回到墙上要停得住。 */
let enteredOnce = false;

export function HomePage({ sites, me }: { sites: Loaded<Site[]>; me: Me | null }) {
  const data = sites.data;
  const redirected = useRef(false);

  useEffect(() => {
    if (!data || data.length !== 1 || enteredOnce || redirected.current) return;
    enteredOnce = true;
    redirected.current = true;
    go({ name: "site", slug: data[0].slug, tab: "results" });
  }, [data]);

  if (sites.loading) return <Loading />;
  if (sites.error) return <Failed error={sites.error} onRetry={sites.reload} />;
  if (!data || data.length === 0) return <Start me={me} />;

  return (
    <>
      <header class="stage-head">
        <h1>作品</h1>
        <p class="muted">{data.length} 个</p>
      </header>
      <ul class="wall">
        {data.map((site) => (
          <li key={site.slug}>
            <Tile site={site} />
          </li>
        ))}
      </ul>
    </>
  );
}

function Tile({ site }: { site: Site }) {
  const tag = tagOf(site);

  return (
    <a class="tile" href={href({ name: "site", slug: site.slug, tab: "results" })}>
      <Cover site={site}>{tag ? <span class={`tag ${tag.tone}`}>{tag.text}</span> : null}</Cover>
      <div class="tile-body">
        <h2>{site.title}</h2>
        <p class="tile-slug mono">{site.slug}</p>
        <p class="meta">
          <span class="fact">{factOf(site)}</span>
          <span class="verb">打开 →</span>
        </p>
      </div>
    </a>
  );
}

/** 窗左上最多一个标：撤下 > 正在找人测 > 在广场上。私密作品没有标。 */
function tagOf(site: Site): { text: string; tone: string } | null {
  const listing = site.listing;
  if (!listing?.public) return null;
  if (listing.hidden) return { text: "已从广场撤下", tone: "warn" };
  if (listing.seeking) return { text: "正在找人测", tone: "" };
  return { text: "在广场上", tone: "plain" };
}

/** 卡上只出现一件事实。 */
function factOf(site: Site): string {
  const listing = site.listing;
  if (listing?.seats) return `${listing.joined} / ${listing.seats} 位`;
  const remaining = left(site.expires_at);
  if (remaining) return remaining;
  if (listing && listing.followers > 0) return `${listing.followers} 人关注`;
  if (site.current_version !== undefined) return `v${site.current_version}`;
  return `建于 ${moment(site.created_at)}`;
}

// ------------------------------------------------------------------ 起点

/**
 * 没有作品时的那一屏。三条命令、发出去之后这里会出现什么、CLI 输出里哪一行会把人带回来。
 * 发布在终端里完成，这里不放「上传」按钮（DESIGN §3.10）。
 */
function Start({ me }: { me: Me | null }) {
  const anonymous = me?.kind !== "github";
  return (
    <section class="start">
      <p class="start-kicker mono">在终端里</p>
      <h1>把能玩的版本发出去</h1>
      <p class="start-lead">在作品目录里跑一条命令。链接、二维码、邀请卡一起出来；谁来玩过，回到这里看。</p>
      <ol class="start-commands">
        <li>
          <code>playtest ./dist</code>
          <span>上传一个静态目录</span>
        </li>
        <li>
          <code>playtest 5173</code>
          <span>把本机端口开给别人</span>
        </li>
        <li>
          <code>playtest ./dist --public --seats 10</code>
          <span>放到广场上，找 10 位试玩</span>
        </li>
      </ol>
      {anonymous ? <p class="muted">现在是 24 小时匿名身份。用 GitHub 登录后作品不再到期。</p> : null}
    </section>
  );
}
