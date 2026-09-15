use std::collections::HashSet;

use playtest_common::collection::{Collection, CollectionEntry, CollectionKind};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{clock, db};

pub fn owner(conn: &Connection, slug: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT user_id FROM collections WHERE slug = ?1 AND deleted_at IS NULL",
        [slug],
        |row| row.get(0),
    )
    .optional()
}

pub fn list(conn: &Connection, user: Option<&str>, now: &str) -> rusqlite::Result<Vec<Collection>> {
    let mut statement = conn.prepare(
        "SELECT c.slug, c.title, c.summary, c.kind, c.prompt, c.rules, c.closes_at,
                c.public, c.hidden, u.display_name, c.created_at, c.updated_at
         FROM collections c JOIN users u ON u.id = c.user_id
         WHERE c.deleted_at IS NULL AND
           ((?1 IS NOT NULL AND c.user_id = ?1) OR (?1 IS NULL AND c.public = 1 AND c.hidden = 0))
         ORDER BY c.created_at DESC, c.slug",
    )?;
    let visible: HashSet<String> = db::plaza_candidates(conn, now)?
        .into_iter()
        .filter(|candidate| {
            candidate
                .site
                .expires_at
                .as_deref()
                .is_none_or(|expiry| expiry > now)
        })
        .map(|candidate| candidate.site.slug)
        .collect();
    let mut collections: Vec<Collection> = statement
        .query_map([user], |row| {
            Ok(Collection {
                slug: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                kind: if row.get::<_, String>(3)? == "challenge" {
                    CollectionKind::Challenge
                } else {
                    CollectionKind::Collection
                },
                prompt: row.get(4)?,
                rules: row.get(5)?,
                closes_at: row.get(6)?,
                public: row.get(7)?,
                hidden: row.get(8)?,
                creator: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
                entries: Vec::new(),
                blocked_slugs: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    for collection in &mut collections {
        let mut entries = conn.prepare(
            "SELECT site_slug, submitted_version, submitted_at, note, blocked, sites.title
             FROM collection_entries JOIN sites ON sites.slug=site_slug WHERE collection_slug = ?1 ORDER BY submitted_at DESC, site_slug",
        )?;
        let rows = entries.query_map([&collection.slug], |row| {
            Ok((
                CollectionEntry {
                    slug: row.get(0)?,
                    submitted_version: row.get(1)?,
                    submitted_at: row.get(2)?,
                    title: row.get(5)?,
                    note: row.get(3)?,
                },
                row.get::<_, bool>(4)?,
            ))
        })?;
        for row in rows {
            let (entry, blocked) = row?;
            if blocked {
                if user.is_some() {
                    collection.blocked_slugs.push(entry.slug);
                }
            } else if visible.contains(&entry.slug) {
                collection.entries.push(entry);
            }
        }
    }
    Ok(collections)
}

pub fn public_one(
    conn: &Connection,
    slug: &str,
    now: &str,
) -> rusqlite::Result<Option<Collection>> {
    Ok(list(conn, None, now)?
        .into_iter()
        .find(|collection| collection.slug == slug))
}

pub fn enqueue_confirmation(
    conn: &Connection,
    player: &str,
    title: &str,
    url: &str,
    now: time::OffsetDateTime,
) -> rusqlite::Result<()> {
    let subject = format!("Confirm your follow: {title}");
    let body = format!("You asked to follow {title}. Once you confirm, you get one email a week summarising new entries, and none in a week with no new entries.\n\nThis follows the collection, not each project in it. Authors never see your address, and you can stop any time.\n\n{url}");
    let now = clock::format(now);
    db::enqueue_notification(
        conn,
        &db::NewNotification {
            player_id: player,
            kind: "confirm",
            target_slug: None,
            subject: &subject,
            body: &body,
            url: Some(url),
            channel: "email",
            not_before: &now,
            created_at: &now,
        },
    )?;
    Ok(())
}

pub fn refresh_digest(
    conn: &Connection,
    notification: &mut db::NotificationRow,
    root: &str,
    now: time::OffsetDateTime,
) -> rusqlite::Result<bool> {
    let (slug, queued_at): (String, String) = conn.query_row(
        "SELECT target_slug,created_at FROM notifications WHERE id=?1",
        [notification.id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let now_text = clock::format(now);
    let Some(collection) = public_one(conn, &slug, &now_text)? else {
        return Ok(false);
    };
    if !db::confirmed_followers(conn, "collection", Some(&slug))?
        .iter()
        .any(|recipient| recipient.player_id == notification.player_id && recipient.email.is_some())
    {
        return Ok(false);
    }
    let followed_at: String = conn.query_row("SELECT created_at FROM follows WHERE player_id=?1 AND target_kind='collection' AND target_slug=?2", params![notification.player_id,slug], |row| row.get(0))?;
    let Some(queued_time) = clock::parse(&queued_at) else {
        return Ok(false);
    };
    let since = clock::format(queued_time - time::Duration::days(7));
    let entries: Vec<_> = collection
        .entries
        .iter()
        .filter(|entry| {
            entry.submitted_at > since
                && entry.submitted_at > followed_at
                && entry.submitted_at <= queued_at
        })
        .collect();
    if entries.is_empty() {
        return Ok(false);
    }
    notification.subject = new_entries_subject(&collection.title, entries.len());
    notification.body = new_entries_intro(&collection.title);
    for entry in entries.iter().take(10) {
        notification.body.push_str(&format!(
            "\n{}\n{}/p/{}?collection={}&from=notice\n",
            entry.title,
            root.trim_end_matches('/'),
            entry.slug,
            slug
        ));
    }
    notification.url = Some(format!(
        "{}/c/{slug}?from=notice",
        root.trim_end_matches('/')
    ));
    notification.body.push_str(&format!(
        "\nSee the whole collection: {}\n",
        notification.url.as_deref().unwrap_or_default()
    ));
    Ok(true)
}

/// 合集摘要那封信的主题与开头。入队和重新核对两条路都用它，免得两处各写一句。
fn new_entries_subject(title: &str, count: usize) -> String {
    // entry 的复数不是加 s，words::count 只管规则名词，这句自己说。
    let entries = if count == 1 { "entry" } else { "entries" };
    format!("{title}: {count} new {entries}")
}

fn new_entries_intro(title: &str) -> String {
    format!("New entries in {title}, a collection you follow:\n")
}

pub fn enqueue_digests(
    conn: &Connection,
    now: time::OffsetDateTime,
    root: &str,
) -> rusqlite::Result<usize> {
    let now_text = clock::format(now);
    let since = clock::format(now - time::Duration::days(7));
    let local = now.to_offset(time::UtcOffset::from_hms(8, 0, 0).unwrap());
    let (year, week, _) = local.to_iso_week_date();
    let period = format!("{year}-{week:02}");
    let mut queued = 0;
    for collection in list(conn, None, &now_text)? {
        for recipient in db::confirmed_followers(conn, "collection", Some(&collection.slug))? {
            if recipient.email.is_none() {
                continue;
            }
            let followed_at: String = conn.query_row(
                "SELECT created_at FROM follows WHERE player_id=?1 AND target_kind='collection' AND target_slug=?2",
                params![recipient.player_id, collection.slug], |row| row.get(0),
            )?;
            let entries: Vec<_> = collection
                .entries
                .iter()
                .filter(|entry| entry.submitted_at > since && entry.submitted_at > followed_at)
                .collect();
            if entries.is_empty() {
                continue;
            }
            let transaction = conn.unchecked_transaction()?;
            if transaction.execute(
                "INSERT OR IGNORE INTO collection_digests(collection_slug, player_id, period) VALUES(?1,?2,?3)",
                params![collection.slug, recipient.player_id, period],
            )? == 0 { continue; }
            let url = format!(
                "{}{}/{}?from=notice",
                root.trim_end_matches('/'),
                "/c",
                collection.slug
            );
            let subject = new_entries_subject(&collection.title, entries.len());
            let mut body = new_entries_intro(&collection.title);
            for entry in entries.iter().take(10) {
                if let Some(site) = db::find_site(&transaction, &entry.slug)? {
                    body.push_str(&format!(
                        "\n{}\n{}/p/{}?collection={}&from=notice\n",
                        site.title,
                        root.trim_end_matches('/'),
                        entry.slug,
                        collection.slug
                    ));
                }
            }
            body.push_str(&format!(
                "\nSee the whole collection: {url}\nNotes are written by the authors. This is not a ranking.\n"
            ));
            db::enqueue_notification(
                &transaction,
                &db::NewNotification {
                    player_id: &recipient.player_id,
                    kind: "collection_digest",
                    target_slug: Some(&collection.slug),
                    subject: &subject,
                    body: &body,
                    url: Some(&url),
                    channel: "email",
                    not_before: &now_text,
                    created_at: &now_text,
                },
            )?;
            transaction.commit()?;
            queued += 1;
        }
    }
    Ok(queued)
}
