use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

/// Bootstrap the database schema via versioned migrations.
pub async fn bootstrap(db: &Surreal<Db>) -> Result<()> {
    super::migrations::run_all(db).await
}

/// Read a metadata value by key.
pub async fn get_metadata(db: &Surreal<Db>, key: &str) -> Result<Option<String>> {
    let mut result = db
        .query("SELECT `value` FROM db_metadata WHERE key = $key")
        .bind(("key", key))
        .await?;
    let row: Option<String> = result.take("value")?;
    Ok(row)
}

/// Write a metadata value by key.
pub async fn set_metadata(db: &Surreal<Db>, key: &str, value: &str) -> Result<()> {
    db.query("UPSERT db_metadata SET `value` = $value, updated_at = time::now() WHERE key = $key")
        .bind(("key", key))
        .bind(("value", value))
        .await?;
    Ok(())
}

/// Ensure the HNSW vector index matches the expected dimension.
///
/// If the dimension has changed, drops the old index and all chunks,
/// then recreates with the new dimension.
pub async fn ensure_hnsw_index(db: &Surreal<Db>, dimension: usize) -> Result<()> {
    let current = get_metadata(db, "hnsw_dimension").await?;
    if current.as_deref() == Some(&dimension.to_string()) {
        return Ok(());
    }

    tracing::info!(
        old = ?current,
        new = dimension,
        "recreating HNSW index for new embedding dimension"
    );

    // Remove old index if it exists
    let _ = db
        .query("REMOVE INDEX IF EXISTS hnsw_chunk_embedding ON chunk")
        .await;

    // Clear old chunks with different dimension
    db.query("DELETE chunk").await?;

    // Create new HNSW index
    let sql = format!(
        "DEFINE INDEX hnsw_chunk_embedding ON chunk FIELDS embedding HNSW DIMENSION {} DIST COSINE TYPE F32 EFC 150 M 12",
        dimension
    );
    db.query(&sql).await?;

    set_metadata(db, "hnsw_dimension", &dimension.to_string()).await?;
    Ok(())
}
