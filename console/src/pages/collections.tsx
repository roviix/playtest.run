import { useEffect, useState } from "preact/hooks";
import { api, AUTH_REQUEST_EVENT, type Collection, type CollectionDraft, type EntryDraft, type Me, type Site } from "../api";
import { href } from "../router";
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
  const [busy, setBusy] = useState(false);
  const [section, setSection] = useState<"mine" | "join">("mine");
  const [search, setSearch] = useState("");
  const [tab, setTab] = useState<"entries" | "settings" | "prompt">("entries");
  const [copied, setCopied] = useState(false);

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
          <a class="back-link" href={collectionLink()}>
            <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m15 18-6-6 6-6" /></svg>
            <span>我的合集</span>
          </a>
          <span class="breadcrumb-sep" aria-hidden="true">/</span>
          <span class="breadcrumb-title" title={selected.title}>{selected.title}</span>
        </nav>

        {error ? (
          <div class="collection-error" role="alert">
            <span>{error}</span>
            <button class="button small" type="button" onClick={refresh}>
              重新读取
            </button>
          </div>
        ) : null}
        {notice ? (
          <p class="collection-notice" role="status">
            {notice}
          </p>
        ) : null}

        <header class="collection-identity">
          <div class="collection-emblem" aria-hidden="true">
            {isChallenge ? (
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10" />
                <polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76" />
              </svg>
            ) : (
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" />
              </svg>
            )}
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
            {selected.public && !selected.hidden ? (
              <a class="button" href={publicUrl} target="_blank" rel="noreferrer">
                打开玩家页面 ↗
              </a>
            ) : null}
            {selected.public ? (
              <button class="button quiet" type="button" onClick={copyPublicUrl}>
                {copied ? "已复制链接 ✓" : "复制链接"}
              </button>
            ) : null}
          </div>
        </header>

        {/* 详情页选项卡导航 */}
        <nav class="collection-tabs-nav" aria-label="合集功能分段">
          <button
            type="button"
            class={`collection-tab-btn ${tab === "entries" ? "active" : ""}`}
            onClick={() => setTab("entries")}
          >
            收录作品 <span class="collection-tab-badge">{entries.length}</span>
          </button>
          {owner ? (
            <button
              type="button"
              class={`collection-tab-btn ${tab === "settings" ? "active" : ""}`}
              onClick={() => setTab("settings")}
            >
              资料与设置
            </button>
          ) : isChallenge ? (
            <button
              type="button"
              class={`collection-tab-btn ${tab === "prompt" ? "active" : ""}`}
              onClick={() => setTab("prompt")}
            >
              题目与规则
            </button>
          ) : null}
        </nav>

        {/* 选项卡一：作品列表与投稿舱 */}
        {tab === "entries" ? (
          <div>
            {!selected.hidden && !closed && authenticated && (owner || isChallenge) ? (
              <Submission
                collection={selected}
                sites={sites}
                onSubmit={async (draft) => {
                  await api.submitCollection(selected.slug, draft);
                  setNotice("作品已加入。玩家页面可能需要最多 30 秒刷新。");
                  refresh();
                }}
              />
            ) : !authenticated && me ? (
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
              <h2>收录的作品 ({entries.length})</h2>
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
                  <h2>还没有收录任何公开作品</h2>
                  <p>被设为私密、到期或已被移除的作品不会展示在这里。通过上方表单选择作品加入。</p>
                </div>
              )}
            </section>
          </div>
        ) : null}

        {/* 选项卡二：资料与设置 (Owner Only) */}
        {tab === "settings" && owner ? (
          <div>
            <div class="collection-panel">
              <div class="collection-panel-head">
                <div>
                  <h2>编辑合集资料与公开状态</h2>
                  <p>管理合集的标题、一句话简介与发现状态。</p>
                </div>
              </div>
              <Editor
                key={`${selected.slug}-${selected.updated_at}`}
                collection={selected}
                onSave={async (draft) => {
                  await api.updateCollection(selected.slug, draft);
                  setNotice("合集资料已保存。");
                  refresh();
                }}
              />
            </div>

            {selected.blocked_slugs?.length ? (
              <section class="collection-panel">
                <div class="collection-panel-head">
                  <div>
                    <h2>已移除的投稿</h2>
                    <p>被你移除的作品被限制再次提交，你可以在此解除限制。</p>
                  </div>
                </div>
                <ul class="collection-entry-list">
                  {selected.blocked_slugs.map((siteSlug) => (
                    <li class="collection-entry-item" key={siteSlug}>
                      <span class="mono">{siteSlug}</span>
                      <button
                        class="button small quiet"
                        type="button"
                        disabled={busy}
                        onClick={() =>
                          void act(
                            () => api.unblockCollection(selected.slug, siteSlug),
                            "已允许重新投稿，原作品不会自动恢复。"
                          )
                        }
                      >
                        允许重新投稿
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}

            <section class="collection-danger-zone">
              <div class="collection-danger-info">
                <h4>删除合集</h4>
                <p>删除合集只移除这个聚合页面，里面的所有作品与分享链接不受任何影响。</p>
              </div>
              <button
                class="button danger"
                type="button"
                disabled={busy}
                onClick={() => {
                  if (confirm("删除这个合集及其公开页面？原作品不会删除。")) {
                    void act(async () => {
                      await api.deleteCollection(selected.slug);
                      location.hash = collectionLink();
                    }, "合集已删除。");
                  }
                }}
              >
                删除合集
              </button>
            </section>
          </div>
        ) : null}

        {/* 选项卡三：挑战题目与规则 (Participant Only) */}
        {tab === "prompt" && isChallenge ? (
          <div>
            <div class="collection-prompt-canvas">
              <div class="collection-prompt-canvas-head">
                <h3>创作题目</h3>
                <button
                  class="button small quiet"
                  type="button"
                  onClick={async () => {
                    try {
                      await navigator.clipboard.writeText(selected.prompt);
                      setNotice("题目已复制到剪贴板。");
                    } catch {
                      setNotice("复制失败，请手动选取题目文本。");
                    }
                  }}
                >
                  复制题目
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
            My Collections
            {mine ? <span class="workspace-count" style={{ marginLeft: "8px" }}>{mine.length}</span> : null}
          </h1>
        </div>

        <div>
          {authenticated ? (
            <button
              class={`button ${creating ? "quiet" : "primary"}`}
              type="button"
              onClick={() => setCreating((value) => !value)}
            >
              {creating ? "Collapse" : "+ New Collection"}
            </button>
          ) : null}
        </div>
      </header>

      {error ? (
        <div class="collection-error" role="alert">
          <span>{error}</span>
          <button class="button small" type="button" onClick={refresh}>
            Retry
          </button>
        </div>
      ) : null}
      {notice ? (
        <p class="collection-notice" role="status">
          {notice}
        </p>
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

      {/* 新建合集抽屉 */}
      {creating ? (
        <div class="collection-create-card">
          <div class="collection-create-head">
            <div>
              <h2>新建作品组织</h2>
              <p>选择你要创建的组织形态。默认保存为草稿，明确选择公开后才会出现在广场索引中。</p>
            </div>
            <button class="button small quiet" type="button" onClick={() => setCreating(false)}>
              ✕
            </button>
          </div>
          <Editor
            onSave={async (draft) => {
              const created = await api.createCollection(draft);
              location.hash = collectionLink(created.slug);
            }}
            onCancel={() => setCreating(false)}
          />
        </div>
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
                <div class="collection-card-head">
                  <span class="collection-card-type">
                    {isChallenge ? (
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76" />
                      </svg>
                    ) : (
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" />
                      </svg>
                    )}
                    {isChallenge ? "创作挑战" : "作品集"}
                  </span>
                  <span class={`collection-chip ${status.tone} ${status.pulse ? "pulse" : ""}`}>
                    {status.text}
                  </span>
                </div>

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
    </div>
  );
}

// ------------------------------------------------------------------ 合集编辑器 (Editor)
function Editor({
  collection,
  onSave,
  onCancel,
}: {
  collection?: Collection;
  onSave: (draft: CollectionDraft) => Promise<void>;
  onCancel?: () => void;
}) {
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

  const locked = Boolean(collection?.entries?.length);
  const isChallenge = draft.kind === "challenge";
  const deadline = draft.closes_at
    ? new Date(Date.parse(draft.closes_at) - new Date(draft.closes_at).getTimezoneOffset() * 60000)
        .toISOString()
        .slice(0, 16)
    : "";

  return (
    <form
      class="collection-form"
      onSubmit={async (event) => {
        event.preventDefault();
        if (busy) return;
        setBusy(true);
        setError("");
        try {
          await onSave(draft);
        } catch (failure) {
          setError(message(failure));
        } finally {
          setBusy(false);
        }
      }}
    >
      <fieldset disabled={busy} style={{ border: 0, padding: 0, margin: 0, minWidth: 0 }}>
        {/* 新建时选择组织形态：分段单选卡片 */}
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
                <p>整理自己的已发布作品，汇聚成个人系列专题。仅自己可添加作品。</p>
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
                <p>设定一道题目与规则，邀请所有创作者一起参与并投稿。</p>
              </div>
            </div>
          </div>
        ) : null}

        {/* 标题 */}
        <div class="field">
          <label class="field-label">
            <span>合集名称</span>
            <span class="field-hint">最多 80 字</span>
          </label>
          <input
            required
            maxLength={80}
            value={draft.title}
            onInput={(event) => setDraft({ ...draft, title: event.currentTarget.value })}
            placeholder="例如：鹈鹕骑单车"
          />
        </div>

        {/* Slug 地址 */}
        {!collection ? (
          <div class="field">
            <label class="field-label">
              <span>自定义地址</span>
              <span class="field-hint">选填，3-63 位小写字母、数字及横杠</span>
            </label>
            <div class="collection-slug-preview">
              <span class="collection-slug-prefix">playtest.run/c/</span>
              <input
                pattern="[a-z0-9-]{3,63}"
                maxLength={63}
                value={draft.slug ?? ""}
                onInput={(event) =>
                  setDraft({ ...draft, slug: event.currentTarget.value || undefined })
                }
                placeholder="pelican-bicycle（留空自动生成）"
              />
            </div>
          </div>
        ) : null}

        {/* 简介 */}
        <div class="field">
          <label class="field-label">
            <span>一句话介绍</span>
            <span class="field-hint">介绍合集的亮点或主题</span>
          </label>
          <textarea
            rows={2}
            maxLength={280}
            value={draft.summary}
            onInput={(event) => setDraft({ ...draft, summary: event.currentTarget.value })}
            placeholder="用简明的一两句话告诉读者这里收录了什么…"
          />
        </div>

        {/* 创作挑战特有字段 */}
        {isChallenge ? (
          <>
            <div class="field">
              <label class="field-label">
                <span>创作题目</span>
                <span class="field-hint">公开展示前必填</span>
              </label>
              <textarea
                rows={4}
                maxLength={6000}
                required={draft.public}
                disabled={locked}
                value={draft.prompt}
                onInput={(event) => setDraft({ ...draft, prompt: event.currentTarget.value })}
                placeholder="清晰描述参与者要做什么。例如：请用一个 HTML 文件做一只鹈鹕骑单车的动画…"
              />
            </div>

            <div class="field">
              <label class="field-label">
                <span>投稿规则 · 选填</span>
                <span class="field-hint">约定格式、限制或提交注意事项</span>
              </label>
              <textarea
                rows={2}
                maxLength={3000}
                disabled={locked}
                value={draft.rules}
                onInput={(event) => setDraft({ ...draft, rules: event.currentTarget.value })}
                placeholder="例如：只提交自己原创的作品；不使用违规素材…"
              />
            </div>

            <div class="field">
              <label class="field-label">
                <span>截止时间 · 选填</span>
                <span class="field-hint">按你的本地时区设置，截止后停止新增投稿</span>
              </label>
              <input
                type="datetime-local"
                value={deadline}
                disabled={locked}
                onInput={(event) =>
                  setDraft({
                    ...draft,
                    closes_at: event.currentTarget.value
                      ? new Date(event.currentTarget.value).toISOString()
                      : undefined,
                  })
                }
              />
            </div>

            {locked ? (
              <p class="muted">
                已有投稿后不再修改题目、规则与截止时间，避免改变大家参加时的约定。
              </p>
            ) : null}
          </>
        ) : null}

        {/* 公开状态开关卡片 */}
        <div
          class={`collection-toggle-card ${draft.public ? "active" : ""}`}
          onClick={() => setDraft({ ...draft, public: !draft.public })}
          role="switch"
          aria-checked={draft.public}
          tabIndex={0}
        >
          <div class="collection-toggle-info">
            <h4>公开展示到合集大厅与搜索</h4>
            <p>
              {draft.public
                ? "已设为公开：所有人均可在广场合集大厅看到并检索此合集。"
                : "当前为草稿：只有你自己能看到和管理。合集公开不会自动公开你的私密作品。"}
            </p>
          </div>
          <div class="collection-switch-pill" aria-hidden="true" />
        </div>

        {error ? (
          <p role="alert" class="collection-error">
            {error}
          </p>
        ) : null}

        <div class="collection-form-actions">
          {onCancel ? (
            <button class="button quiet" type="button" disabled={busy} onClick={onCancel}>
              取消
            </button>
          ) : null}
          <button class="button primary" type="submit" disabled={busy}>
            {busy ? "正在保存…" : collection ? "保存资料" : "创建合集"}
          </button>
        </div>
      </fieldset>
    </form>
  );
}

// ------------------------------------------------------------------ 投稿工作舱 (Submission)
function Submission({
  collection,
  sites,
  onSubmit,
}: {
  collection: Collection;
  sites: Site[];
  onSubmit: (draft: EntryDraft) => Promise<void>;
}) {
  const [draft, setDraft] = useState<EntryDraft>({ slug: "", note: "" });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const eligible = sites.filter(
    (site) =>
      site.current_version != null &&
      site.listing?.public &&
      !site.listing.hidden &&
      !site.expires_at
  );
  const existing = (collection.entries ?? []).some((entry) => entry.slug === draft.slug);

  return (
    <section class="collection-panel">
      <div class="collection-panel-head">
        <div>
          <h2>把我的作品放进来</h2>
          <p>
            {collection.kind === "challenge"
              ? "参与本次创作挑战，将你已发布的公开作品提交至本合集。"
              : "将自己的公开作品收录到当前作品集中。"}
          </p>
        </div>
      </div>

      {eligible.length ? (
        <form
          class="collection-form"
          onSubmit={async (event) => {
            event.preventDefault();
            if (busy) return;
            setBusy(true);
            setError("");
            try {
              await onSubmit(draft);
            } catch (failure) {
              setError(message(failure));
            } finally {
              setBusy(false);
            }
          }}
        >
          <fieldset disabled={busy} style={{ border: 0, padding: 0, margin: 0, minWidth: 0 }}>
            <legend class="sr-only">投稿</legend>

            <div class="field">
              <label class="field-label">
                <span>选择已发布的公开作品</span>
                <span class="field-hint">要求已上广场且未设置访问门禁</span>
              </label>
              <select
                required
                value={draft.slug}
                onChange={(event) => {
                  const selectedSlug = event.currentTarget.value;
                  const entry = (collection.entries ?? []).find((item) => item.slug === selectedSlug);
                  setDraft({ slug: selectedSlug, note: entry?.note ?? "" });
                }}
              >
                <option value="">选择一件作品…</option>
                {eligible.map((site) => (
                  <option key={site.slug} value={site.slug}>
                    {site.title} · v{site.current_version} ({site.slug})
                  </option>
                ))}
              </select>
            </div>

            <div class="field">
              <label class="field-label">
                <span>作者附言 · 选填</span>
                <span class="field-hint">关于本次创作的说明或心得，会公开展示</span>
              </label>
              <textarea
                rows={2}
                maxLength={6000}
                value={draft.note ?? ""}
                placeholder="写两句制作时的想法或提示…"
                onInput={(event) => setDraft({ ...draft, note: event.currentTarget.value })}
              />
            </div>

            <p class="muted">
              记录本次投稿版本；以后更新作品，玩家会打开当前最新版，合集内标明版本变化。
            </p>

            {error ? (
              <p role="alert" class="collection-error">
                {error}
              </p>
            ) : null}

            <div class="collection-form-actions">
              <button class="button primary" type="submit" disabled={busy}>
                {busy ? "正在提交…" : existing ? "更新这件投稿" : "加入合集"}
              </button>
            </div>
          </fieldset>
        </form>
      ) : (
        <div class="collection-callout">
          先发布并公开一件长期作品，再回来选择。已有作品不需要重复上传。
          <a href={href({ name: "sites" })}>回到我的作品 →</a>
        </div>
      )}
    </section>
  );
}

