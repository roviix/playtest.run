// 作品列表。进来第一眼看到的东西，一行一个作品，点进去是时间线。

import { api } from "../api";
import { useLoad } from "../load";
import { href } from "../router";
import { moment } from "../words";
import { Empty, Failed, Loading } from "./status";

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
    <ul class="cards">
      {data.map((site) => (
        <li key={site.slug} class="card">
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
          <p class="row-actions">
            <a href={href({ name: "timeline", slug: site.slug })}>看结果</a>
            <a href={href({ name: "feedback", slug: site.slug })}>反馈</a>
            <a href={site.url} target="_blank" rel="noreferrer">
              玩家看到的链接
            </a>
          </p>
          {site.expires_at ? (
            <p class="muted">这个链接 {moment(site.expires_at)} 到期</p>
          ) : null}
        </li>
      ))}
    </ul>
  );
}
