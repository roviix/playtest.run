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

function Block({ title, lead, children }: { title: string; lead?: string; children: ComponentChildren }) {
  return (
    <section class="block">
      <div class="block-head">
        <h2>{title}</h2>
        {lead ? <p class="muted">{lead}</p> : null}
      </div>
      <div class="block-body">{children}</div>
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
      <Block title="广场" lead="上传第一版之后才能放到广场上。">
        <p class="muted">广场是玩家不用注册就能翻的地方，只放开发者主动公开的作品。</p>
      </Block>
    );
  }

  if (!listing.public) {
    return (
      <Block title="广场" lead="现在只有拿到链接的人能玩。">
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
            放上去，找人测
          </button>
          <a class="muted" href={plazaUrl} target="_blank" rel="noreferrer">
            广场是什么 ↗
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
          ? "被多人举报，已从广场撤下等复核。链接照常能开。"
          : `在广场上${listing.seeking ? "，正在找人测" : ""}。${listing.has_cover ? "" : "加 --cover 可以配一张封面。"}`
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
              想让来的人重点看什么（{Array.from(note).length}/{SEEK_NOTE_MAX}）
            </span>
            <input
              value={note}
              maxLength={SEEK_NOTE_MAX}
              placeholder="例如：新手引导看得懂吗？第三关难不难？"
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
            人找够了
          </button>
        ) : (
          <button class="button" type="button" disabled={busy} onClick={() => onChange({ seeking: true })}>
            标「正在找人测」
          </button>
        )}
        <button class="button quiet" type="button" disabled={busy} onClick={() => onChange({ public: false })}>
          从广场上拿下来
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
      setWrong(`要一个 1 到 ${MAX_SEATS} 之间的整数。想取消就点「不限」。`);
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
          ? `在找 ${listing.seats} 位，已有 ${listing.joined} 位加入。`
          : listing.joined > 0
            ? `没设名额。已有 ${listing.joined} 位留了名字。`
            : "没设名额，门禁页上不出现这一行。"
      }
    >
      <form class="field-row" onSubmit={save}>
        <label class="field">
          <span>想找几位试玩者</span>
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
      <p class="muted">留了名字的人算加入；到齐之后玩家仍然可以玩。</p>
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
      setWrong("要一个 http:// 或 https:// 开头的地址。留空就是不放。");
      return;
    }
    setWrong(null);
    void onChange({ community_url: url });
  }

  return (
    <Block title="开发者的群" lead="玩家在门禁页和反馈之后看到「开发者的群」。QQ 群、微信群的二维码页、Discord……去哪都行。">
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
      title="让玩家看到彼此的反馈"
      lead={
        listing.feedback_public
          ? "开着：门禁页上显示最近 3 条，署他们留的名字；哪一条不想让人看到，在「反馈」里逐条隐藏。"
          : "关着：只有你看得到反馈。打开之后门禁页上会显示最近 3 条。"
      }
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
        boost
          ? "推广位的买卖尚未开放：付款通道要等主体落地。这一段是运营者给的。"
          : "尚未开放：付款通道要等主体落地。开放后可以买 3 天或 7 天，作品会出现在广场顶部并标「推广」，免费流不受影响。"
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
          <span class="chip">{kind}</span> 排到 {day(boost.starts_at)}。上推广位之前我们会人工看一眼。
        </p>
      );
    case "live":
      return (
        <p>
          <span class="chip">{kind}</span> 推广中{boost.ends_at ? `，到 ${day(boost.ends_at)}` : ""}
          。现在它在广场顶部，标着「推广」。
        </p>
      );
    case "ended":
      return (
        <p class="muted">
          {kind}已经结束{boost.ends_at ? `（${day(boost.ends_at)}）` : ""}。
        </p>
      );
    case "rejected":
      return (
        <p class="says warn">
          {kind}没通过人工复核{boost.reason ? `：${boost.reason}` : ""}。付过的钱会全额退。
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
    if (!confirm(`让玩家看到的换回 v${version}？链接不变，点开就是那一版。`)) return;
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
    <Block title="版本" lead="每次上传就是一版，都留着；换哪一版给玩家看，链接都不变。">
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
                <span class="chip good">玩家看到的</span>
              ) : (
                <button
                  class="button small"
                  type="button"
                  disabled={busy !== null}
                  onClick={() => activate(one.version)}
                >
                  {busy === one.version ? "正在换…" : "让玩家看这一版"}
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
          链接立刻失效，所有版本、点名册、反馈一起消失，关注这个作品的人也不再收到通知。删了就找不回来。
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
          {state === "working" ? "正在删…" : "删除这个作品"}
        </button>
      </form>
      {state !== "idle" && state !== "working" ? <p class="notice">{state}</p> : null}
    </section>
  );
}
