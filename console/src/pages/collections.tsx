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

function stateOf(collection: Collection): string {
  if (collection.hidden) return "已被移除公开展示";
  if (!collection.public) return "草稿 · 只有你看得到";
  if (collection.closes_at && Date.parse(collection.closes_at) <= Date.now()) return "已结束 · 仍可观看";
  return collection.kind === "challenge" ? "开放投稿" : "公开作品集";
}

export function CollectionsPage({ slug, sites, me, plazaUrl }: { slug?: string; sites: Site[]; me: Me | null; plazaUrl: string }) {
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
  useEffect(() => {
    let current = true;
    Promise.all([api.collections(), api.openCollections(), slug ? api.collection(slug) : Promise.resolve(null)])
      .then(([owned, publicCollections, collection]) => {
        if (!current) return;
        setMine(owned); setAvailable(publicCollections); setSelected(collection); setError("");
      }).catch(failure => { if (current) setError(message(failure)); });
    return () => { current = false; };
  }, [slug, revision]);
  const refresh = () => setRevision(value => value + 1);
  const owner = Boolean(selected && mine?.some(collection => collection.slug === selected.slug));
  const memberSlugs = new Set(sites.map(site => site.slug));
  const authenticated = me?.kind === "github";
  const closed = Boolean(selected?.closes_at && Date.parse(selected.closes_at) <= Date.now());
  async function act(action: () => Promise<unknown>, success: string) {
    if (busy) return;
    setBusy(true); setError(""); setNotice("");
    try { await action(); setNotice(success); refresh(); }
    catch (failure) { setError(message(failure)); }
    finally { setBusy(false); }
  }
  const publicUrl = `${plazaUrl.replace(/\/+$/, "")}/c/${slug ?? ""}`;
  const list = (section === "mine" ? mine ?? [] : available.filter(collection => collection.kind === "challenge"))
    .filter(collection => `${collection.title} ${collection.summary}`.toLowerCase().includes(search.toLowerCase()));
  return <div class="collection-workspace">
    {slug ? <a class="collection-back" href={collectionLink()}>← 合集与挑战</a> : null}
    <header class="workspace-head"><div><h1>{selected?.title ?? "合集与挑战"}</h1><p>{selected ? stateOf(selected) : "把作品放在一起，让一个题目带来下一位创作者。"}</p></div>
      {selected?.public && !selected.hidden ? <a class="button" href={publicUrl} target="_blank" rel="noreferrer">打开玩家页面 ↗</a> : !slug && authenticated ? <button type="button" onClick={() => setCreating(value => !value)}>{creating ? "收起" : "新建合集"}</button> : null}
    </header>
    {error ? <div class="collection-error" role="alert">{error}<button type="button" onClick={refresh}>重新读取</button></div> : null}
    {notice ? <p class="collection-notice" role="status">{notice}</p> : null}
    {!mine && !error ? <p aria-live="polite">正在读取合集…</p> : null}
    {!authenticated && me ? <p class="collection-callout">创建或投稿合集需要 GitHub 账号。<button class="button quiet small" type="button" onClick={() => window.dispatchEvent(new Event(AUTH_REQUEST_EVENT))}>登录</button></p> : null}
    {creating && !slug ? <Editor onSave={async draft => {
      const collection = await api.createCollection(draft);
      location.hash = collectionLink(collection.slug);
    }} /> : null}
    {!slug && mine ? <>
      <div class="collection-list-tools"><div class="collection-switch" role="group" aria-label="合集范围"><button type="button" aria-pressed={section === "mine"} onClick={() => setSection("mine")}>我的合集</button><button type="button" aria-pressed={section === "join"} onClick={() => setSection("join")}>参与挑战</button></div><label><span class="sr-only">查找合集</span><input type="search" value={search} onInput={event => setSearch(event.currentTarget.value)} placeholder="查找一个题目" /></label></div>
      {list.length ? <div class="collection-list">{list.map(collection => <a class="collection-list-card" key={collection.slug} href={collectionLink(collection.slug)}><span>{stateOf(collection)}</span><h2>{collection.title}</h2><p>{collection.summary || "把能打开的作品放在一起。"}</p><footer><span>{collection.creator}</span><span>{(collection.entries ?? []).length} 件作品 →</span></footer></a>)}</div> : <section class="collection-blank"><h2>{search ? "没有找到这个合集" : section === "mine" ? "给作品一个共同的去处" : "下一场挑战，还在等一个好题目"}</h2><p>{section === "mine" ? "建一个作品集整理自己的创作，或发起挑战，邀请别人一起做。" : "已公开的挑战会出现在这里。也可以先把自己的作品发布到广场。"}</p>{authenticated && !search ? <button type="button" onClick={() => setCreating(true)}>新建合集</button> : null}</section>}
    </> : null}
    {selected ? <>
      {owner ? <details class="collection-panel"><summary>编辑资料与公开状态</summary><Editor key={`${selected.slug}-${selected.updated_at}`} collection={selected} onSave={async draft => { await api.updateCollection(selected.slug, draft); setNotice("合集资料已保存。"); refresh(); }} /></details> : <section class="collection-panel"><p>{selected.summary}</p><h2>这次做什么</h2><pre>{selected.prompt}</pre>{selected.rules ? <details><summary>投稿规则</summary><p class="collection-preserve">{selected.rules}</p></details> : null}</section>}
      {!selected.hidden && !closed && authenticated && (owner || selected.kind === "challenge") ? <Submission collection={selected} sites={sites} onSubmit={async draft => { await api.submitCollection(selected.slug, draft); setNotice("作品已加入。玩家页面可能需要最多 30 秒刷新。"); refresh(); }} /> : <p class="collection-callout">{closed ? "已经截止，不能新增或替换投稿。已有作品仍可观看，也可以撤回。" : "当前不能投稿；你的原作品不受影响。"}</p>}
      <section class="collection-panel"><h2>合集里的作品</h2>{(selected.entries ?? []).length ? <ul class="collection-entry-list">{(selected.entries ?? []).map(entry => <li key={entry.slug}><div><a href={`${plazaUrl.replace(/\/+$/, "")}/p/${entry.slug}?collection=${selected.slug}`} target="_blank" rel="noreferrer">{entry.title}</a><p>投稿 v{entry.submitted_version}{entry.note ? ` · ${entry.note}` : ""}</p></div>{owner || memberSlugs.has(entry.slug) ? <button disabled={busy} type="button" onClick={() => {
        const ownEntry = memberSlugs.has(entry.slug);
        if (confirm(ownEntry ? "从这个合集撤回？原作品和分享链接不受影响。" : "移除这件投稿并阻止反复重投？原作品不会被删除。")) void act(() => api.withdrawCollection(selected.slug, entry.slug), "已从合集中移除，原作品未删除。");
      }}>{memberSlugs.has(entry.slug) ? "撤回" : "移除"}</button> : null}</li>)}</ul> : <p class="muted">还没有有效的公开作品。被设为不公开、到期或隐藏的作品不会展示。</p>}</section>
      {owner && selected.blocked_slugs?.length ? <section class="collection-panel"><h2>已移除的投稿</h2><ul class="collection-entry-list">{selected.blocked_slugs.map(site => <li key={site}><span>{site}</span><button type="button" disabled={busy} onClick={() => void act(() => api.unblockCollection(selected.slug, site), "已允许重新投稿，原作品不会自动恢复。")}>允许重新投稿</button></li>)}</ul></section> : null}
      {owner ? <section class="collection-danger"><p>删除合集只移除这个聚合页面，不会删除里面的作品。</p><button type="button" disabled={busy} onClick={() => { if (confirm("删除这个合集及其公开页面？原作品不会删除。")) void act(async () => { await api.deleteCollection(selected.slug); location.hash = collectionLink(); }, "合集已删除。"); }}>删除合集</button></section> : null}
    </> : null}
  </div>;
}

