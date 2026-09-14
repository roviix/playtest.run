// 结果（DESIGN §3.13）：每版一块，数字先、句子后、来源在末、和上一版比收尾。
//
// 四个大数字不是图表：它把 §3.5 那段话里最要紧的几个数从行文里提出来，仍然是计数，不是比例。
// 0 不排大，退成灰字——0 占着地方却什么也没说。句子那部分仍由 words.ts 生成，措辞集中在那里。

import { useState } from "preact/hooks";

import { api, type Site, type SiteResults, type VersionResults } from "../api";
import type { Loaded } from "../load";
import { href } from "../router";
import { moment, sentences, versusDeltas } from "../words";
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
    if (!confirm(`Set v${version} as current? Link stays the same.`)) return;
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
      <Empty
        icon={
          <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="20" x2="18" y2="10" />
            <line x1="12" y1="20" x2="12" y2="4" />
            <line x1="6" y1="20" x2="6" y2="14" />
          </svg>
        }
        title="No build versions yet"
        description="Run playtest ./dist in your project directory to publish and begin tracking playthrough metrics."
      />
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
                {isCurrent ? <span class="chip good">Current</span> : null}
                <span class="muted">{moment(version.created_at ?? version.first_at)}</span>
                {version.note ? <span class="ver-note">“{version.note}”</span> : null}
              </header>

              <Figures version={version} kind={site.kind} />

              <div class="says-block">
                {sentences(version, site.kind).map((line) => (
                  <p key={line} class="says">
                    {line}
                  </p>
                ))}
                {previous ? <Versus version={version} previous={previous} /> : null}
              </div>

              <p class="row-actions">
                <a href={href({ name: "site", slug, tab: "roster", version: version.version })}>
                  Roster →
                </a>
                {version.feedback_count > 0 ? (
                  <a href={href({ name: "site", slug, tab: "feedback" })}>Feedback →</a>
                ) : null}
                {!isCurrent ? (
                  <button
                    class="button small"
                    type="button"
                    disabled={busy !== null}
                    onClick={() => rollbackTo(version.version)}
                  >
                    {busy === version.version ? "Switching…" : "Set as Current"}
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

function Figures({ version, kind }: { version: VersionResults; kind: Site["kind"] }) {
  if (kind === "article" || kind === "video") {
    return (
      <div class="figures">
        <Figure n={version.opened} label="Page Views" />
        <Figure n={version.returned ?? 0} label="Returned" />
        <Figure n={version.feedback_count} label="Feedback" />
      </div>
    );
  }
  const dropped = version.dropped_before_first_frame;
  const entered =
    dropped === null || dropped === undefined
      ? { n: version.entered, label: "Clicked Start" }
      : { n: Math.max(version.entered - dropped, 0), label: "Entered Game" };
  return (
    <div class="figures">
      <Figure n={version.opened} label="Opened" />
      <Figure n={entered.n} label={entered.label} />
      <Figure n={version.played_5min_plus} label="5m+ Played" />
      <Figure n={version.feedback_count} label="Feedback" />
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
  const res = versusDeltas(version, previous);
  if (!res) return null;
  return (
    <div class="versus" aria-label={`Compared with v${res.previousVersion}`}>
      <span class="versus-lead">vs v{res.previousVersion}:</span>
      <div class="versus-pills">
        {res.deltas.map((delta, i) => (
          <span key={i} class={`versus-pill ${delta.kind}`}>
            {delta.label}
          </span>
        ))}
      </div>
    </div>
  );
}
