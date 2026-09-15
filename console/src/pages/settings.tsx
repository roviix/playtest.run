// 设置（DESIGN §3.13）：广场、名额、开发者的群、公开反馈、推广、版本与回滚、危险区。
//
// 这些是偶尔改一次的东西，所以在最后一个签里，不压在结果上面。
// 关注只有数字，且已经在身份栏的签上——开发者永远看不到玩家的邮箱（DESIGN §3.6），
// 那不是偷懒，是我们替他扛下了一份玩家邮箱的合规责任。
// 推广那一节没有任何按钮：付款通道要等主体落地，放一个点不动的「购买」比什么都不放更伤人。

import type { ComponentChildren } from "preact";
import { useEffect, useState } from "preact/hooks";

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
      <WorkDetails site={site} busy={busy} onChange={change} />
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

// ------------------------------------------------------------------ 基本信息

function WorkDetails({
  site,
  busy,
  onChange,
}: {
  site: Site;
  busy: boolean;
  onChange: Change;
}) {
  const [title, setTitle] = useState(site.title);
  const [summary, setSummary] = useState(site.listing?.summary ?? "");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    setTitle(site.title);
    setSummary(site.listing?.summary ?? "");
  }, [site.title, site.listing?.summary]);

  const hasChanged =
    title.trim() !== site.title ||
    summary.trim() !== (site.listing?.summary ?? "");

  async function submit(event: Event) {
    event.preventDefault();
    if (!title.trim() || !hasChanged) return;
    await onChange({
      title: title.trim(),
      summary: summary.trim(),
    });
    setSaved(true);
    setTimeout(() => setSaved(false), 2500);
  }

  return (
    <Block
      title="Basics"
      lead="Project title and one-line summary. Changes show up on the door page and the Plaza card right away."
    >
      <form class="work-meta-form" onSubmit={submit}>
        <div class="field-grid">
          <label class="field">
            <span>Title</span>
            <input
              value={title}
              required
              maxLength={80}
              placeholder="Project title"
              onInput={(event) => setTitle((event.target as HTMLInputElement).value)}
            />
          </label>
          <label class="field">
            <span>Summary · {Array.from(summary).length}/140</span>
            <input
              value={summary}
              maxLength={140}
              placeholder="One line about this project, shown on Plaza and the invite card"
              onInput={(event) => setSummary((event.target as HTMLInputElement).value)}
            />
          </label>
        </div>

        <div class="meta-footer">
          <div class="cover-status-badge">
            <span class={`now-dot ${site.listing?.has_cover ? "" : "idle"}`} aria-hidden="true" />
            <span class="muted">
              {site.listing?.has_cover
                ? "Cover image set · replace it with playtest ./dist --cover <file>"
                : "No cover image yet, a monogram card is used instead · add one with playtest ./dist --cover <file>"}
            </span>
          </div>
          <div class="meta-action">
            {saved ? <span class="save-toast">✓ Saved</span> : null}
            <button
              class="button primary"
              type="submit"
              disabled={busy || !hasChanged || !title.trim()}
            >
              {busy ? "Saving…" : "Save"}
            </button>
          </div>
        </div>
      </form>
    </Block>
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
      <Block title="Plaza" lead="Available for Plaza listing after uploading your first version." />
    );
  }

  if (!listing.public) {
    return (
      <Block title="Plaza" lead="Unlisted. Only people with the link can play.">
        <p class="row-actions">
          <button class="button" type="button" disabled={busy} onClick={() => onChange({ public: true })}>
            Publish to Plaza
          </button>
          <button
            class="button primary"
            type="button"
            disabled={busy}
            onClick={() => onChange({ public: true, seeking: true })}
          >
            List to Recruit
          </button>
          <a class="muted" href={plazaUrl} target="_blank" rel="noreferrer">
            Plaza ↗
          </a>
        </p>
      </Block>
    );
  }

  return (
    <Block
      title="Plaza"
      lead={
        listing.hidden
          ? "Multiple reports received, removed from Plaza pending review. Direct link still works."
          : `Listed on Plaza${listing.seeking ? ", recruiting playtesters" : ""}.`
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
              What to look for · {Array.from(note).length}/{SEEK_NOTE_MAX}
            </span>
            <input
              value={note}
              maxLength={SEEK_NOTE_MAX}
              placeholder="Can you understand the tutorial?"
              onInput={(event) => setNote((event.target as HTMLInputElement).value)}
            />
          </label>
          <button class="button" type="submit" disabled={busy || note.trim() === (listing.seek_note ?? "")}>
            Save
          </button>
        </form>
      ) : null}
      <p class="row-actions">
        {listing.seeking ? (
          <button class="button" type="button" disabled={busy} onClick={() => onChange({ seeking: false })}>
            Stop Recruiting
          </button>
        ) : (
          <button class="button" type="button" disabled={busy} onClick={() => onChange({ seeking: true })}>
            Recruit Playtesters
          </button>
        )}
        <button class="button quiet" type="button" disabled={busy} onClick={() => onChange({ public: false })}>
          Unlist
        </button>
        <a class="muted" href={plazaUrl} target="_blank" rel="noreferrer">
          View on Plaza ↗
        </a>
      </p>
    </Block>
  );
}

