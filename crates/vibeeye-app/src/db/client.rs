use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::opt::auth::Root;

use super::schema;

/// Wrapper around a SurrealDB connection (local SurrealKV, remote WS/HTTP, or memory).
#[derive(Debug, Clone)]
pub struct DbClient {
    inner: Surreal<Any>,
}

impl DbClient {
    /// Connect to a SurrealDB instance via a URL.
    ///
    /// Supported schemes:
    /// - `surrealkv://path` – embedded file-based database
    /// - `ws://user:pass@host:port` / `wss://...` – remote WebSocket
    /// - `http://user:pass@host:port` / `https://...` – remote HTTP
    /// - `mem://` – in-memory (ephemeral)
    pub async fn connect(url: &str) -> Result<Self> {
        let db = surrealdb::engine::any::connect(url).await?;

        // For remote endpoints, extract credentials from the URL and sign in.
        // SurrealDB's Any engine establishes the connection but does not
        // automatically authenticate using URL credentials.
        if let Ok(parsed) = url::Url::parse(url) {
            let scheme = parsed.scheme();
            if matches!(scheme, "ws" | "wss" | "http" | "https") {
                if let Some(password) = parsed.password() {
                    let username = if parsed.username().is_empty() {
                        None
                    } else {
                        Some(parsed.username())
                    };
                    if let Some(username) = username {
                        db.signin(Root {
                            username: username.to_owned(),
                            password: password.to_owned(),
                        })
                        .await?;
                    }
                }
            }
        }

        Ok(Self { inner: db })
    }

    /// Connect to an in-memory database (for tests).
    pub async fn connect_mem() -> Result<Self> {
        let db = surrealdb::engine::any::connect("mem://").await?;
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
        // Use indexed SELECT subqueries because DELETE WHERE does not use
        // indexes as of SurrealDB v3 (fixed planned for v2.3.0+).
        self.inner
            .query("DELETE (SELECT id FROM page WHERE `group` = $group)")
            .bind(("group", group.to_string()))
            .await?;
        self.inner
            .query("DELETE (SELECT id FROM discovered WHERE `group` = $group)")
            .bind(("group", group.to_string()))
            .await?;
        #[cfg(feature = "embeddings")]
        self.inner
            .query("DELETE (SELECT id FROM chunk WHERE `group` = $group)")
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
    type Target = Surreal<Any>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
