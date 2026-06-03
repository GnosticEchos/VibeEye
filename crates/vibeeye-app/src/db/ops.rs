use anyhow::Result;

use crate::db::DbClient;
use crate::db::models::{LinkRecord, PageRecord, QueryResult};

/// Extract a snippet around the first match of any query term.
/// Falls back to the first `max_len` characters if no match is found.
fn find_first_term_pos(content_lower: &str, terms: &[&str]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for term in terms {
        let term_lower = term.to_lowercase();
        if let Some(pos) = content_lower.find(&term_lower) {
            best = Some(best.map_or(pos, |b| b.min(pos)));
        }
    }
    best
}

fn compute_snippet_bounds(start: usize, content_len: usize, max_len: usize) -> (usize, usize) {
    let half = max_len / 2;
    let snippet_start = start.saturating_sub(half);
    let snippet_end = (snippet_start + max_len).min(content_len);
    let adjusted_start = if snippet_end - snippet_start < max_len && content_len > max_len {
        content_len.saturating_sub(max_len)
    } else {
        snippet_start
    };
    (adjusted_start, snippet_end)
}

fn extract_match_snippet(content: &str, query: &str, max_len: usize) -> String {
    let terms: Vec<&str> = query
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|t| !t.is_empty())
        .collect();

    let content_lower = content.to_lowercase();
    let start = find_first_term_pos(&content_lower, &terms).unwrap_or(0);
    let (adjusted_start, snippet_end) = compute_snippet_bounds(start, content.len(), max_len);

    let mut snippet = content[adjusted_start..snippet_end].to_string();
    if adjusted_start > 0 {
        snippet.insert_str(0, "...");
    }
    if snippet_end < content.len() {
        snippet.push_str("...");
    }
    snippet
}

impl DbClient {
    /// Insert or update a crawled page record.
    ///
    /// SurrealDB v3 UPSERT over WebSocket returns empty results with parameter
    /// binding, so we SELECT first, then CREATE or UPDATE.
    pub async fn insert_page(&self, record: &PageRecord) -> Result<surrealdb::types::RecordId> {
        if let Some(id) = self.find_page_id(&record.url, &record.group).await? {
            self.update_page(id.clone(), record).await?;
            return Ok(id);
        }
        self.create_page(record).await
    }