// ------------------------------------------------------------------ Seats

function Seats({ listing, busy, onChange }: { listing: Listing; busy: boolean; onChange: Change }) {
  const [value, setValue] = useState(listing.seats ? String(listing.seats) : "");
  const [wrong, setWrong] = useState<string | null>(null);

  function save(event: Event) {
    event.preventDefault();
    const seats = Number(value.trim());
    if (!Number.isInteger(seats) || seats < 1 || seats > MAX_SEATS) {
      setWrong(`Must be an integer between 1 and ${MAX_SEATS}.`);
      return;
    }
    setWrong(null);
    void onChange({ seats });
  }

  return (
    <Block
      title="Seats"
      lead={
        listing.seats
          ? `Seeking ${listing.seats} playtesters, ${listing.joined ?? 0} joined so far.`
          : (listing.joined ?? 0) > 0
            ? `No seat limit. ${listing.joined} joined so far.`
            : "No seat limit."
      }
    >
      <form class="field-row" onSubmit={save}>
        <label class="field">
          <span>Seats</span>
          <input
            type="number"
            min={1}
            max={MAX_SEATS}
            inputMode="numeric"
            value={value}
            placeholder="Unlimited"
            onInput={(event) => setValue((event.target as HTMLInputElement).value)}
          />
        </label>
        <button class="button" type="submit" disabled={busy}>
          Save
        </button>
        {listing.seats ? (
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
            Unlimited
          </button>
        ) : null}
      </form>
      {wrong ? <p class="notice">{wrong}</p> : null}
      <p class="muted">Leaving a name counts as joined. Playable even when full.</p>
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
    if (url !== "" && !/^https?:\/\/\S+$/.test(url)) {
      setWrong("URL must start with http:// or https://");
      return;
    }
    setWrong(null);
    void onChange({ community_url: url });
  }

  return (
    <Block title="Community" lead="Shown on the Door page and after leaving feedback. Any valid URL accepted.">
      <form class="field-row" onSubmit={save}>
        <label class="field grow">
          <span>URL</span>
          <input
            type="url"
            maxLength={MAX_COMMUNITY_URL_CHARS}
            value={value}
            placeholder="https://"
            onInput={(event) => setValue((event.target as HTMLInputElement).value)}
          />
        </label>
        <button class="button" type="submit" disabled={busy}>
          Save
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
            None
          </button>
        ) : null}
      </form>
      {wrong ? <p class="notice">{wrong}</p> : null}
    </Block>
  );
}

// ------------------------------------------------------------------ Public Feedback

function PublicFeedback({ listing, busy, onChange }: { listing: Listing; busy: boolean; onChange: Change }) {
  return (
    <Block
      title="Public Feedback"
      lead={listing.feedback_public ? "Enabled. Door page displays the 3 latest entries with author names." : "Disabled. Only you can view feedback."}
    >
      <p class="row-actions">
        <button
          class={`button ${listing.feedback_public ? "" : "primary"}`}
          type="button"
          disabled={busy}
          onClick={() => onChange({ feedback_public: !listing.feedback_public })}
        >
          {listing.feedback_public ? "Disable" : "Enable"}
        </button>
      </p>
    </Block>
  );
}

