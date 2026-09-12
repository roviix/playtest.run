// 设置（DESIGN §3.13）：广场、名额、开发者的群、公开反馈、推广、版本与回滚、危险区。
//
// 这些是偶尔改一次的东西，所以在最后一个签里，不压在结果上面。
// 关注只有数字，且已经在身份栏的签上——开发者永远看不到玩家的邮箱（DESIGN §3.6），
// 那不是偷懒，是我们替他扛下了一份玩家邮箱的合规责任。
// 推广那一节没有任何按钮：付款通道要等主体落地，放一个点不动的「购买」比什么都不放更伤人。

import type { ComponentChildren } from "preact";
import { useState } from "preact/hooks";

import { api, type Boost, type Listing, type Site, type UpdateSiteRequest } from "../api";
import { useLoad } from "../load";
import { go } from "../router";
import { bytes, day, moment } from "../words";
import { Failed, Loading } from "./status";

/** 和 common/src/limits.rs 的同名常量是同一份数字。 */
const MAX_SEATS = 500;
const MAX_COMMUNITY_URL_CHARS = 300;
const SEEK_NOTE_MAX = 140;

const EMPTY: Listing = {
  public: false,
  seeking: false,
  hidden: false,
  has_cover: false,
  joined: 0,
  followers: 0,
  feedback_public: false,
};

type Change = (request: UpdateSiteRequest) => Promise<void>;

