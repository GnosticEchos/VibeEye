use std::path::Path;

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use super::schema;

/// Wrapper around a SurrealDB local connection.
#[derive(Debug, Clone)]
pub struct DbClient {
    inner: Surreal<Db>,
}

impl DbClient {
    /// Connect to an embedded SurrealKV database.
    pub async fn connect(path: &Path) -> Result<Self> {
        let db =
            Surreal::new::<surrealdb::engine::local::SurrealKv>(path.to_string_lossy().as_ref())
                .await?;
        Ok(Self { inner: db })
    }

    /// Connect to an in-memory database (for tests).
    pub async fn connect_mem() -> Result<Self> {
        let db = Surreal::new::<surrealdb::engine::local::Mem>(()).await?;
        Ok(Self { inner: db })
    }

    /// Select namespace and database.
    pub async fn use_ns_db(&self, ns: &str, db: &str) -> Result<()> {
        self.inner.use_ns(ns).use_db(db).await?;
        Ok(())
    }

    /// Bootstrap schema if not already present.
    pub async fn bootstrap(&self) -> Result<()> {
        schema::bootstrap(&self.inner).await
    }

    /// Remove all data for a specific group.
    pub async fn reset_group(&self, group: &str) -> Result<()> {
        self.inner
            .query("DELETE page WHERE group = $group")
            .bind(("group", group.to_string()))
            .await?;
        self.inner
            .query("DELETE discovered WHERE group = $group")
            .bind(("group", group.to_string()))
            .await?;
        #[cfg(feature = "embeddings")]
        self.inner
            .query("DELETE chunk WHERE group = $group")
            .bind(("group", group.to_string()))
            .await?;
        Ok(())
    }

    /// Reset everything (all groups). Used with extreme caution.
    pub async fn reset_all(&self) -> Result<()> {
        self.inner.query("REMOVE TABLE IF EXISTS page").await?;
        self.inner
            .query("REMOVE TABLE IF EXISTS discovered")
            .await?;
        #[cfg(feature = "embeddings")]
        self.inner.query("REMOVE TABLE IF EXISTS chunk").await?;
        self.inner
            .query("DELETE db_metadata WHERE key = 'schema_version'")
            .await?;
        Ok(())
    }
}

impl std::ops::Deref for DbClient {
    type Target = Surreal<Db>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
