use playtest_common::plan::Plan;
use playtest_common::quota::Policy;
use rusqlite::{params, OptionalExtension};

use crate::state::AppState;

pub async fn publish(state: &AppState, slug: &str) -> anyhow::Result<()> {
    let policy = {
        let connection = state.db().read().await;
        connection.query_row(
            "SELECT s.user_id, u.kind, s.expires_at FROM sites s JOIN users u ON u.id = s.user_id WHERE s.slug = ?1 AND s.deleted_at IS NULL",
            params![slug],
            |row| Ok(Policy {
                owner: row.get(0)?,
                plan: if row.get::<_, String>(1)? == "github" { Plan::Free } else { Plan::Anon },
                expires_at: row.get(2)?,
            }),
        ).optional()?
    };
    if let Some(policy) = policy {
        state.store().put_policy(slug, &policy).await?;
    }
    Ok(())
}

pub async fn reconcile(state: &AppState) -> anyhow::Result<()> {
    let slugs = {
        let connection = state.db().read().await;
        let mut statement =
            connection.prepare("SELECT slug FROM sites WHERE deleted_at IS NULL")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for slug in slugs {
        publish(state, &slug).await?;
    }
    Ok(())
}