export function SettingsTab({
  site,
  plazaUrl,
  onChanged,
  onVersionsChanged,
}: {
  site: Site;
  plazaUrl: string;
  onChanged: (site: Site) => void;
  onVersionsChanged: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const listing = site.listing ?? EMPTY;

  async function change(request: UpdateSiteRequest) {
    setBusy(true);
    setProblem(null);
    try {
      onChanged(await api.updateSite(site.slug, request));
    } catch (e) {
      setProblem(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div class="settings">
      {problem ? <p class="notice">{problem}</p> : null}
      <Plaza site={site} listing={listing} busy={busy} plazaUrl={plazaUrl} onChange={change} />
      <Seats listing={listing} busy={busy} onChange={change} />
      <Community listing={listing} busy={busy} onChange={change} />
      <PublicFeedback listing={listing} busy={busy} onChange={change} />
      <BoostSection boost={listing.boost} />
      <Versions site={site} onVersionsChanged={onVersionsChanged} />
      <Danger site={site} />
    </div>
  );
}

function Block({ title, lead, children }: { title: string; lead?: string; children?: ComponentChildren }) {
  return (
    <section class="block">
      <div class="block-head">
        <h2>{title}</h2>
        {lead ? <p class="muted">{lead}</p> : null}
      </div>
      {children ? <div class="block-body">{children}</div> : null}
    </section>
  );
}

// ------------------------------------------------------------------ 广场

function Plaza({
  site,
  listing,
  busy,
  plazaUrl,
  onChange,
}: {
  site: Site;
  listing: Listing;
  busy: boolean;
  plazaUrl: string;
  onChange: Change;
}) {
  const [note, setNote] = useState(listing.seek_note ?? "");
  const canPublish = site.current_version !== undefined;

  if (!canPublish) {
    return (
      <Block title="广场" lead="上传第一版之后可以放到广场上。" />
    );
  }

  if (!listing.public) {
    return (
      <Block title="广场" lead="没公开。只有拿到链接的人能玩。">
        <p class="row-actions">
          <button class="button" type="button" disabled={busy} onClick={() => onChange({ public: true })}>
            放到广场上
          </button>
          <button
            class="button primary"
            type="button"
            disabled={busy}
            onClick={() => onChange({ public: true, seeking: true })}
          >
            放上去找人测
          </button>
          <a class="muted" href={plazaUrl} target="_blank" rel="noreferrer">
            广场 ↗
          </a>
        </p>
      </Block>
    );
  }

  return (
    <Block
      title="广场"
      lead={
        listing.hidden
          ? "多人举报，已从广场撤下，待复核。链接仍能打开。"
          : `在广场上${listing.seeking ? "，正在找人测" : ""}。`
      }
    >
      {listing.seeking ? (
        <form
          class="field-row"
          onSubmit={(event) => {
            event.preventDefault();
            void onChange({ seeking: true, seek_note: note.trim() });
          }}
        >
          <label class="field">
            <span>
              想让人看什么 · {Array.from(note).length}/{SEEK_NOTE_MAX}
            </span>
            <input
              value={note}
              maxLength={SEEK_NOTE_MAX}
              placeholder="新手引导看得懂吗？"
              onInput={(event) => setNote((event.target as HTMLInputElement).value)}
            />
          </label>
          <button class="button" type="submit" disabled={busy || note.trim() === (listing.seek_note ?? "")}>
            保存
          </button>
        </form>
      ) : null}
      <p class="row-actions">
        {listing.seeking ? (
          <button class="button" type="button" disabled={busy} onClick={() => onChange({ seeking: false })}>
            找够了
          </button>
        ) : (
          <button class="button" type="button" disabled={busy} onClick={() => onChange({ seeking: true })}>
            找人测
          </button>
        )}
        <button class="button quiet" type="button" disabled={busy} onClick={() => onChange({ public: false })}>
          撤下
        </button>
        <a class="muted" href={plazaUrl} target="_blank" rel="noreferrer">
          去广场看 ↗
        </a>
      </p>
    </Block>
  );
}

// ------------------------------------------------------------------ 名额

function Seats({ listing, busy, onChange }: { listing: Listing; busy: boolean; onChange: Change }) {
  const [value, setValue] = useState(listing.seats ? String(listing.seats) : "");
  const [wrong, setWrong] = useState<string | null>(null);

  function save(event: Event) {
    event.preventDefault();
    const seats = Number(value.trim());
    if (!Number.isInteger(seats) || seats < 1 || seats > MAX_SEATS) {
      setWrong(`1 到 ${MAX_SEATS} 之间的整数。`);
      return;
    }
    setWrong(null);
    void onChange({ seats });
  }

  return (
    <Block
      title="名额"
      lead={
        listing.seats
          ? `在找 ${listing.seats} 位，已有 ${listing.joined ?? 0} 位加入。`
          : (listing.joined ?? 0) > 0
            ? `没设名额。已有 ${listing.joined} 位留名。`
            : "没设名额。"
      }
    >
      <form class="field-row" onSubmit={save}>
        <label class="field">
          <span>名额</span>
          <input
            type="number"
            min={1}
            max={MAX_SEATS}
            inputMode="numeric"
            value={value}
            placeholder="不限"
            onInput={(event) => setValue((event.target as HTMLInputElement).value)}
          />
        </label>
        <button class="button" type="submit" disabled={busy}>
          保存
        </button>
        {listing.seats ? (
          // 0 就是清掉（common/src/api.rs 的 UpdateSiteRequest）。
          <button
            class="button quiet"
            type="button"
            disabled={busy}
            onClick={() => {
              setValue("");
              setWrong(null);
              void onChange({ seats: 0 });
            }}
          >
            不限
          </button>
        ) : null}
      </form>
      {wrong ? <p class="notice">{wrong}</p> : null}
      <p class="muted">留名即加入。满了仍能玩。</p>
    </Block>
  );
}

// ------------------------------------------------------------------ 开发者的群

function Community({ listing, busy, onChange }: { listing: Listing; busy: boolean; onChange: Change }) {
  const [value, setValue] = useState(listing.community_url ?? "");
  const [wrong, setWrong] = useState<string | null>(null);

  function save(event: Event) {
    event.preventDefault();
    const url = value.trim();
    // 空的就是清掉；剩下的只查开头，去哪是开发者的事，我们对去向不承诺（DESIGN §3.3）。
    if (url !== "" && !/^https?:\/\/\S+$/.test(url)) {
      setWrong("要 http:// 或 https:// 开头的地址。");
      return;
    }
    setWrong(null);
    void onChange({ community_url: url });
  }

  return (
    <Block title="开发者的群" lead="玩家在门禁页和留言之后看到。任何地址都行。">
      <form class="field-row" onSubmit={save}>
        <label class="field grow">
          <span>地址</span>
          <input
            type="url"
            maxLength={MAX_COMMUNITY_URL_CHARS}
            value={value}
            placeholder="https://"
            onInput={(event) => setValue((event.target as HTMLInputElement).value)}
          />
        </label>
        <button class="button" type="submit" disabled={busy}>
          保存
        </button>
        {listing.community_url ? (
          <button
            class="button quiet"
            type="button"
            disabled={busy}
            onClick={() => {
              setValue("");
              setWrong(null);
              void onChange({ community_url: "" });
            }}
          >
            不放
          </button>
        ) : null}
      </form>
      {wrong ? <p class="notice">{wrong}</p> : null}
    </Block>
  );
}

// ------------------------------------------------------------------ 公开反馈

function PublicFeedback({ listing, busy, onChange }: { listing: Listing; busy: boolean; onChange: Change }) {
  return (
    <Block
      title="公开反馈"
      lead={listing.feedback_public ? "开着。门禁页显示最近 3 条，署留下的名字。" : "关着。只有你看得到反馈。"}
    >
      <p class="row-actions">
        <button
          class={`button ${listing.feedback_public ? "" : "primary"}`}
          type="button"
          disabled={busy}
          onClick={() => onChange({ feedback_public: !listing.feedback_public })}
        >
          {listing.feedback_public ? "关掉" : "打开"}
        </button>
      </p>
    </Block>
  );
}

// ------------------------------------------------------------------ 推广

const BOOST_KINDS: Record<string, string> = {
  days3: "推广 3 天",
  days7: "推广 7 天",
  digest: "进本周周报",
};

function BoostSection({ boost }: { boost?: Boost }) {
  return (
    <Block
      title="推广"
      lead={
        boost ? "尚未开放购买。这一段由运营者给出。" : "尚未开放。开放后可买 3 天或 7 天，作品排在广场顶部，标「推广」。"
      }
    >
      {boost ? <BoostState boost={boost} /> : null}
    </Block>
  );
}

function BoostState({ boost }: { boost: Boost }) {
  const kind = BOOST_KINDS[boost.kind] ?? boost.kind;
  switch (boost.status) {
    case "pending":
      return (
        <p>
          <span class="chip">{kind}</span> 排到 {day(boost.starts_at)}，上位前人工复核。
        </p>
      );
    case "live":
      return (
        <p>
          <span class="chip">{kind}</span> 推广中{boost.ends_at ? `，到 ${day(boost.ends_at)}` : ""}。
        </p>
      );
    case "ended":
      return (
        <p class="muted">
          {kind}已结束{boost.ends_at ? `，${day(boost.ends_at)}` : ""}。
        </p>
      );
    case "rejected":
      return (
        <p class="says warn">
          {kind}未通过复核{boost.reason ? `：${boost.reason}` : ""}。全额退款。
        </p>
      );
  }
}

// ------------------------------------------------------------------ 版本

function Versions({ site, onVersionsChanged }: { site: Site; onVersionsChanged: () => void }) {
  const { data, error, loading, reload } = useLoad(() => api.versions(site.slug), [site.slug, site.current_version]);
  const [busy, setBusy] = useState<number | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  async function activate(version: number) {
    if (!confirm(`把 v${version} 设为当前？链接不变。`)) return;
    setBusy(version);
    setProblem(null);
    try {
      await api.activateVersion(site.slug, version);
      onVersionsChanged();
      reload();
    } catch (e) {
      setProblem(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  }

  return (
    <Block title="版本" lead="每次上传是一版，都留着。换版不换链接。">
      {loading ? <Loading /> : null}
      {error ? <Failed error={error} onRetry={reload} /> : null}
      {problem ? <p class="notice">{problem}</p> : null}
      {data && data.versions.length === 0 ? <p class="muted">还没有版本。</p> : null}
      {data && data.versions.length > 0 ? (
        <ul class="version-list">
          {data.versions.map((one) => (
            <li key={one.version} class={one.current ? "current" : ""}>
              <span class="mono ver-no">v{one.version}</span>
              <span class="version-when muted">{moment(one.created_at)}</span>
              <span class="version-size muted mono">
                {one.file_count} 个文件 · {bytes(one.total_bytes)}
              </span>
              <span class="version-note">{one.note ? `「${one.note}」` : ""}</span>
              {one.current ? (
                <span class="chip good">当前</span>
              ) : (
                <button
                  class="button small"
                  type="button"
                  disabled={busy !== null}
                  onClick={() => activate(one.version)}
                >
                  {busy === one.version ? "切换中…" : "设为当前"}
                </button>
              )}
            </li>
          ))}
        </ul>
      ) : null}
    </Block>
  );
}

// ------------------------------------------------------------------ 危险区

function Danger({ site }: { site: Site }) {
  const [typed, setTyped] = useState("");
  const [state, setState] = useState<"idle" | "working" | string>("idle");

  async function remove(event: Event) {
    event.preventDefault();
    if (typed.trim() !== site.slug) return;
    setState("working");
    try {
      await api.deleteSite(site.slug);
      // 回到作品墙。壳上的作品清单由 App 在下一次取的时候刷新；这里直接换地址触发一次。
      go({ name: "sites" });
      location.reload();
    } catch (e) {
      setState(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <section class="block danger">
      <div class="block-head">
        <h2>删除作品</h2>
        <p class="muted">
          链接立刻失效，版本、点名册、反馈一起删除。不可恢复。
        </p>
      </div>
      <form class="field-row" onSubmit={remove}>
        <label class="field">
          <span>
            输入 <code>{site.slug}</code> 确认
          </span>
          <input
            value={typed}
            autocomplete="off"
            spellcheck={false}
            placeholder={site.slug}
            onInput={(event) => setTyped((event.target as HTMLInputElement).value)}
          />
        </label>
        <button class="button danger" type="submit" disabled={typed.trim() !== site.slug || state === "working"}>
          {state === "working" ? "删除中…" : "删除"}
        </button>
      </form>
      {state !== "idle" && state !== "working" ? <p class="notice">{state}</p> : null}
    </section>
  );
}