    /// Look up an existing page record by URL and group.
    async fn find_page_id(
        &self,
        url: &str,
        group: &str,
    ) -> Result<Option<surrealdb::types::RecordId>> {
        let mut result = self
            .query("SELECT id FROM page WHERE url = $url AND `group` = $group LIMIT 1")
            .bind(("url", url.to_string()))
            .bind(("group", group.to_string()))
            .await?;
        let existing: Vec<serde_json::Value> = result.take(0)?;
        let Some(id_val) = existing.into_iter().next() else {
            return Ok(None);
        };
        let id_str = id_val["id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("invalid id after select"))?;
        Ok(Some(surrealdb::types::RecordId::parse_simple(&id_str)?))
    }

    /// Update an existing page record.
    async fn update_page(
        &self,
        id: surrealdb::types::RecordId,
        record: &PageRecord,
    ) -> Result<()> {
        self.query(
            "UPDATE $id SET `group` = $group, url = $url, title = $title,
             content = $content, depth = $depth, format = $format,
             crawled_at = $crawled_at, meta = $meta",
        )
        .bind(("id", id))
        .bind(("group", record.group.clone()))
        .bind(("url", record.url.clone()))
        .bind(("title", record.title.clone()))
        .bind(("content", record.content.clone()))
        .bind(("depth", record.depth))
        .bind(("format", record.format.clone()))
        .bind(("crawled_at", record.crawled_at))
        .bind(("meta", record.meta.clone()))
        .await?;
        Ok(())
    }

    /// Create a new page record.
    async fn create_page(&self, record: &PageRecord) -> Result<surrealdb::types::RecordId> {
        let mut result = self
            .query(
                "CREATE page SET `group` = $group, url = $url, title = $title,
                 content = $content, depth = $depth, format = $format,
                 crawled_at = $crawled_at, meta = $meta",
            )
            .bind(("group", record.group.clone()))
            .bind(("url", record.url.clone()))
            .bind(("title", record.title.clone()))
            .bind(("content", record.content.clone()))
            .bind(("depth", record.depth))
            .bind(("format", record.format.clone()))
            .bind(("crawled_at", record.crawled_at))
            .bind(("meta", record.meta.clone()))
            .await?;
        let raw: Vec<serde_json::Value> = result.take(0)?;
        let id_str = raw
            .into_iter()
            .next()
            .and_then(|v| v["id"].as_str().map(|s| s.to_string()))
            .ok_or_else(|| anyhow::anyhow!("page not found after create"))?;
        let id = surrealdb::types::RecordId::parse_simple(&id_str)?;
        Ok(id)
    }

    /// Insert discovered links (graph edges) in a single batched query.
    ///
    /// Creates the relations `page -> discovered -> page` with group tag.
    pub async fn insert_discovered(&self, links: &[LinkRecord]) -> Result<()> {
        if links.is_empty() {
            return Ok(());
        }
        let mut statements = Vec::with_capacity(links.len());
        for (i, _) in links.iter().enumerate() {
            statements.push(format!(
                "RELATE $from{i}->discovered->$to{i} SET `group` = $group{i}, anchor_text = $anchor_text{i}, discovered_at = $discovered_at{i}"
            ));
        }
        let sql = statements.join(";");
        let mut query = self.query(&sql);
        for (i, link) in links.iter().enumerate() {
            query = query.bind((format!("from{i}"), link.from_page.clone()));
            query = query.bind((format!("to{i}"), link.to_page.clone()));
            query = query.bind((format!("group{i}"), link.group.clone()));
            query = query.bind((format!("anchor_text{i}"), link.anchor_text.clone()));
            query = query.bind((format!("discovered_at{i}"), link.discovered_at));
        }
        query.await?;
        Ok(())
    }

    /// Perform a BM25 full-text search over page content.
    ///
    /// If `group` is `Some`, restricts to that group. Returns top `limit`
    /// results ordered by BM25 score descending.
    pub async fn bm25_search(
        &self,
        group: Option<&str>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<QueryResult>> {
        let raw: Vec<serde_json::Value> = if let Some(g) = group {
            self.query(
                "SELECT url, title, content,
                 search::score(0) AS score
                 FROM page WHERE group = $group AND content @@ $query
                 ORDER BY score DESC LIMIT $limit",
            )
            .bind(("group", g.to_string()))
            .bind(("query", query.to_string()))
            .bind(("limit", limit))
            .await?
            .take(0)?
        } else {
            self.query(
                "SELECT url, title, content,
                 search::score(0) AS score
                 FROM page WHERE content @@ $query
                 ORDER BY score DESC LIMIT $limit",
            )
            .bind(("query", query.to_string()))
            .bind(("limit", limit))
            .await?
            .take(0)?
        };
        let results: Vec<QueryResult> = raw
            .into_iter()
            .filter_map(|mut v| {
                if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                    let snippet = extract_match_snippet(content, query, 300);
                    v["snippet"] = serde_json::Value::String(snippet);
                    v.as_object_mut()?.remove("content");
                }
                serde_json::from_value(v).ok()
            })
            .collect();
        Ok(results)
    }

    /// Update the structured metadata for a page by URL and group.
    pub async fn update_page_meta(
        &self,
        group: &str,
        url: &str,
        meta: Option<serde_json::Value>,
    ) -> Result<()> {
        self.query("UPDATE page SET meta = $meta WHERE url = $url AND `group` = $group")
            .bind(("group", group.to_string()))
            .bind(("url", url.to_string()))
            .bind(("meta", meta))
            .await?;
        Ok(())
    }

    /// List all distinct crawl groups.
    pub async fn list_groups(&self) -> Result<Vec<String>> {
        let raw: Vec<serde_json::Value> = self
            .query("SELECT VALUE `group` FROM page GROUP BY `group`")
            .await?
            .take(0)?;
        let groups: Vec<String> = raw
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        Ok(groups)
    }

    /// Return stats (page count, link count, chunk count) for a group.
    pub async fn group_stats(&self, group: &str) -> Result<crate::db::models::DbStats> {
        let mut resp = self
            .query("SELECT count() FROM page WHERE group = $group GROUP ALL")
            .bind(("group", group.to_string()))
            .await?;
        let raw: Vec<serde_json::Value> = resp.take(0)?;
        let page_count = raw
            .into_iter()
            .next()
            .and_then(|v| v["count"].as_u64())
            .unwrap_or(0);

        let mut resp = self
            .query("SELECT count() FROM discovered WHERE group = $group GROUP ALL")
            .bind(("group", group.to_string()))
            .await?;
        let raw: Vec<serde_json::Value> = resp.take(0)?;
        let link_count = raw
            .into_iter()
            .next()
            .and_then(|v| v["count"].as_u64())
            .unwrap_or(0);

        let mut resp = self
            .query("SELECT count() FROM chunk WHERE group = $group GROUP ALL")
            .bind(("group", group.to_string()))
            .await?;
        let raw: Vec<serde_json::Value> = resp.take(0)?;
        let chunk_count = raw
            .into_iter()
            .next()
            .and_then(|v| v["count"].as_u64())
            .unwrap_or(0);

        Ok(crate::db::models::DbStats {
            page_count,
            link_count,
            chunk_count,
        })
    }
}

#[cfg(feature = "embeddings")]
impl DbClient {
    /// Insert chunks for a page in a single batched query.
    pub async fn insert_chunks(&self, chunks: &[crate::db::models::ChunkRecord]) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }
        let mut parts = Vec::with_capacity(chunks.len());
        for (i, _) in chunks.iter().enumerate() {
            parts.push(format!(
                "{{ group: $group{i}, page: $page{i}, chunk_index: $chunk_index{i}, \
                 chunk_text: $chunk_text{i}, heading_path: $heading_path{i}, \
                 embedding: $embedding{i}, model: $model{i}, \
                 dimensions: $dimensions{i}, created_at: $created_at{i} }}"
            ));
        }
        let sql = format!("INSERT INTO chunk [{}]", parts.join(", "));
        let mut query = self.query(&sql);
        for (i, chunk) in chunks.iter().enumerate() {
            query = query.bind((format!("group{i}"), chunk.group.clone()));
            query = query.bind((format!("page{i}"), chunk.page.clone()));
            query = query.bind((format!("chunk_index{i}"), chunk.chunk_index));
            query = query.bind((format!("chunk_text{i}"), chunk.chunk_text.clone()));
            query = query.bind((format!("heading_path{i}"), chunk.heading_path.clone()));
            query = query.bind((format!("embedding{i}"), chunk.embedding.clone()));
            query = query.bind((format!("model{i}"), chunk.model.clone()));
            query = query.bind((format!("dimensions{i}"), chunk.dimensions));
            query = query.bind((format!("created_at{i}"), chunk.created_at));
        }
        query.await?;
        Ok(())
    }

    /// Delete all chunks for a specific page.
    pub async fn delete_chunks_for_page(&self, page_id: &surrealdb::types::RecordId) -> Result<()> {
        self.query("DELETE chunk WHERE page = $page_id")
            .bind(("page_id", page_id.clone()))
            .await?;
        Ok(())
    }

    /// Vector similarity search over chunks (brute-force cosine; avoids SurrealDB KNN operator bug).
    pub async fn knn_search(
        &self,
        group: Option<&str>,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<crate::db::models::VectorResult>> {
        let raw = self
            .vector_chunk_query(group, query_embedding, limit, None)
            .await?;

        let results: Vec<crate::db::models::VectorResult> = raw
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();

        Ok(results)
    }

    /// Hybrid search: BM25 pre-filter on pages, then KNN on their chunks.
    pub async fn hybrid_search(
        &self,
        group: Option<&str>,
        text_query: &str,
        query_embedding: &[f32],
        bm25_limit: usize,
        knn_limit: usize,
    ) -> Result<Vec<crate::db::models::HybridResult>> {
        let (candidate_urls, bm25_scores) =
            self.bm25_candidates(group, text_query, bm25_limit).await?;

        if candidate_urls.is_empty() {
            return Ok(Vec::new());
        }

        let raw = self
            .vector_chunk_query(group, query_embedding, knn_limit, Some(candidate_urls))
            .await?;

        self.assemble_hybrid_results(group, raw, &bm25_scores).await
    }

    async fn assemble_hybrid_results(
        &self,
        group: Option<&str>,
        raw: Vec<serde_json::Value>,
        bm25_scores: &std::collections::HashMap<String, f64>,
    ) -> Result<Vec<crate::db::models::HybridResult>> {
        let (results, keys) = Self::extract_hybrid_rows(&raw, bm25_scores);
        self.apply_expansions(group, results, &keys).await
    }

    fn extract_hybrid_rows(
        raw: &[serde_json::Value],
        bm25_scores: &std::collections::HashMap<String, f64>,
    ) -> (Vec<crate::db::models::HybridResult>, Vec<(String, i32)>) {
        let mut results = Vec::new();
        let mut keys = Vec::new();
        for v in raw {
            let mut hr: crate::db::models::HybridResult = match serde_json::from_value(v.clone()) {
                Ok(r) => r,
                Err(_) => continue,
            };
            hr.bm25_score = bm25_scores.get(&hr.page_url).copied();
            hr.vector_score = Some(hr.vector_score.unwrap_or(0.0));
            keys.push((hr.page_url.clone(), hr.chunk_index));
            results.push(hr);
        }
        (results, keys)
    }

    async fn apply_expansions(
        &self,
        group: Option<&str>,
        mut results: Vec<crate::db::models::HybridResult>,
        keys: &[(String, i32)],
    ) -> Result<Vec<crate::db::models::HybridResult>> {
        if keys.is_empty() {
            return Ok(results);
        }
        let expanded = self.expand_chunks(group, keys).await?;
        for hr in results.iter_mut() {
            hr.expanded_text = expanded
                .get(&(hr.page_url.clone(), hr.chunk_index))
                .cloned()
                .unwrap_or_else(|| hr.chunk_text.clone());
        }
        Ok(results)
    }

    /// BM25 pre-filter: return candidate page URLs and a URL → score map.
    async fn bm25_candidates(
        &self,
        group: Option<&str>,
        text_query: &str,
        limit: usize,
    ) -> Result<(Vec<String>, std::collections::HashMap<String, f64>)> {
        let raw = self.run_bm25_query(group, text_query, limit).await?;
        Ok(Self::parse_bm25_rows(raw))
    }

    async fn run_bm25_query(
        &self,
        group: Option<&str>,
        text_query: &str,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        let sql = if group.is_some() {
            "SELECT id, url, search::score(0) AS score
             FROM page WHERE group = $group AND content @@ $query
             ORDER BY score DESC LIMIT $limit"
                .to_string()
        } else {
            "SELECT id, url, search::score(0) AS score
             FROM page WHERE content @@ $query
             ORDER BY score DESC LIMIT $limit"
                .to_string()
        };
        let mut q = self.query(&sql);
        if let Some(g) = group {
            q = q.bind(("group", g.to_string()));
        }
        Ok(q.bind(("query", text_query.to_string()))
            .bind(("limit", limit))
            .await?
            .take(0)?)
    }

    fn parse_bm25_rows(
        raw: Vec<serde_json::Value>,
    ) -> (Vec<String>, std::collections::HashMap<String, f64>) {
        let mut urls = Vec::new();
        let mut scores = std::collections::HashMap::new();
        for v in raw {
            if let Some(url) = v["url"].as_str() {
                urls.push(url.to_string());
                if let Some(score) = v["score"].as_f64() {
                    scores.insert(url.to_string(), score);
                }
            }
        }
        (urls, scores)
    }

    /// Fetch adjacent chunks (±1) for each top result to build context window.
    async fn expand_chunks(
        &self,
        group: Option<&str>,
        keys: &[(String, i32)],
    ) -> Result<std::collections::HashMap<(String, i32), String>> {
        let mut result = std::collections::HashMap::new();
        for (url, idx) in keys {
            let raw = self
                .run_expansion_query(group, url, idx.saturating_sub(1), idx + 1)
                .await?;
            if let Some(text) = Self::join_expansion_rows(raw) {
                result.insert((url.clone(), *idx), text);
            }
        }
        Ok(result)
    }

    async fn run_expansion_query(
        &self,
        group: Option<&str>,
        url: &str,
        min_idx: i32,
        max_idx: i32,
    ) -> Result<Vec<serde_json::Value>> {
        let sql = if group.is_some() {
            "SELECT chunk_index, chunk_text FROM chunk
             WHERE page.url = $url AND group = $group
               AND chunk_index >= $min_idx AND chunk_index <= $max_idx
             ORDER BY chunk_index"
                .to_string()
        } else {
            "SELECT chunk_index, chunk_text FROM chunk
             WHERE page.url = $url
               AND chunk_index >= $min_idx AND chunk_index <= $max_idx
             ORDER BY chunk_index"
                .to_string()
        };
        let mut q = self.query(&sql);
        if let Some(g) = group {
            q = q.bind(("group", g.to_string()));
        }
        Ok(q.bind(("url", url.to_string()))
            .bind(("min_idx", min_idx))
            .bind(("max_idx", max_idx))
            .await?
            .take(0)?)
    }

    fn join_expansion_rows(raw: Vec<serde_json::Value>) -> Option<String> {
        let parts: Vec<String> = raw
            .into_iter()
            .filter_map(|v| v["chunk_text"].as_str().map(|s| s.to_string()))
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }

    fn vector_chunk_sql(limit: usize, alias: &str, where_clause: &str) -> String {
        format!(
            "SELECT page.url AS page_url, page.title AS page_title,
             chunk_text, heading_path, chunk_index,
             vector::similarity::cosine(embedding, $embedding) AS {alias}
             FROM chunk {where_clause}
             ORDER BY {alias} DESC LIMIT {limit}"
        )
    }

    /// Build and execute a vector similarity query over chunks.
    /// If `page_urls` is Some, restricts to chunks from those pages.
    async fn vector_chunk_query(
        &self,
        group: Option<&str>,
        embedding: &[f32],
        limit: usize,
        page_urls: Option<Vec<String>>,
    ) -> Result<Vec<serde_json::Value>> {
        let (sql, group_bind, page_urls_bind) =
            Self::build_vector_query(group, embedding, limit, page_urls);
        let mut q = self.query(sql).bind(("embedding", embedding.to_vec()));
        if let Some(g) = group_bind {
            q = q.bind(("group", g));
        }
        if let Some(urls) = page_urls_bind {
            q = q.bind(("urls", urls));
        }
        let mut response = q.await?;
        let raw: Vec<serde_json::Value> = response.take(0)?;
        Ok(raw)
    }

    fn build_vector_query(
        group: Option<&str>,
        _embedding: &[f32],
        limit: usize,
        page_urls: Option<Vec<String>>,
    ) -> (String, Option<String>, Option<Vec<String>>) {
        let alias = if page_urls.is_some() {
            "vector_score"
        } else {
            "score"
        };
        let mut conditions = vec!["true"];
        if group.is_some() {
            conditions.push("group = $group");
        }
        if page_urls.is_some() {
            conditions.push("page.url IN $urls");
        }
        let where_clause = format!("WHERE {}", conditions.join(" AND "));
        let sql = Self::vector_chunk_sql(limit, alias, &where_clause);
        let group_bind = group.map(|g| g.to_string());
        (sql, group_bind, page_urls)
    }

    /// Ensure the HNSW vector index matches the expected dimension.
    pub async fn ensure_embeddings_index(&self, dimension: usize) -> Result<()> {
        super::schema::ensure_hnsw_index(self, dimension).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_insert_and_search() -> Result<()> {
        let db = DbClient::connect_mem().await?;
        db.use_ns_db("test", "test").await?;
        db.bootstrap().await?;

        let page = PageRecord {
            id: None,
            group: "docs".to_string(),
            url: "https://example.com/a".to_string(),
            title: "Hello".to_string(),
            content: "Hello world this is a test page".to_string(),
            depth: 0,
            format: "markdown".to_string(),
            crawled_at: Utc::now(),
            meta: None,
        };
        db.insert_page(&page).await?;

        let results = db.bm25_search(Some("docs"), "hello", 10).await?;
        assert!(!results.is_empty());
        assert_eq!(results[0].url, "https://example.com/a");
        Ok(())
    }

    #[tokio::test]
    async fn test_full_pipeline() -> Result<()> {
        let db = DbClient::connect_mem().await?;
        db.use_ns_db("test", "test").await?;
        db.bootstrap().await?;

        // Insert two pages in the same group
        let page1 = PageRecord {
            id: None,
            group: "blog".to_string(),
            url: "https://example.com/post1".to_string(),
            title: "First Post".to_string(),
            content: "SurrealDB is a multi-model database with full-text search".to_string(),
            depth: 0,
            format: "markdown".to_string(),
            crawled_at: Utc::now(),
            meta: None,
        };
        let page2 = PageRecord {
            id: None,
            group: "blog".to_string(),
            url: "https://example.com/post2".to_string(),
            title: "Second Post".to_string(),
            content: "Graph traversal is powerful in SurrealDB".to_string(),
            depth: 1,
            format: "markdown".to_string(),
            crawled_at: Utc::now(),
            meta: None,
        };
        let id1 = db.insert_page(&page1).await?;
        let id2 = db.insert_page(&page2).await?;

        // Insert a discovered link between them
        let link = LinkRecord {
            group: "blog".to_string(),
            from_page: id1,
            to_page: id2,
            anchor_text: Some("read more".to_string()),
            discovered_at: Utc::now(),
        };
        db.insert_discovered(&[link]).await?;

        // Cross-group page
        let page3 = PageRecord {
            id: None,
            group: "docs".to_string(),
            url: "https://docs.example.com/intro".to_string(),
            title: "Introduction".to_string(),
            content: "Welcome to the documentation".to_string(),
            depth: 0,
            format: "html".to_string(),
            crawled_at: Utc::now(),
            meta: None,
        };
        db.insert_page(&page3).await?;

        // BM25 search scoped to group
        let blog_results = db.bm25_search(Some("blog"), "SurrealDB", 10).await?;
        assert_eq!(blog_results.len(), 2, "should find both blog posts");

        // BM25 search across all groups
        let all_results = db.bm25_search(None, "SurrealDB", 10).await?;
        assert_eq!(all_results.len(), 2, "cross-group search finds blog posts");

        // Group stats
        let stats = db.group_stats("blog").await?;
        assert_eq!(stats.page_count, 2);
        assert_eq!(stats.link_count, 1);

        // List groups
        let groups = db.list_groups().await?;
        assert!(groups.contains(&"blog".to_string()));
        assert!(groups.contains(&"docs".to_string()));

        // Reset group and verify
        db.reset_group("blog").await?;
        let stats_after = db.group_stats("blog").await?;
        assert_eq!(stats_after.page_count, 0);
        assert_eq!(stats_after.link_count, 0);

        // Docs group should still exist
        let groups_after = db.list_groups().await?;
        assert!(!groups_after.contains(&"blog".to_string()));
        assert!(groups_after.contains(&"docs".to_string()));

        Ok(())
    }
}
