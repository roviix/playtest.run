// 结果（DESIGN §3.13）：每版一块，紧凑克制，层次分明。
//
// 四个大数字是度量磁贴，带上一版增减；分段流转轨直观呈现转化流向，不画伪造图表；
// 来源与留存采用紧凑胶囊收纳；错误堆栈收拢于诊断卡片，可按需展开，不污染正常统计。

import { useState } from "preact/hooks";

import { api, type Site, type SiteResults, type VersionResults } from "../api";
import type { Loaded } from "../load";
import { href } from "../router";
import { clip, moment, seconds, sourceLabel } from "../words";
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
        title="No versions yet"
        description="Run playtest ./dist to publish the first one."
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
            <li key={version.version} class={`ver ${isCurrent ? "current" : "history"}`}>
              <header class="ver-head">
                <span class="ver-no mono">v{version.version}</span>
                {isCurrent ? <span class="chip good">Current</span> : null}
                <span class="ver-when muted">{moment(version.created_at ?? version.first_at)}</span>
                {version.note ? <span class="ver-note">“{version.note}”</span> : null}
                {previous ? <span class="ver-vs muted mono">vs v{previous.version}</span> : null}
              </header>

              <Figures version={version} previous={previous} kind={site.kind} />
              <FlowTrack version={version} kind={site.kind} slug={slug} />
              <ContextBar version={version} />
              <Diagnostics version={version} previous={previous} slug={slug} />

              <p class="row-actions">
                <a href={href({ name: "site", slug, tab: "roster", version: version.version })}>
                  Roster ({version.opened}) →
                </a>
                {version.feedback_count > 0 ? (
                  <a href={href({ name: "site", slug, tab: "feedback" })}>
                    Feedback ({version.feedback_count}) →
                  </a>
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

/** 进到游戏的人数：报过首帧的版本减掉加载时掉的；没接 SDK 就是点了「开始」的人数。 */
function enteredOf(version: VersionResults): number {
  const dropped = version.dropped_before_first_frame;
  return dropped == null ? version.entered : Math.max(version.entered - dropped, 0);
}

// ------------------------------------------------------------------ 核心度量磁贴

function Figures({ version, previous, kind }: { version: VersionResults; previous?: VersionResults; kind: Site["kind"] }) {
  const delta = (pick: (one: VersionResults) => number) => (previous ? pick(version) - pick(previous) : undefined);

  if (kind === "article" || kind === "video") {
    return (
      <div class="figures">
        <Figure n={version.opened} label="Page Views" delta={delta((one) => one.opened)} />
        <Figure n={version.returned ?? 0} label="Returned" delta={delta((one) => one.returned ?? 0)} />
        <Figure n={version.feedback_count} label="Feedback" delta={delta((one) => one.feedback_count)} />
      </div>
    );
  }

  const entered = enteredOf(version);
  const dwellNote = version.dwell_median_s != null ? `${seconds(version.dwell_median_s)} median` : undefined;

  return (
    <div class="figures">
      <Figure n={version.opened} label="Opened" delta={delta((one) => one.opened)} />
      <Figure
        n={entered}
        label="Entered"
        delta={delta(enteredOf)}
      />
      <Figure
        n={version.played_5min_plus}
        label="Played 5m+"
        note={dwellNote}
        delta={delta((one) => one.played_5min_plus)}
      />
      <Figure
        n={version.feedback_count}
        label="Feedback"
        delta={delta((one) => one.feedback_count)}
      />
    </div>
  );
}

function Figure({
  n,
  label,
  note,
  noteTone,
  delta,
}: {
  n: number;
  label: string;
  note?: string;
  noteTone?: "warn";
  delta?: number;
}) {
  return (
    <div class={`figure ${n === 0 ? "zero" : ""}`}>
      <div class="figure-row">
        <span class="figure-n mono">{n}</span>
        {delta ? (
          <span class={`figure-delta mono ${delta > 0 ? "up" : "down"}`}>
            {delta > 0 ? `+${delta}` : `−${Math.abs(delta)}`}
          </span>
        ) : null}
      </div>
      <span class="figure-label">{label}</span>
      {note ? <span class={`figure-note ${noteTone ?? ""}`}>{note}</span> : null}
    </div>
  );
}

// ------------------------------------------------------------------ 分段流转条 (Playthrough Flow Track)

function FlowTrack({ version, kind, slug }: { version: VersionResults; kind: Site["kind"]; slug: string }) {
  const total = version.opened;
  if (total === 0) return null;

  if (kind === "article" || kind === "video") {
    const returned = Math.min(version.returned ?? 0, total);
    const once = Math.max(0, total - returned);
    return (
      <div class="flow-panel">
        <a class="flow-track" href={href({ name: "site", slug, tab: "roster", version: version.version })} title="View in roster">
          <div class="flow-seg deep" style={{ flex: returned }} />
          <div class="flow-seg in" style={{ flex: once }} />
        </a>
        <div class="flow-legend">
          <span class="flow-legend-item"><i class="flow-dot deep" /><b>{returned}</b> returned</span>
          <span class="flow-legend-item"><i class="flow-dot in" /><b>{once}</b> visited once</span>
        </div>
      </div>
    );
  }

  const dropped = version.dropped_before_first_frame;
  const tracked = dropped != null;
  const entered = Math.min(enteredOf(version), total);
  const deep = Math.min(version.played_5min_plus, entered);
  const active = Math.max(0, entered - deep);
  const fell = tracked ? Math.min(dropped, total - entered) : 0;
  const bounced = Math.max(0, total - entered - fell);

  return (
    <div class="flow-panel">
      <a
        class="flow-track"
        href={href({ name: "site", slug, tab: "roster", version: version.version })}
        aria-label={`${total} visitors: ${deep} played 5m+, ${active} entered, ${fell} dropped, ${bounced} not started`}
        title="Inspect in roster"
      >
        {deep > 0 ? <div class="flow-seg deep" style={{ flex: deep }} /> : null}
        {active > 0 ? <div class="flow-seg in" style={{ flex: active }} /> : null}
        {fell > 0 ? <div class="flow-seg dropped" style={{ flex: fell }} /> : null}
        {bounced > 0 ? <div class="flow-seg bounced" style={{ flex: bounced }} /> : null}
      </a>
      <div class="flow-legend">
        {deep > 0 ? (
          <span class="flow-legend-item"><i class="flow-dot deep" /><b>{deep}</b> played 5m+</span>
        ) : null}
        {active > 0 ? (
          <span class="flow-legend-item"><i class="flow-dot in" /><b>{active}</b> entered game</span>
        ) : null}
        {fell > 0 ? (
          <span class="flow-legend-item warn"><i class="flow-dot dropped" /><b>{fell}</b> dropped in load</span>
        ) : null}
        {bounced > 0 ? (
          <span class="flow-legend-item muted"><i class="flow-dot bounced" /><b>{bounced}</b> did not start</span>
        ) : null}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ 事实与来源胶囊

function ContextBar({ version }: { version: VersionResults }) {
  const sources = (version.sources ?? []).filter((s) => s.count > 0);
  const named = version.named ?? 0;
  const returned = version.returned ?? 0;
  const hasSources = sources.length > 0;
  const hasRetention = named > 0 || returned > 0;

  if (!hasSources && !hasRetention) return null;

  return (
    <div class="context-bar">
      {named > 0 ? <span class="context-pill"><b>{named}</b> left name</span> : null}
      {returned > 0 ? <span class="context-pill"><b>{returned}</b> returned</span> : null}
      {hasSources ? (
        <span class="context-sources">
          <span class="context-lead">From</span>
          {sources.map((s) => (
            <span key={s.kind} class="context-source-tag">
              <b>{s.count}</b> {sourceLabel(s.kind)}
            </span>
          ))}
        </span>
      ) : null}
    </div>
  );
}

// ------------------------------------------------------------------ 阻力与异常诊断 (Diagnostics)

function Diagnostics({ version, previous, slug }: { version: VersionResults; previous?: VersionResults; slug: string }) {
  const [open, setOpen] = useState(false);
  const errors = version.errors;
  const loadFailures = version.load_failures;
  const untracked = version.dropped_before_first_frame == null;

  const hasErrors = errors.total > 0;
  const hasFailures = loadFailures > 0;
  const hadErrorsBefore = (previous?.errors.total ?? 0) > 0;
  const resolved = !hasErrors && hadErrorsBefore;

  if (!hasErrors && !hasFailures && !resolved && !untracked) return null;

  const top = errors.top?.[0];

  return (
    <div class="diagnostics">
      {hasErrors ? (
        <div class="diag-card warn">
          <div
            class="diag-head"
            onClick={() => setOpen(!open)}
            role="button"
            aria-expanded={open}
            tabIndex={0}
          >
            <div class="diag-summary">
              <span class="diag-dot warn" />
              <span class="diag-count"><b>{errors.total}</b> {errors.total === 1 ? "error hit" : "error hits"}</span>
              {previous && previous.errors.total !== errors.total ? (
                <span class="diag-was mono">was {previous.errors.total}</span>
              ) : null}
              {top ? <span class="diag-snippet mono">{clip(top.fingerprint, 38)}</span> : null}
            </div>
            <span class="diag-toggle">{open ? "Hide stack" : "View stack"}</span>
          </div>

          {open ? (
            <div class="diag-body">
              <ol class="diag-list">
                {(errors.top ?? []).map((err, i) => (
                  <li key={i} class="diag-item">
                    <span class="diag-item-count mono">×{err.count}</span>
                    <code class="mono">{err.fingerprint}</code>
                  </li>
                ))}
              </ol>
              <p class="diag-actions">
                <a href={href({ name: "site", slug, tab: "roster", version: version.version })}>
                  Inspect sessions in Roster →
                </a>
              </p>
            </div>
          ) : null}
        </div>
      ) : resolved ? (
        <div class="diag-card resolved">
          <span class="diag-dot good" />
          <span><b>0</b> errors</span>
          <span class="diag-was mono">resolved (was {previous?.errors.total} in v{previous?.version})</span>
        </div>
      ) : null}

      {hasFailures ? (
        <div class="diag-card fail">
          <span class="diag-dot warn" />
          <span><b>{loadFailures}</b> {loadFailures === 1 ? "resource failed to load" : "resources failed to load"}</span>
          {previous && previous.load_failures !== loadFailures ? (
            <span class="diag-was mono">was {previous.load_failures}</span>
          ) : null}
        </div>
      ) : null}

      {untracked ? (
        <div class="diag-card untracked">
          <span class="diag-dot idle" />
          <span>No playtest.js: first frame, drop-offs and errors unknown</span>
        </div>
      ) : null}
    </div>
  );
}