function Editor({ collection, onSave }: { collection?: Collection; onSave: (draft: CollectionDraft) => Promise<void> }) {
  const [draft, setDraft] = useState<CollectionDraft>(() => collection ? { slug: collection.slug, title: collection.title, summary: collection.summary, kind: collection.kind, prompt: collection.prompt, rules: collection.rules, closes_at: collection.closes_at, public: collection.public } : { title: "", slug: undefined, summary: "", kind: "collection", prompt: "", rules: "", closes_at: undefined, public: false });
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const locked = Boolean(collection?.entries?.length);
  const deadline = draft.closes_at ? new Date(Date.parse(draft.closes_at) - new Date(draft.closes_at).getTimezoneOffset() * 60000).toISOString().slice(0, 16) : "";
  return <form class="collection-editor" onSubmit={async event => {
    event.preventDefault(); if (busy) return; setBusy(true); setError("");
    try { await onSave(draft); } catch (failure) { setError(message(failure)); } finally { setBusy(false); }
  }}>
    <fieldset disabled={busy}><legend>{collection ? "合集资料" : "新建合集"}</legend>
      {!collection ? <label>你想怎么组织作品<select value={draft.kind} onChange={event => setDraft({ ...draft, kind: event.currentTarget.value as "collection" | "challenge" })}><option value="collection">作品集 · 整理自己的作品</option><option value="challenge">创作挑战 · 邀请大家一起做</option></select></label> : null}
      <label>名字<input required maxLength={80} value={draft.title} onInput={event => setDraft({ ...draft, title: event.currentTarget.value })} placeholder="例如：鹈鹕骑单车" /></label>
      {!collection ? <label>合集地址 · 选填<input pattern="[a-z0-9-]{3,63}" maxLength={63} value={draft.slug ?? ""} onInput={event => setDraft({ ...draft, slug: event.currentTarget.value || undefined })} placeholder="pelican-bicycle；留空自动生成" /></label> : null}
      <label>一句话介绍<textarea rows={2} maxLength={280} value={draft.summary} onInput={event => setDraft({ ...draft, summary: event.currentTarget.value })} /></label>
      {draft.kind === "challenge" ? <><label>创作题目<textarea rows={5} maxLength={6000} required={draft.public} disabled={locked} value={draft.prompt} onInput={event => setDraft({ ...draft, prompt: event.currentTarget.value })} placeholder="让参与者知道要做什么。" /></label><label>投稿规则 · 选填<textarea rows={3} maxLength={3000} disabled={locked} value={draft.rules} onInput={event => setDraft({ ...draft, rules: event.currentTarget.value })} /></label><label>截止时间 · 选填，按你的本地时区<input type="datetime-local" value={deadline} disabled={locked} onInput={event => setDraft({ ...draft, closes_at: event.currentTarget.value ? new Date(event.currentTarget.value).toISOString() : undefined })} /></label>{locked ? <p class="muted">已有投稿后不再修改题目、规则与截止时间，避免改变大家参加时的约定。</p> : null}</> : null}
      <label class="collection-check"><input type="checkbox" checked={draft.public} onChange={event => setDraft({ ...draft, public: event.currentTarget.checked })} />公开展示到合集页和搜索中</label>
      <p class="muted">默认是草稿。合集公开不会自动公开你的任何作品。</p>
      {error ? <p role="alert" class="collection-error">{error}</p> : null}
      <button type="submit">{busy ? "正在保存…" : collection ? "保存资料" : "创建合集"}</button>
    </fieldset>
  </form>;
}

