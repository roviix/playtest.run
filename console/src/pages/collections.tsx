import { useEffect, useRef, useState } from "preact/hooks";
import { api, AUTH_REQUEST_EVENT, type Collection, type CollectionDraft, type EntryDraft, type Me, type Site } from "../api";
import { href } from "../router";
import { hue, monogram } from "../hue";
import { BrandMark } from "../auth";
import "../collections.css";

function message(error: unknown): string {
  return error instanceof Error ? error.message : "操作没有完成，请稍后再试。";
}

function collectionLink(slug?: string): string {
  return href({ name: "collections", slug });
}

type StateTone = { text: string; tone: "good" | "warn" | "neutral" | "soft"; pulse?: boolean };

function stateOf(collection: Collection): StateTone {
  if (collection.hidden) return { text: "已被移除公开展示", tone: "warn" };
  if (!collection.public) return { text: "草稿 · 仅自己可见", tone: "soft" };
  if (collection.closes_at && Date.parse(collection.closes_at) <= Date.now()) {
    return { text: "已结束 · 仍可观看", tone: "neutral" };
  }
  if (collection.kind === "challenge") {
    return { text: "开放投稿", tone: "good", pulse: true };
  }
  return { text: "公开作品集", tone: "good" };
}

export function CollectionsPage({
  slug,
  sites,
  me,
  plazaUrl,
}: {
  slug?: string;
  sites: Site[];
  me: Me | null;
  plazaUrl: string;
}) {
  const [mine, setMine] = useState<Collection[] | null>(null);
  const [available, setAvailable] = useState<Collection[]>([]);
  const [selected, setSelected] = useState<Collection | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [revision, setRevision] = useState(0);
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState(false);
  const [addingWork, setAddingWork] = useState(false);
  const [busy, setBusy] = useState(false);
  const [section, setSection] = useState<"mine" | "join">("mine");
  const [search, setSearch] = useState("");
  const [tab, setTab] = useState<"entries" | "prompt">("entries");
  const [copied, setCopied] = useState(false);
  const [promptCopied, setPromptCopied] = useState(false);

  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(""), 2800);
    return () => clearTimeout(timer);
  }, [notice]);

  useEffect(() => {
    let current = true;
    Promise.all([
      api.collections(),
      api.openCollections(),
      slug ? api.collection(slug) : Promise.resolve(null),
    ])
      .then(([owned, publicCollections, collection]) => {
        if (!current) return;
        setMine(owned);
        setAvailable(publicCollections);
        setSelected(collection);
        setError("");
      })
      .catch((failure) => {
        if (current) setError(message(failure));
      });
    return () => {
      current = false;
    };
  }, [slug, revision]);

  const refresh = () => setRevision((value) => value + 1);
  const owner = Boolean(selected && mine?.some((collection) => collection.slug === selected.slug));
  const memberSlugs = new Set(sites.map((site) => site.slug));
  const authenticated = me?.kind === "github";
  const closed = Boolean(selected?.closes_at && Date.parse(selected.closes_at) <= Date.now());

  async function act(action: () => Promise<unknown>, success: string) {
    if (busy) return;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await action();
      setNotice(success);
      refresh();
    } catch (failure) {
      setError(message(failure));
    } finally {
      setBusy(false);
    }
  }

  const publicUrl = (() => {
    if (!slug) return "";
    const path = `/c/${slug}`;
    if (plazaUrl.startsWith("http://") || plazaUrl.startsWith("https://")) {
      return `${plazaUrl.replace(/\/+$/, "")}${path}`;
    }
    if (typeof window !== "undefined" && window.location?.origin) {
      return `${window.location.origin}${path}`;
    }
    return path;
  })();

  async function copyPublicUrl() {
    try {
      await navigator.clipboard.writeText(publicUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
      setNotice("合集链接已复制到剪贴板。");
    } catch {
      setNotice(`公开链接：${publicUrl}`);
    }
  }

  // ------------------------------------------------------------------ 详情视图 (Detail View)
  if (slug && selected) {
    const status = stateOf(selected);
    const entries = selected.entries ?? [];
    const isChallenge = selected.kind === "challenge";

    return (
      <div class="collection-workspace">
        <nav class="collection-breadcrumb" aria-label="返回导航">
          <a class="breadcrumb-item" href={collectionLink()}>
            <svg class="breadcrumb-arrow" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M15 18l-6-6 6-6" />
            </svg>
            <span>我的合集</span>
          </a>
          <span class="breadcrumb-sep" aria-hidden="true">/</span>
          <span class="breadcrumb-current" title={selected.title}>{selected.title}</span>
        </nav>

        {error ? (
          <div class="collection-error" role="alert">
            <span>{error}</span>
            <button class="button small" type="button" onClick={refresh}>
              重新读取
            </button>
          </div>
        ) : null}

        <header class="collection-identity">
          <div class="collection-emblem" style={`--h: ${hue(selected.slug)}`} aria-hidden="true">
            <span class="collection-emblem-glyph">{monogram(selected.title, selected.slug)}</span>
            <span class="collection-emblem-sub">{isChallenge ? "🎯" : "📚"}</span>
          </div>

          <div class="collection-identity-body">
            <div class="collection-identity-title-row">
              <h1>{selected.title}</h1>
              <span class="collection-identity-slug mono">{selected.slug}</span>
            </div>

            <div class="collection-chips">
              <span class="collection-chip">
                {isChallenge ? "🎯 创作挑战" : "📚 自选作品集"}
              </span>
              <span class={`collection-chip ${status.tone} ${status.pulse ? "pulse" : ""}`}>
                {status.text}
              </span>
              <span class="collection-chip">
                {entries.length} 件收录作品
              </span>
              {selected.closes_at ? (
                <span class={`collection-chip ${closed ? "warn" : ""}`}>
                  {closed ? "已截止" : `截止于 ${new Date(selected.closes_at).toLocaleDateString()}`}
                </span>
              ) : null}
            </div>

            <p class="collection-identity-summary">
              {selected.summary || "未填写一句话介绍。把能打开的作品放在一起。"}
            </p>
          </div>

          <div class="collection-header-actions">
            {selected.public ? (
              <button
                class={`collection-action-btn ${copied ? "copied" : ""}`}
                type="button"
                onClick={copyPublicUrl}
                title="复制合集公开访问链接"
              >
                {copied ? (
                  <>
                    <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" style={{ width: "13px", height: "13px" }} aria-hidden="true">
                      <path d="M20 6L9 17l-5-5" />
                    </svg>
                    <span>已复制链接</span>
                  </>
                ) : (
                  <>
                    <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style={{ width: "13px", height: "13px" }} aria-hidden="true">
                      <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
                      <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
                    </svg>
                    <span>复制链接</span>
                  </>
                )}
              </button>
            ) : null}

            {selected.public && !selected.hidden ? (
              <a
                class="collection-action-btn"
                href={publicUrl}
                target="_blank"
                rel="noreferrer"
                title="在新标签页中以玩家视角查看此合集"
              >
                <span>打开玩家页面</span>
                <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style={{ width: "13px", height: "13px" }} aria-hidden="true">
                  <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                  <polyline points="15 3 21 3 21 9" />
                  <line x1="10" y1="14" x2="21" y2="3" />
                </svg>
              </a>
            ) : null}

            {owner ? (
              <button
                class="collection-action-btn"
                type="button"
                onClick={() => setEditing(true)}
                title="修改合集资料、开放范围或删除合集"
              >
                <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style={{ width: "13px", height: "13px" }} aria-hidden="true">
                  <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
                  <circle cx="12" cy="12" r="3" />
                </svg>
                <span>设置</span>
              </button>
            ) : null}
          </div>
        </header>

        {/* 若为挑战，提供题目与规则的切换标签 */}
        {isChallenge ? (
          <nav class="collection-tabs-nav" aria-label="合集功能分段">
            <button
              type="button"
              class={`collection-tab-btn ${tab === "entries" ? "active" : ""}`}
              onClick={() => setTab("entries")}
            >
              收录作品 <span class="collection-tab-badge">{entries.length}</span>
            </button>
            <button
              type="button"
              class={`collection-tab-btn ${tab === "prompt" ? "active" : ""}`}
              onClick={() => setTab("prompt")}
            >
              题目与规则
            </button>
          </nav>
        ) : null}

        {/* 区域一：作品列表 */}
        {tab === "entries" ? (
          <div>
            {!authenticated && me ? (
              <div class="collection-callout">
                投稿作品需要 GitHub 长期账号。
                <button
                  class="button quiet small"
                  type="button"
                  onClick={() => window.dispatchEvent(new Event(AUTH_REQUEST_EVENT))}
                >
                  登录
                </button>
              </div>
            ) : closed ? (
              <div class="collection-callout">
                本次挑战已经截止，停止接收新投稿。已有作品仍可观看，创作者亦可随时撤回。
              </div>
            ) : null}

            <section class="collection-entries-section">
              <div class="collection-entries-head">
                <div class="collection-entries-title">
                  <h2>收录的作品</h2>
                  <span class="collection-entries-count">{entries.length}</span>
                </div>
                {!selected.hidden && !closed && authenticated && (owner || isChallenge) ? (
                  <button
                    class="button primary small"
                    type="button"
                    onClick={() => setAddingWork(true)}
                  >
                    {owner ? "+ 添加作品" : "+ 投稿作品"}
                  </button>
                ) : null}
              </div>

              {entries.length ? (
                <ul class="collection-entry-list">
                  {entries.map((entry, index) => {
                    const isOwn = memberSlugs.has(entry.slug);
                    return (
                      <li class="collection-entry-item" key={entry.slug}>
                        <div class="collection-entry-left">
                          <div class="collection-entry-icon">
                            #{index + 1}
                          </div>
                          <div class="collection-entry-info">
                            <div class="collection-entry-info-top">
                              <a
                                class="collection-entry-link"
                                href={`${plazaUrl.replace(/\/+$/, "")}/p/${entry.slug}?collection=${selected.slug}`}
                                target="_blank"
                                rel="noreferrer"
                              >
                                {entry.title}
                              </a>
                              <span class="collection-entry-ver">v{entry.submitted_version}</span>
                              {isOwn ? <span class="collection-chip good">你的作品</span> : null}
                            </div>
                            {entry.note ? (
                              <p class="collection-entry-note">“{entry.note}”</p>
                            ) : null}
                          </div>
                        </div>

                        <div class="collection-entry-actions">
                          {owner || isOwn ? (
                            <button
                              class={`button small ${isOwn ? "quiet" : "danger"}`}
                              disabled={busy}
                              type="button"
                              onClick={() => {
                                if (
                                  confirm(
                                    isOwn
                                      ? "从这个合集撤回？原作品和分享链接不受影响。"
                                      : "移除这件投稿并阻止反复重投？原作品不会被删除。"
                                  )
                                ) {
                                  void act(
                                    () => api.withdrawCollection(selected.slug, entry.slug),
                                    "已从合集中移除，原作品未删除。"
                                  );
                                }
                              }}
                            >
                              {isOwn ? "撤回" : "移除"}
                            </button>
                          ) : null}
                        </div>
                      </li>
                    );
                  })}
                </ul>
              ) : (
                <div class="collection-empty">
                  <div class="collection-empty-icon" aria-hidden="true">
                    <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2">
                      <rect x="3" y="3" width="18" height="18" rx="2" />
                      <path d="M3 9h18M9 21V9" />
                    </svg>
                  </div>
                  <h2>{isChallenge ? "还没有收到任何公开投稿" : "还没有收录任何公开作品"}</h2>
                  <p>
                    {isChallenge
                      ? "参与本次创作挑战，将你已发布的公开作品投递进来。"
                      : "把已发布的作品汇聚在一起，让玩家一口气体验完整系列。"}
                  </p>
                  {!selected.hidden && !closed && authenticated && (owner || isChallenge) ? (
                    <button
                      class="button primary"
                      type="button"
                      onClick={() => setAddingWork(true)}
                    >
                      {owner ? "+ 添加作品" : "+ 投稿我的作品"}
                    </button>
                  ) : null}
                </div>
              )}
            </section>
          </div>
        ) : null}

        {/* 区域二：挑战题目与规则 */}
        {tab === "prompt" && isChallenge ? (
          <div>
            <div class="collection-prompt-canvas">
              <div class="collection-prompt-canvas-head">
                <h3>创作题目</h3>
                <button
                  class={`collection-action-btn small ${promptCopied ? "copied" : ""}`}
                  type="button"
                  onClick={async () => {
                    try {
                      await navigator.clipboard.writeText(selected.prompt);
                      setPromptCopied(true);
                      setTimeout(() => setPromptCopied(false), 2000);
                      setNotice("题目已复制到剪贴板。");
                    } catch {
                      setNotice("复制失败，请手动选取题目文本。");
                    }
                  }}
                >
                  {promptCopied ? (
                    <>
                      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" style={{ width: "12px", height: "12px" }} aria-hidden="true">
                        <path d="M20 6L9 17l-5-5" />
                      </svg>
                      <span>已复制</span>
                    </>
                  ) : (
                    <>
                      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style={{ width: "12px", height: "12px" }} aria-hidden="true">
                        <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                      </svg>
                      <span>复制题目</span>
                    </>
                  )}
                </button>
              </div>
              <p class="collection-prompt-text">{selected.prompt}</p>
            </div>

            {selected.rules ? (
              <div class="collection-rules-card">
                <h4>投稿规则与约定</h4>
                <p>{selected.rules}</p>
              </div>
            ) : null}
          </div>
        ) : null}

        {/* 弹窗：合集设置 (Owner) */}
        {editing && owner ? (
          <CollectionDialog
            collection={selected}
            onSave={async (draft) => {
              await api.updateCollection(selected.slug, draft);
              setNotice("合集资料已保存。");
              refresh();
            }}
            onDelete={async () => {
              await api.deleteCollection(selected.slug);
              location.hash = collectionLink();
            }}
            onUnblock={async (siteSlug) => {
              await api.unblockCollection(selected.slug, siteSlug);
              setNotice("已允许重新投稿。");
              refresh();
            }}
            onClose={() => setEditing(false)}
          />
        ) : null}

        {/* 弹窗：添加/投稿作品 */}
        {addingWork ? (
          <WorkSubmitDialog
            collection={selected}
            sites={sites}
            onSubmit={async (draft) => {
              await api.submitCollection(selected.slug, draft);
              setNotice("作品已加入。玩家页面可能需要最多 30 秒刷新。");
              refresh();
            }}
            onClose={() => setAddingWork(false)}
          />
        ) : null}

        {notice ? (
          <div class="collection-toast" role="status" aria-live="polite">
            <svg class="collection-toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true">
              <path d="M20 6L9 17l-5-5" />
            </svg>
            <span>{notice}</span>
          </div>
        ) : null}
      </div>
    );
  }

  // ------------------------------------------------------------------ 列表视图 (Index View)
  const availableChallenges = available.filter((item) => item.kind === "challenge");
  const list = (section === "mine" ? mine ?? [] : availableChallenges).filter(
    (item) => `${item.title} ${item.summary} ${item.creator}`.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div class="collection-workspace">
      <header class="workspace-head">
        <div>
          <h1>
            我的合集
            {mine ? <span class="workspace-count" style={{ marginLeft: "8px" }}>{mine.length}</span> : null}
          </h1>
        </div>

        <div>
          {authenticated ? (
            <button
              class="button primary"
              type="button"
              onClick={() => setCreating(true)}
            >
              + 新建合集
            </button>
          ) : null}
        </div>
      </header>

      {error ? (
        <div class="collection-error" role="alert">
          <span>{error}</span>
          <button class="button small" type="button" onClick={refresh}>
            重新读取
          </button>
        </div>
      ) : null}

      {!authenticated && me ? (
        <div class="collection-callout">
          创建合集与参与挑战需要使用 GitHub 长期账号。
          <button
            class="button quiet small"
            type="button"
            onClick={() => window.dispatchEvent(new Event(AUTH_REQUEST_EVENT))}
          >
            登录
          </button>
        </div>
      ) : null}

      {/* 新建合集弹窗 */}
      {creating ? (
        <CollectionDialog
          onSave={async (draft) => {
            const created = await api.createCollection(draft);
            location.hash = collectionLink(created.slug);
          }}
          onClose={() => setCreating(false)}
        />
      ) : null}

      {/* 工具栏：分段切换与搜索 */}
      <div class="collection-toolbar">
        <div class="switch" role="group" aria-label="合集范围">
          <button
            type="button"
            class={section === "mine" ? "on" : ""}
            onClick={() => setSection("mine")}
          >
            我的合集 ({mine?.length ?? 0})
          </button>
          <button
            type="button"
            class={section === "join" ? "on" : ""}
            onClick={() => setSection("join")}
          >
            参与挑战 ({availableChallenges.length})
          </button>
        </div>

        <div class="collection-search-wrap">
          <svg class="collection-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.35-4.35" />
          </svg>
          <input
            class="collection-search-input"
            type="search"
            value={search}
            onInput={(event) => setSearch(event.currentTarget.value)}
            placeholder="查找合集或题目…"
          />
          {search ? (
            <button
              class="collection-search-clear"
              type="button"
              aria-label="清空搜索"
              onClick={() => setSearch("")}
            >
              ✕
            </button>
          ) : null}
        </div>
      </div>

      {/* 卡片网格或空状态 */}
      {!mine && !error ? (
        <p class="muted" aria-live="polite">
          正在读取合集…
        </p>
      ) : list.length ? (
        <div class="collection-grid">
          {list.map((item) => {
            const status = stateOf(item);
            const isChallenge = item.kind === "challenge";
            return (
              <a class="collection-card" key={item.slug} href={collectionLink(item.slug)}>
                <div class="collection-card-art" style={`--h: ${hue(item.slug)}`} aria-hidden="true">
                  <div class="collection-card-art-bg" />
                  <div class="collection-card-art-medallion">
                    <span class="collection-card-glyph">{monogram(item.title, item.slug)}</span>
                  </div>
                  <span class="collection-card-badge-floating">
                    {isChallenge ? "🎯 挑战" : "📚 作品集"}
                  </span>
                  <span class={`collection-chip-floating ${status.tone} ${status.pulse ? "pulse" : ""}`}>
                    {status.text}
                  </span>
                </div>

                <div class="collection-card-body">
                  <h2>{item.title}</h2>
                  <p class="collection-card-summary">
                    {item.summary || "把能打开的作品放在一起。"}
                  </p>

                  {isChallenge && item.prompt ? (
                    <div class="collection-card-prompt">{item.prompt}</div>
                  ) : null}

                  <div class="collection-card-footer">
                    <span class="creator">{item.creator}</span>
                    <span class="count">{(item.entries ?? []).length} 件作品 →</span>
                  </div>
                </div>
              </a>
            );
          })}
        </div>
      ) : (
        <section class="collection-empty">
          <div class="collection-empty-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" />
            </svg>
          </div>
          <h2>
            {search
              ? "没有找到符合条件的合集"
              : section === "mine"
              ? "给作品一个共同的去处"
              : "下一场挑战，还在等一个好题目"}
          </h2>
          <p>
            {search
              ? "尝试更换搜索词，或清空搜索条件。"
              : section === "mine"
              ? "建一个作品集整理自己的系列创作，或发起同题挑战，邀请大家一起做。"
              : "已公开的挑战会出现在这里。也可以先把自己的作品发布到广场。"}
          </p>
          {authenticated && !search && section === "mine" ? (
            <button class="button primary" type="button" onClick={() => setCreating(true)}>
              新建合集
            </button>
          ) : null}
        </section>
      )}

      {notice ? (
        <div class="collection-toast" role="status" aria-live="polite">
          <svg class="collection-toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true">
            <path d="M20 6L9 17l-5-5" />
          </svg>
          <span>{notice}</span>
        </div>
      ) : null}
    </div>
  );
}

