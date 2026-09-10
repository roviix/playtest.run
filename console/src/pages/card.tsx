// 邀请卡（DESIGN §3.4、§3.13）：发出去之前最后一眼，所以是单独一页，不塞进设置里。
//
// 两张：竖版发群里，横版做链接预览。都由边缘按当前版本现画，这里只是放大到能看清字。
// 跨源的 <a download> 会被浏览器忽略（卡在作品域上，控制台在开发者域），所以按钮是新标签页打开，
// 说明里写「长按 / 右键存图」——不假装能一键保存。

import { useState } from "preact/hooks";

import { cardUrl, cardWideUrl, type Site } from "../api";
import { Empty } from "./status";

export function CardTab({ site }: { site: Site }) {
  const [copied, setCopied] = useState<string | null>(null);

  if (site.current_version === undefined) {
    return (
      <Empty>
        <p>上传第一版之后，这里就有一张可以发到群里的卡。</p>
      </Empty>
    );
  }

  // 卡随版本重画，地址却是同一个，所以带上版本号绕开浏览器缓存。
  const tall = `${cardUrl(site.url)}?v=${site.current_version}`;
  const wide = `${cardWideUrl(site.url)}?v=${site.current_version}`;
  const canShare = typeof navigator !== "undefined" && typeof navigator.share === "function";

  async function copyLink() {
    try {
      await navigator.clipboard.writeText(site.url);
      setCopied("已复制");
    } catch {
      setCopied(site.url);
    }
    setTimeout(() => setCopied(null), 2500);
  }

  async function share() {
    try {
      await navigator.share({ title: site.title, url: site.url });
    } catch {
      // 用户取消了，或者浏览器不让——都不算错。
    }
  }

  return (
    <div class="cards-page">
      <div class="cards-row">
        <CardImage src={tall} ratio="1080 / 1350" caption="竖版 · 发到群里，别人长按识别二维码就能玩" />
        <CardImage src={wide} ratio="1200 / 630" caption="横版 · 链接在 Discord、Telegram、iMessage 里展开时的预览" />
      </div>
      <p class="row-actions">
        <a class="button" href={tall} target="_blank" rel="noreferrer">
          打开竖版
        </a>
        <a class="button" href={wide} target="_blank" rel="noreferrer">
          打开横版
        </a>
        <button class="button" type="button" onClick={copyLink}>
          {copied ?? "复制链接"}
        </button>
        {canShare ? (
          <button class="button primary" type="button" onClick={share}>
            分享
          </button>
        ) : null}
      </p>
      <p class="muted">
        打开之后右键或长按存图。卡上写的是 {site.listing?.seats ? "当前名额进度和" : ""}
        v{site.current_version}，发新版本后会自己重画。
        {site.listing?.public ? "" : " 这个作品没公开，卡上不出现「分享」。"}
      </p>
    </div>
  );
}

function CardImage({ src, ratio, caption }: { src: string; ratio: string; caption: string }) {
  const [broken, setBroken] = useState(false);
  return (
    <figure class="card-figure">
      {broken ? (
        <div class="card-image missing" style={`aspect-ratio:${ratio}`}>
          <p class="muted">邀请卡稍后可拿。它由边缘按当前版本现画，刷新一下多半就有了。</p>
        </div>
      ) : (
        <img
          class="card-image"
          style={`aspect-ratio:${ratio}`}
          src={src}
          alt="邀请卡"
          onError={() => setBroken(true)}
        />
      )}
      <figcaption class="muted">{caption}</figcaption>
    </figure>
  );
}