// ------------------------------------------------------------------ Boost

const BOOST_KINDS: Record<string, string> = {
  days3: "3-Day Boost",
  days7: "7-Day Boost",
  digest: "Weekly Digest Feature",
};

function BoostSection({ boost }: { boost?: Boost }) {
  return (
    <Block
      title="Boost"
      lead={
        boost ? "Not open for purchase yet. Managed by platform operators." : "Coming soon. Featured placement at top of Plaza with a 'Boosted' badge."
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
          <span class="chip">{kind}</span> scheduled for {day(boost.starts_at)}, manual review before going live.
        </p>
      );
    case "live":
      return (
        <p>
          <span class="chip">{kind}</span> active{boost.ends_at ? `, until ${day(boost.ends_at)}` : ""}.
        </p>
      );
    case "ended":
      return (
        <p class="muted">
          {kind} ended{boost.ends_at ? `, ${day(boost.ends_at)}` : ""}.
        </p>
      );
    case "rejected":
      return (
        <p class="says warn">
          {kind} rejected{boost.reason ? `: ${boost.reason}` : ""}. Full refund issued.
        </p>
      );
  }
}

// ------------------------------------------------------------------ Versions

function Versions({ site, onVersionsChanged }: { site: Site; onVersionsChanged: () => void }) {
  const { data, error, loading, reload } = useLoad(() => api.versions(site.slug), [site.slug, site.current_version]);
  const [busy, setBusy] = useState<number | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  async function activate(version: number) {
    if (!confirm(`Set v${version} as current? Link stays the same.`)) return;
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
    <Block title="Versions" lead="Every upload creates a new version. Past versions are preserved. Rollback keeps the same link.">
      {loading ? <Loading /> : null}
      {error ? <Failed error={error} onRetry={reload} /> : null}
      {problem ? <p class="notice">{problem}</p> : null}
      {data && data.versions.length === 0 ? <p class="muted">No versions yet.</p> : null}
      {data && data.versions.length > 0 ? (
        <ul class="version-list">
          {data.versions.map((one) => (
            <li key={one.version} class={one.current ? "current" : ""}>
              <span class="mono ver-no">v{one.version}</span>
              <span class="version-when muted">{moment(one.created_at)}</span>
              <span class="version-size muted mono">
                {one.file_count} {one.file_count === 1 ? "file" : "files"} · {bytes(one.total_bytes)}
              </span>
              <span class="version-note">{one.note ? `“${one.note}”` : ""}</span>
              {one.current ? (
                <span class="chip good">Current</span>
              ) : (
                <button
                  class="button small"
                  type="button"
                  disabled={busy !== null}
                  onClick={() => activate(one.version)}
                >
                  {busy === one.version ? "Switching…" : "Set as Current"}
                </button>
              )}
            </li>
          ))}
        </ul>
      ) : null}
    </Block>
  );
}

// ------------------------------------------------------------------ Danger Zone

function Danger({ site }: { site: Site }) {
  const [typed, setTyped] = useState("");
  const [state, setState] = useState<"idle" | "working" | string>("idle");

  async function remove(event: Event) {
    event.preventDefault();
    if (typed.trim() !== site.slug) return;
    setState("working");
    try {
      await api.deleteSite(site.slug);
      go({ name: "sites" });
      location.reload();
    } catch (e) {
      setState(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <section class="block danger">
      <div class="block-head">
        <h2>Delete Project</h2>
        <p class="muted">
          The link will become invalid immediately. Versions, roster, and feedback will be permanently deleted. This cannot be undone.
        </p>
      </div>
      <form class="field-row" onSubmit={remove}>
        <label class="field">
          <span>
            Enter <code>{site.slug}</code> to confirm
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
          {state === "working" ? "Deleting…" : "Delete"}
        </button>
      </form>
      {state !== "idle" && state !== "working" ? <p class="notice">{state}</p> : null}
    </section>
  );
}
