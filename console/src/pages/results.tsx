// 结果（DESIGN §3.13）：每版一块，数字先、句子后、来源在末、和上一版比收尾。
//
// 四个大数字不是图表：它把 §3.5 那段话里最要紧的几个数从行文里提出来，仍然是计数，不是比例。
// 0 不排大，退成灰字——0 占着地方却什么也没说。句子那部分仍由 words.ts 生成，措辞集中在那里。

import { useState } from "preact/hooks";

import { api, type Site, type SiteResults, type VersionResults } from "../api";
import type { Loaded } from "../load";
import { href } from "../router";
import { moment, sentences, versus } from "../words";
import { Empty, Failed, Loading } from "./status";

export function ResultsTab({
  slug,
  site,
  results,
  onVersionsChanged,
}: {
  slug: string;
  site: Site;
  results: Loaded<SiteResults>;
  onVersionsChanged: () => void;
}) {
  const [busy, setBusy] = useState<number | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  // 回滚只动「当前版本」指针，不删任何版本；确认一句是因为玩家那边会立刻变。
  async function rollbackTo(version: number) {
    if (!confirm(`让玩家看到的换回 v${version}？链接不变，点开就是那一版。`)) return;
    setBusy(version);
    setProblem(null);
    try {
      await api.activateVersion(slug, version);
      onVersionsChanged();
    } catch (e) {
      setProblem(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  }

  if (results.loading) return <Loading />;
  if (results.error) return <Failed error={results.error} onRetry={results.reload} />;
  const data = results.data;
  if (!data) return null;

  if (data.versions.length === 0) {
    return (
      <Empty>
        <p>这个作品还没有版本。</p>
        <p class="muted">在作品目录里运行 playtest，上传完这里就会出现第一块。</p>
      </Empty>
    );
  }

  return (
    <>
      {problem ? <p class="notice">{problem}</p> : null}
      <ol class="versions">
        {data.versions.map((version, index) => {
          const previous = data.versions[index + 1];
          const isCurrent = version.version === site.current_version;
          return (
            <li key={version.version} class={`ver ${isCurrent ? "current" : ""}`}>
              <header class="ver-head">
                <span class="ver-no mono">v{version.version}</span>
                {isCurrent ? <span class="chip good">玩家看到的</span> : null}
                <span class="muted">{moment(version.created_at ?? version.first_at)}</span>
                {version.note ? <span class="ver-note">「{version.note}」</span> : null}
              </header>

              <Figures version={version} />

              <div class="says-block">
                {sentences(version).map((line) => (
                  <p key={line} class="says">
                    {line}
                  </p>
                ))}
                {previous ? <Versus version={version} previous={previous} /> : null}
              </div>

              <p class="row-actions">
                <a href={href({ name: "site", slug, tab: "roster", version: version.version })}>
                  {version.opened > 0 ? `看这 ${version.opened} 个人 →` : "点名册 →"}
                </a>
                {version.feedback_count > 0 ? (
                  <a href={href({ name: "site", slug, tab: "feedback" })}>看反馈 →</a>
                ) : null}
                {!isCurrent ? (
                  <button
                    class="button small"
                    type="button"
                    disabled={busy !== null}
                    onClick={() => rollbackTo(version.version)}
                  >
                    {busy === version.version ? "正在换…" : "让玩家看这一版"}
                  </button>
                ) : null}
              </p>
            </li>
          );
        })}
      </ol>
    </>
  );
}

/**
 * 四个数：打开 / 进到游戏 / 5 分钟以上 / 反馈。
 * 没接 SDK 的版本报不出首帧，第二格退成「点了开始」——不拿「0 个人在加载时走了」冒充。
 */
function Figures({ version }: { version: VersionResults }) {
  const dropped = version.dropped_before_first_frame;
  const entered =
    dropped === null || dropped === undefined
      ? { n: version.entered, label: "点了开始" }
      : { n: Math.max(version.entered - dropped, 0), label: "进到游戏" };
  return (
    <div class="figures">
      <Figure n={version.opened} label="打开" />
      <Figure n={entered.n} label={entered.label} />
      <Figure n={version.played_5min_plus} label="5 分钟以上" />
      <Figure n={version.feedback_count} label="反馈" />
    </div>
  );
}

function Figure({ n, label }: { n: number; label: string }) {
  return (
    <div class={`figure ${n === 0 ? "zero" : ""}`}>
      <span class="figure-n mono">{n}</span>
      <span class="figure-label">{label}</span>
    </div>
  );
}

function Versus({ version, previous }: { version: VersionResults; previous: VersionResults }) {
  const text = versus(version, previous);
  return text ? <p class="versus">{text}</p> : null;
}
