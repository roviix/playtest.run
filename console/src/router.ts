// 四个页面，一个哈希路由。
//
// 用哈希不用 history：这个站构建出来是一堆静态文件，哈希路由不需要托管方额外配
// 「所有路径都回 index.html」的重写规则，少一处会坏的地方。地址仍然可以直接分享。

import { useEffect, useState } from "preact/hooks";

export type Route =
  | { name: "sites" }
  | { name: "timeline"; slug: string }
  | { name: "roster"; slug: string; version: number }
  | { name: "feedback"; slug: string }
  | { name: "token" };

export function parse(hash: string): Route {
  const path = hash.replace(/^#/, "").replace(/^\/+/, "");
  const parts = path.split("/").filter(Boolean).map(decodeURIComponent);

  if (parts[0] === "token") return { name: "token" };
  if (parts[0] === "s" && parts[1]) {
    const slug = parts[1];
    if (parts[2] === "v" && parts[3]) {
      const version = Number(parts[3]);
      if (Number.isInteger(version) && version > 0) return { name: "roster", slug, version };
    }
    if (parts[2] === "feedback") return { name: "feedback", slug };
    return { name: "timeline", slug };
  }
  return { name: "sites" };
}

export function href(route: Route): string {
  switch (route.name) {
    case "sites":
      return "#/";
    case "token":
      return "#/token";
    case "timeline":
      return `#/s/${encodeURIComponent(route.slug)}`;
    case "roster":
      return `#/s/${encodeURIComponent(route.slug)}/v/${route.version}`;
    case "feedback":
      return `#/s/${encodeURIComponent(route.slug)}/feedback`;
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