function Submission({ collection, sites, onSubmit }: { collection: Collection; sites: Site[]; onSubmit: (draft: EntryDraft) => Promise<void> }) {
  const [draft, setDraft] = useState<EntryDraft>({ slug: "", note: "" });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const eligible = sites.filter(site => site.current_version != null && site.listing?.public && !site.listing.hidden && !site.expires_at);
  const existing = (collection.entries ?? []).some(entry => entry.slug === draft.slug);
  return <section class="collection-panel"><h2>把我的作品放进来</h2>{eligible.length ? <form class="collection-editor" onSubmit={async event => {
    event.preventDefault(); if (busy) return; setBusy(true); setError("");
    try { await onSubmit(draft); } catch (failure) { setError(message(failure)); } finally { setBusy(false); }
  }}><fieldset disabled={busy}><legend class="sr-only">投稿</legend>
    <label>选择已发布的公开作品<select required value={draft.slug} onChange={event => {
      const slug = event.currentTarget.value; const entry = (collection.entries ?? []).find(entry => entry.slug === slug);
      setDraft({ slug, note: entry?.note ?? "" });
    }}><option value="">选择一件作品</option>{eligible.map(site => <option key={site.slug} value={site.slug}>{site.title} · v{site.current_version}</option>)}</select></label>
    <label>作者附言 · 选填<textarea rows={3} maxLength={6000} value={draft.note ?? ""} placeholder="关于这件作品的说明或留言，会公开展示。" onInput={event => setDraft({ ...draft, note: event.currentTarget.value })} /></label>
    <p class="muted">附言由你填写。记录本次投稿版本；以后更新作品，玩家会打开当前版本，页面会标明版本变化。</p>
    {error ? <p role="alert" class="collection-error">{error}</p> : null}
    <button type="submit">{busy ? "正在提交…" : existing ? "更新这件投稿" : "加入合集"}</button>
  </fieldset></form> : <p class="collection-callout">先发布并公开一件长期作品，再回来选择。已有作品不需要重复上传。<a href={href({ name: "sites" })}>回到我的作品 →</a></p>}</section>;
}
