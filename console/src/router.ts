// 三种地址，一个哈希路由：作品墙、某个作品的某一页、账号。
//
// 用哈希不用 history：这个站构建出来是一堆静态文件，哈希路由不需要托管方额外配
// 「所有路径都回 index.html」的重写规则，少一处会坏的地方。地址仍然可以直接分享。
//
// 作品页的分页签是地址的一部分（DESIGN §3.13）：刷新不丢、能把「这一版的点名册」直接发给同伴。

import { useEffect, useState } from "preact/hooks";

/** 作品页的五个分页签。 */
export type Tab = "results" | "roster" | "feedback" | "card" | "settings";

export type Route =
  | { name: "sites" }
  | { name: "site"; slug: string; tab: Tab; version?: number }
  | { name: "token" };

const TABS: readonly Tab[] = ["results", "roster", "feedback", "card", "settings"];

function isTab(value: string | undefined): value is Tab {
  return TABS.includes(value as Tab);
}

export function parse(hash: string): Route {
  const path = hash.replace(/^#/, "").replace(/^\/+/, "");
  const parts = path.split("/").filter(Boolean).map(decodeURIComponent);

  if (parts[0] === "token") return { name: "token" };
  if (parts[0] === "s" && parts[1]) {
    const slug = parts[1];
    // 旧地址 #/s/<slug>/v/<n> 是这一版的点名册，照样认。
    if (parts[2] === "v" && parts[3]) {
      const version = positive(parts[3]);
      return version ? { name: "site", slug, tab: "roster", version } : { name: "site", slug, tab: "results" };
    }
    if (parts[2] === "roster") {
      const version = positive(parts[3]);
      return version ? { name: "site", slug, tab: "roster", version } : { name: "site", slug, tab: "roster" };
    }
    if (isTab(parts[2])) return { name: "site", slug, tab: parts[2] };
    return { name: "site", slug, tab: "results" };
  }
  return { name: "sites" };
}

function positive(text: string | undefined): number | undefined {
  const value = Number(text);
  return Number.isInteger(value) && value > 0 ? value : undefined;
}

export function href(route: Route): string {
  switch (route.name) {
    case "sites":
      return "#/";
    case "token":
      return "#/token";
    case "site": {
      const base = `#/s/${encodeURIComponent(route.slug)}`;
      if (route.tab === "results") return base;
      if (route.tab === "roster") {
        return route.version ? `${base}/roster/${route.version}` : `${base}/roster`;
      }
      return `${base}/${route.tab}`;
    }
  }
}

export function go(route: Route): void {
  location.hash = href(route);
}

export function useRoute(): Route {
  const [route, setRoute] = useState<Route>(() => parse(location.hash));
  useEffect(() => {
    const onChange = () => setRoute(parse(location.hash));
    addEventListener("hashchange", onChange);
    return () => removeEventListener("hashchange", onChange);
  }, []);
  return route;
}
