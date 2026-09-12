// 作品的窗：有封面就是封面，拿不到（还没画出来、边缘不在、断网）就退成字卡。
//
// 字卡的规则和广场卡（edge/src/plaza.rs）同一份：按 slug 定色的色田、作品名第一个字的花押、
// 右上角的 slug。破图标不是一种状态，所以图一失败就换成字卡（AGENTS 第 4 条：失败形态是设计出来的）。

import type { ComponentChildren } from "preact";
import { useState } from "preact/hooks";

import { coverUrl, type Site } from "../api";
import { hue, monogram } from "../hue";

export function Cover({ site, small, children }: { site: Site; small?: boolean; children?: ComponentChildren }) {
  const url =
    site.listing?.has_cover && site.current_version !== undefined
      ? coverUrl(site.url, site.current_version)
      : null;
  const [brokenUrl, setBrokenUrl] = useState<string | null>(null);

  return (
    <div class={`shot ${small ? "small" : ""}`} style={`--h:${hue(site.slug)};view-transition-name:cover-${Array.from(site.slug).map((char) => char.codePointAt(0)!.toString(16)).join("-")}`}>
      {url && brokenUrl !== url ? (
        <img class="cover" src={url} alt="" loading={small ? "eager" : "lazy"} onError={() => setBrokenUrl(url)} />
      ) : (
        <div class="cover word">
          <b>{monogram(site.title, site.slug)}</b>
          <i>{site.slug}</i>
        </div>
      )}
      {children}
    </div>
  );
}
