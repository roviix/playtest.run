// 作品时间线：每版一张卡，卡上是一段话（DESIGN §3.4）。
//
// 这里一个图表都没有，也不会有：5–50 个点的柱状图只会显得空（DESIGN §3.7）。
// 卡上每一句都能点进去看到具体是哪些人。

import { api } from "../api";
import { useLoad } from "../load";
import { href } from "../router";
import { moment, sentences, versus } from "../words";
import { Empty, Failed, Loading } from "./status";

export function TimelinePage({ slug }: { slug: string }) {
  const { data, error, loading, reload } = useLoad(() => api.results(slug), [slug]);

  if (loading) return <Loading />;
  if (error) return <Failed error={error} onRetry={reload} />;
  if (!data) return null;

  return (
    <>
      <header class="page-head">
        <h1>{data.title}</h1>
        <p class="muted">
          <code>{slug}</code>
          {data.current_version ? ` · 最新 v${data.current_version}` : ""}
        </p>
        <p class="row-actions">
          <a href={href({ name: "feedback", slug })}>反馈流</a>
        </p>
      </header>

      {data.versions.length === 0 ? (
        <Empty>
          <p>这个作品还没有版本。</p>
          <p class="muted">在作品目录里运行 playtest，上传完这里就会出现第一张卡。</p>
        </Empty>
      ) : (
        <ul class="cards">
          {data.versions.map((version, index) => {
            const previous = data.versions[index + 1];
            const diff = previous ? versus(version, previous) : null;
            return (
              <li key={version.version} class="card">
                <p class="card-head">
                  <strong>v{version.version}</strong>
                  {version.created_at || version.first_at ? (
                    <span class="muted"> · {moment(version.created_at ?? version.first_at)}</span>
                  ) : null}
                  {version.note ? <span> · 「{version.note}」</span> : null}
                </p>

                {sentences(version).map((line) => (
                  <p key={line} class="says">
                    {line}
                  </p>
                ))}

                {diff ? <p class="versus">{diff}</p> : null}

                <p class="row-actions">
                  <a href={href({ name: "roster", slug, version: version.version })}>
                    {version.opened > 0 ? `看这 ${version.opened} 个人` : "点名册"}
                  </a>
                  {version.feedback_count > 0 ? (
                    <a href={href({ name: "feedback", slug })}>看反馈</a>
                  ) : null}
                </p>
              </li>
            );
          })}
        </ul>
      )}
    </>
  );
}
