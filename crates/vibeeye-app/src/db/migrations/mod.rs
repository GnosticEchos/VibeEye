//! Versioned schema migrations using embedded `.surql` files.

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

pub const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", include_str!("001_initial.surql")),
    ("002_add_chunks", include_str!("002_add_chunks.surql")),
];

/// Run all pending migrations in order.
pub async fn run_all(db: &Surreal<Db>) -> Result<()> {
    // Ensure metadata table exists so we can track version
    db.query(
        r#"
        DEFINE TABLE db_metadata TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;
        DEFINE FIELD key ON db_metadata TYPE string ASSERT $value != NONE PERMISSIONS FULL;
        DEFINE FIELD value ON db_metadata TYPE string PERMISSIONS FULL;
        DEFINE FIELD updated_at ON db_metadata TYPE datetime DEFAULT time::now() PERMISSIONS FULL;
        DEFINE INDEX idx_metadata_key ON db_metadata FIELDS key UNIQUE;
        "#,
    )
    .await?;

    let current = get_version(db).await?;

    for (name, sql) in MIGRATIONS {
        // Compare versions lexicographically (001 < 002)
        if name > &current.as_str() {
            tracing::info!(migration = name, "applying schema migration");
            db.query(*sql).await?;
            set_version(db, name).await?;
        }
    }

    Ok(())
}

async fn get_version(db: &Surreal<Db>) -> Result<String> {
    let mut result = db
        .query("SELECT `value` FROM db_metadata WHERE key = 'schema_version'")
        .await?;
    let row: Option<String> = result.take("value")?;
    Ok(row.unwrap_or_default())
}

async fn set_version(db: &Surreal<Db>, version: &str) -> Result<()> {
    db.query(
        "UPSERT db_metadata SET value = $version, updated_at = time::now() WHERE key = 'schema_version'"
    )
    .bind(("version", version))
    .await?;
    Ok(())
}