// ------------------------------------------------------------------ 合集弹窗：设置与新建 (CollectionDialog)
function CollectionDialog({
  collection,
  onSave,
  onDelete,
  onUnblock,
  onClose,
}: {
  collection?: Collection;
  onSave: (draft: CollectionDraft) => Promise<void>;
  onDelete?: () => Promise<void>;
  onUnblock?: (siteSlug: string) => Promise<void>;
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [draft, setDraft] = useState<CollectionDraft>(() =>
    collection
      ? {
          slug: collection.slug,
          title: collection.title,
          summary: collection.summary,
          kind: collection.kind,
          prompt: collection.prompt,
          rules: collection.rules,
          closes_at: collection.closes_at,
          public: collection.public,
        }
      : {
          title: "",
          slug: undefined,
          summary: "",
          kind: "collection",
          prompt: "",
          rules: "",
          closes_at: undefined,
          public: false,
        }
  );
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    dialog.current?.showModal();
    return () => {
      if (dialog.current?.open) dialog.current.close();
    };
  }, []);

  const locked = Boolean(collection?.entries?.length);
  const isChallenge = draft.kind === "challenge";
  const deadline = draft.closes_at
    ? new Date(Date.parse(draft.closes_at) - new Date(draft.closes_at).getTimezoneOffset() * 60000)
        .toISOString()
        .slice(0, 16)
    : "";

  return (
    <dialog
      ref={dialog}
      class="overlay"
      aria-labelledby="collection-dialog-title"
      onClose={onClose}
      onClick={(e) => {
        if (e.target === dialog.current) onClose();
      }}
    >
      <div class="sheet collection-dialog-sheet">
        <div class="dialog-head">
          <div class="dialog-title-wrap">
            <BrandMark />
            <h2 id="collection-dialog-title">
              {collection ? "合集设置" : "新建合集 / 创作挑战"}
            </h2>
          </div>
          <button
            class="close"
            type="button"
            aria-label="关闭"
            onClick={onClose}
          >
            <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="m6 6 12 12M6 18 18 6" />
            </svg>
          </button>
        </div>

        <form
          class="collection-form"
          onSubmit={async (event) => {
            event.preventDefault();
            if (busy) return;
            setBusy(true);
            setError("");
            try {
              await onSave(draft);
              onClose();
            } catch (failure) {
              setError(message(failure));
            } finally {
              setBusy(false);
            }
          }}
        >
          <fieldset disabled={busy} style={{ border: 0, padding: 0, margin: 0, minWidth: 0 }}>
            {!collection ? (
              <div class="collection-type-grid" role="radiogroup" aria-label="组织形态">
                <div
                  class={`collection-type-option ${draft.kind === "collection" ? "active" : ""}`}
                  onClick={() => setDraft({ ...draft, kind: "collection" })}
                  role="radio"
                  aria-checked={draft.kind === "collection"}
                  tabIndex={0}
                >
                  <div class="collection-type-icon" aria-hidden="true">
                    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" />
                    </svg>
                  </div>
                  <div class="collection-type-info">
                    <h3>自选作品集</h3>
                    <p>汇聚自己的系列作品</p>
                  </div>
                </div>

                <div
                  class={`collection-type-option ${draft.kind === "challenge" ? "active" : ""}`}
                  onClick={() => setDraft({ ...draft, kind: "challenge" })}
                  role="radio"
                  aria-checked={draft.kind === "challenge"}
                  tabIndex={0}
                >
                  <div class="collection-type-icon" aria-hidden="true">
                    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
                      <circle cx="12" cy="12" r="10" />
                      <polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76" />
                    </svg>
                  </div>
                  <div class="collection-type-info">
                    <h3>创作挑战</h3>
                    <p>出题邀请所有创作者投稿</p>
                  </div>
                </div>
              </div>
            ) : null}

            <div class="field">
              <label class="field-label">合集名称</label>
              <input
                required
                maxLength={80}
                value={draft.title}
                onInput={(e) => setDraft({ ...draft, title: e.currentTarget.value })}
                placeholder="例如：鹈鹕骑单车"
                autoFocus
              />
            </div>

            {!collection ? (
              <div class="field">
                <label class="field-label">自定义地址 (选填)</label>
                <div class="collection-slug-preview">
                  <span class="collection-slug-prefix">playtest.run/c/</span>
                  <input
                    pattern="[a-z0-9-]{3,63}"
                    maxLength={63}
                    value={draft.slug ?? ""}
                    onInput={(e) =>
                      setDraft({ ...draft, slug: e.currentTarget.value || undefined })
                    }
                    placeholder="留空自动生成"
                  />
                </div>
              </div>
            ) : null}

            <div class="field">
              <label class="field-label">一句话介绍</label>
              <textarea
                rows={2}
                maxLength={280}
                value={draft.summary}
                onInput={(e) => setDraft({ ...draft, summary: e.currentTarget.value })}
                placeholder="简短介绍这个合集收录了什么…"
              />
            </div>

            {isChallenge ? (
              <>
                <div class="field">
                  <label class="field-label">创作题目</label>
                  <textarea
                    rows={3}
                    maxLength={6000}
                    required={draft.public}
                    disabled={locked}
                    value={draft.prompt}
                    onInput={(e) => setDraft({ ...draft, prompt: e.currentTarget.value })}
                    placeholder="清晰描述参与者要做什么…"
                  />
                </div>

                <div class="field">
                  <label class="field-label">投稿规则 (选填)</label>
                  <textarea
                    rows={2}
                    maxLength={3000}
                    disabled={locked}
                    value={draft.rules}
                    onInput={(e) => setDraft({ ...draft, rules: e.currentTarget.value })}
                    placeholder="约定格式、限制或注意事项…"
                  />
                </div>

                <div class="field">
                  <label class="field-label">截止时间 (选填)</label>
                  <input
                    type="datetime-local"
                    value={deadline}
                    disabled={locked}
                    onInput={(e) =>
                      setDraft({
                        ...draft,
                        closes_at: e.currentTarget.value
                          ? new Date(e.currentTarget.value).toISOString()
                          : undefined,
                      })
                    }
                  />
                </div>
              </>
            ) : null}

            <div
              class={`collection-toggle-card ${draft.public ? "active" : ""}`}
              onClick={() => setDraft({ ...draft, public: !draft.public })}
              role="switch"
              aria-checked={draft.public}
              tabIndex={0}
            >
              <div class="collection-toggle-info">
                <h4>公开到合集大厅与搜索</h4>
                <p>
                  {draft.public
                    ? "已公开：所有人均可在大厅与搜索中浏览"
                    : "草稿中：仅你自己可见与管理"}
                </p>
              </div>
              <div class="collection-switch-pill" aria-hidden="true" />
            </div>

            {collection?.blocked_slugs?.length && onUnblock ? (
              <div class="collection-dialog-blocked">
                <label class="field-label">已移除的投稿 ({collection.blocked_slugs.length})</label>
                <div class="collection-dialog-blocked-list">
                  {collection.blocked_slugs.map((siteSlug) => (
                    <div class="collection-dialog-blocked-item" key={siteSlug}>
                      <span class="mono">{siteSlug}</span>
                      <button
                        class="button small quiet"
                        type="button"
                        onClick={() => void onUnblock(siteSlug)}
                      >
                        允许重新投稿
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}

            {error ? <p role="alert" class="collection-error">{error}</p> : null}

            <div class="collection-dialog-foot">
              {collection && onDelete ? (
                <button
                  class="button danger small"
                  type="button"
                  onClick={() => {
                    if (confirm("确定删除这个合集？里面的所有作品与分享链接不受影响。")) {
                      void onDelete();
                    }
                  }}
                >
                  删除合集
                </button>
              ) : <div />}

              <div class="collection-dialog-actions">
                <button class="button quiet" type="button" onClick={onClose}>
                  取消
                </button>
                <button class="button titanium" type="submit" disabled={busy}>
                  {busy ? "保存中…" : collection ? "保存设置" : "创建合集"}
                </button>
              </div>
            </div>
          </fieldset>
        </form>
      </div>
    </dialog>
  );
}

// ------------------------------------------------------------------ 作品收录与投稿弹窗 (WorkSubmitDialog)
function WorkSubmitDialog({
  collection,
  sites,
  onSubmit,
  onClose,
}: {
  collection: Collection;
  sites: Site[];
  onSubmit: (draft: EntryDraft) => Promise<void>;
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [draft, setDraft] = useState<EntryDraft>({ slug: "", note: "" });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const existingSlugs = new Set((collection.entries ?? []).map((e) => e.slug));
  const eligible = sites.filter(
    (site) =>
      site.current_version != null &&
      site.listing?.public &&
      !site.listing.hidden &&
      !site.expires_at &&
      !existingSlugs.has(site.slug)
  );

  useEffect(() => {
    if (eligible.length > 0 && !draft.slug) {
      setDraft((d) => ({ ...d, slug: eligible[0].slug }));
    }
    dialog.current?.showModal();
    return () => {
      if (dialog.current?.open) dialog.current.close();
    };
  }, []);

  return (
    <dialog
      ref={dialog}
      class="overlay"
      aria-labelledby="work-submit-title"
      onClose={onClose}
      onClick={(e) => {
        if (e.target === dialog.current) onClose();
      }}
    >
      <div class="sheet collection-submit-sheet">
        <div class="dialog-head">
          <div class="dialog-title-wrap">
            <BrandMark />
            <h2 id="work-submit-title">
              {collection.kind === "challenge" ? "投稿作品到挑战" : "添加作品到合集"}
            </h2>
          </div>
          <button class="close" type="button" aria-label="关闭" onClick={onClose}>
            <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="m6 6 12 12M6 18 18 6" />
            </svg>
          </button>
        </div>

        {eligible.length ? (
          <form
            class="collection-form"
            onSubmit={async (event) => {
              event.preventDefault();
              if (busy || !draft.slug) return;
              setBusy(true);
              setError("");
              try {
                await onSubmit(draft);
                onClose();
              } catch (failure) {
                setError(message(failure));
              } finally {
                setBusy(false);
              }
            }}
          >
            <div class="field">
              <label class="field-label">选择你的公开作品</label>
              <select
                required
                value={draft.slug}
                onChange={(e) => setDraft({ ...draft, slug: e.currentTarget.value })}
              >
                {eligible.map((site) => (
                  <option key={site.slug} value={site.slug}>
                    {site.title || site.slug} (v{site.current_version}) · /{site.slug}
                  </option>
                ))}
              </select>
            </div>

            <div class="field">
              <label class="field-label">一句话附言 (选填)</label>
              <input
                maxLength={140}
                value={draft.note}
                onInput={(e) => setDraft({ ...draft, note: e.currentTarget.value })}
                placeholder="例如：支持键盘和手柄操控"
              />
            </div>

            {error ? <p role="alert" class="collection-error">{error}</p> : null}

            <div class="collection-dialog-actions" style={{ marginTop: "16px", justifyContent: "flex-end" }}>
              <button class="button quiet" type="button" onClick={onClose}>
                取消
              </button>
              <button class="button titanium" type="submit" disabled={busy}>
                {busy ? "添加中…" : "确认添加"}
              </button>
            </div>
          </form>
        ) : (
          <div class="collection-empty-submit">
            <div class="empty-emoji" aria-hidden="true">📦</div>
            <h3>暂无可收录的公开作品</h3>
            <p>
              合集仅可收录已在平台公开发布的作品。已有的私密或草稿作品无需重新上传，只需在控制台中设为公开即可。
            </p>
            <div class="collection-dialog-actions" style={{ justifyContent: "center", marginTop: "20px" }}>
              <a class="button primary" href="#/" onClick={onClose}>
                前往我的作品 ↗
              </a>
            </div>
          </div>
        )}
      </div>
    </dialog>
  );
}
