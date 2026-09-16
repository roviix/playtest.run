// 邀请卡（DESIGN §3.4、§3.13）：发出去之前最后一眼，所以是单独一页，不塞进设置里。
//
// 两张：竖版发群里，横版做链接预览。都由边缘按当前版本现画，这里只是放大到能看清字。
// 跨源的 <a download> 会被浏览器忽略（卡在作品域上，控制台在开发者域），所以按钮是新标签页打开，
// 说明里写「长按 / 右键存图」——不假装能一键保存。

import { useState } from "preact/hooks";

import { cardUrl, cardWideUrl, doorUrl, type Site } from "../api";
import { Empty } from "./status";

export function CardTab({ site }: { site: Site }) {
  const [copied, setCopied] = useState<string | null>(null);

  if (site.current_version === undefined) {
    return (
      <Empty
        icon={
          <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
            <circle cx="8.5" cy="8.5" r="1.5" />
            <polyline points="21 15 16 10 5 21" />
          </svg>
        }
        title="No card yet"
        description="Cards are drawn from the first published version."
      />
    );
  }

  const tall = `${cardUrl(site.url)}?v=${site.current_version}`;
  const wide = `${cardWideUrl(site.url)}?v=${site.current_version}`;
  const canShare = typeof navigator !== "undefined" && typeof navigator.share === "function";
  const link = doorUrl(site.url, site.slug);

  async function copyLink() {
    try {
      await navigator.clipboard.writeText(link);
      setCopied("Copied");
    } catch {
      setCopied(link);
    }
    setTimeout(() => setCopied(null), 2500);
  }

  async function share() {
    try {
      await navigator.share({ title: site.title, url: link });
    } catch {
      // User cancelled or browser rejected.
    }
  }

  return (
    <div class="cards-page">
      <div class="cards-row">
        <CardImage src={tall} ratio="1080 / 1350" caption="Portrait · chats" />
        <CardImage src={wide} ratio="1200 / 630" caption="Landscape · link preview" />
      </div>
      <p class="row-actions">
        <a class="button" href={tall} target="_blank" rel="noreferrer">
          Open Portrait
        </a>
        <a class="button" href={wide} target="_blank" rel="noreferrer">
          Open Landscape
        </a>
        <button class="button" type="button" onClick={copyLink}>
          {copied ?? "Copy Link"}
        </button>
        {canShare ? (
          <button class="button primary" type="button" onClick={share}>
            Share
          </button>
        ) : null}
      </p>
      <p class="muted">Right-click or long-press to save. Cards follow the current version.</p>
    </div>
  );
}

function CardImage({ src, ratio, caption }: { src: string; ratio: string; caption: string }) {
  const [broken, setBroken] = useState(false);
  return (
    <figure class="card-figure">
      {broken ? (
        <div class="card-image missing" style={`aspect-ratio:${ratio}`}>
          <p class="muted">Not ready yet. Refresh later.</p>
        </div>
      ) : (
        <img
          class="card-image"
          style={`aspect-ratio:${ratio}`}
          src={src}
          alt="Invite Card"
          onError={() => setBroken(true)}
        />
      )}
      <figcaption class="muted">{caption}</figcaption>
    </figure>
  );
}
