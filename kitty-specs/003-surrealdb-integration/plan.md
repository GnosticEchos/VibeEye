# Implementation Plan: SurrealDB Integration — Phase 1
**Mission**: 003-surrealdb-integration  
**Date**: 2026-05-27  
**Spec**: [spec.md](./spec.md)  
**Brainstorm**: [brainstorm.md](./brainstorm.md)

---

## Summary

Add an optional SurrealDB backend to VibeEye for persistent crawl storage, BM25 full-text search, and graph link topology. All new code is feature-gated behind `surrealdb`. Phase 1 contains no embeddings, no chunking, and no migration system.

---

## Technical Context

**Language**: Rust 1.86+ (edition 2024)  
**Primary Dependency**: `surrealdb` 3.1 with `kv-surrealkv` feature  
**Storage**: SurrealKV embedded (`~/.local/share/vibeeye/db.surrealkv`)  
**Feature Flag**: `surrealdb` (default off)  
**Testing**: `kv-mem` backend for unit/integration tests

---

## Work Packages

### WP01: Workspace & Dependency Setup
**Owner**: vibeeye-app, vibeeye-cli, workspace Cargo.toml  
**Blocked by**: None

1. Add `surrealdb = { version = "3.1", default-features = false, features = ["kv-surrealkv"] }` to workspace `Cargo.toml` `[workspace.dependencies]`
2. Add optional `surrealdb` dep to `vibeeye-app/Cargo.toml` with `surrealdb = ["dep:surrealdb"]` feature
3. Add `surrealdb = ["vibeeye-app/surrealdb"]` feature to `vibeeye-cli/Cargo.toml`
4. Verify `cargo check --workspace --features surrealdb` compiles cleanly

**Acceptance**:
- `cargo check --workspace` passes (no feature)
- `cargo check --workspace --features surrealdb` passes
- No new warnings

---

### WP02: Core DB Module (vibeeye-app)
**Owner**: `crates/vibeeye-app/src/db/`  
**Blocked by**: WP01

1. Create `crates/vibeeye-app/src/db/mod.rs` (gated with `#[cfg(feature = "surrealdb")]`)
2. Create `crates/vibeeye-app/src/db/schema.rs`:
   - `SCHEMA_SQL` constant containing all `DEFINE` statements from spec §5
   - `pub async fn bootstrap(db: &Surreal<Any>) -> Result<()>` — runs schema SQL idempotently
3. Create `crates/vibeeye-app/src/db/models.rs`:
   - `PageRecord` struct (Serialize + Deserialize)
   - `QueryResult` struct
   - `LinkRecord` struct (for graph edges)
4. Create `crates/vibeeye-app/src/db/client.rs`:
   - `pub struct DbClient { inner: Surreal<Any> }`
   - `pub async fn connect(path: &Path) -> Result<DbClient>` — uses `Surreal::new::<SurrealKV>(path)`
   - `pub async fn connect_mem() -> Result<DbClient>` — test helper using `Mem`
   - `pub async fn use_ns_db(&self, ns: &str, db: &str) -> Result<()>`
   - `pub async fn reset_group(&self, group: &str) -> Result<()>` — `DELETE page WHERE group = $group`, `DELETE discovered WHERE group = $group`

**Acceptance**:
- `cargo test --features surrealdb` in vibeeye-app passes (even if no tests yet)
- Schema bootstrap runs without error on `kv-mem`

---

### WP03: DB Operations API
**Owner**: `crates/vibeeye-app/src/db/mod.rs`  
**Blocked by**: WP02

1. Implement `store_page(db, group: &str, page: PageRecord) -> Result<()>`:
   - Include `group` field in record; composite unique `(url, group)` prevents duplicates within group
   - Use `db.upsert(("page", (group, url_hash))).content(page)` or parameterized `INSERT ... ON DUPLICATE KEY UPDATE`
2. Implement `store_links(db, group: &str, from_url: &str, links: Vec<LinkDiscovered>) -> Result<()>`:
   - For each link, ensure target page record exists (upsert minimal placeholder with same group)
   - Create `RELATE page:target_id -> discovered -> page:source_id CONTENT { group, anchor_text, discovered_at }`
3. Implement `query(db, group: Option<&str>, q: &str, limit: usize) -> Result<Vec<QueryResult>>`:
   - If `group` is `Some`: `SELECT ... FROM page WHERE group = $group AND content @@ $q ORDER BY search::score(1) DESC LIMIT $limit`
   - If `group` is `None` (default): `SELECT ... FROM page WHERE content @@ $q ORDER BY search::score(1) DESC LIMIT $limit` (cross-group)
   - Extract snippets from content (first 200 chars)
4. Implement `stats(db, group: Option<&str>) -> Result<DbStats>`:
   - Count pages and links, optionally filtered by group
5. Implement `list_groups(db) -> Result<Vec<String>>`:
   - `SELECT DISTINCT group FROM page ORDER BY group`

**Acceptance**:
- Integration test: store_page → query returns the page
- Integration test: duplicate URL upsert updates content, not duplicates
- Integration test: store_links creates graph edges, queryable via `->discovered->`

---

### WP04: Config Integration
**Owner**: `crates/vibeeye-app/src/config/crawl.rs`  
**Blocked by**: WP02

1. Add `SurrealDbConfig` struct:
   ```rust
   #[derive(Debug, Deserialize, Serialize, Clone)]
   pub struct SurrealDbConfig {
       pub endpoint: String,    // default: "surrealkv://~/.local/share/vibeeye/db"
       pub namespace: String,   // default: "vibeeye"
       pub database: String,    // default: "crawl" (fixed database for all groups)
   }
   ```
2. Add `surrealdb: Option<SurrealDbConfig>` to `CrawlProfile`
3. Update `CrawlConfig::default()` with sensible SurrealDB defaults
4. Expand `~` in endpoint path using `shellexpand` or manual tilde expansion
5. Add group name derivation utility in `vibeeye-app/src/db/util.rs`:
   - `derive_group(url, override) -> String` — extracts domain, sanitizes to valid identifier
   - `sanitize_identifier(name: &str) -> String` — lowercase, replace non-alphanumeric with `_`, truncate to 64

**Acceptance**:
- Config round-trip: parse TOML with `[surrealdb]` section → serialize → equal
- `derive_group("https://surrealdb.com/docs", None)` → `"surrealdb_com"`
- `derive_group("https://surrealdb.com/docs", Some("docs-v2"))` → `"docs_v2"

---

### WP05: Output Router
**Owner**: `crates/vibeeye-core/src/output/`  
**Blocked by**: WP04

1. Create `crates/vibeeye-core/src/output/mod.rs`:
   ```rust
   pub trait CrawlOutput: Send + Sync {
       async fn emit(&self, result: CrawlResult) -> Result<()>;
       async fn close(&self) -> Result<()>;
   }
   ```
2. Move existing file output logic into `FileOutput` struct implementing `CrawlOutput`
3. Create `SurrealOutput` (gated behind `#[cfg(feature = "surrealdb")]`) implementing `CrawlOutput`, takes group name and stores it with every record
4. Factory function `create_output(config: &OutputConfig) -> Box<dyn CrawlOutput>`

**Acceptance**:
- File output test still passes
- SurrealOutput compiles when feature enabled

---

### WP06: Crawl Integration
**Owner**: `crates/vibeeye-app/src/crawl/mod.rs`  
**Blocked by**: WP03, WP05

1. Modify `crawl::run` to accept `Box<dyn CrawlOutput>` instead of hardcoded file writer
2. Derive group name from start URL domain (or `--group` override) before crawl begins
3. Pass group name to `SurrealOutput::new(group)` so it tags every record
4. For each completed page, call `output.emit(crawl_result).await?`
5. For `SurrealOutput`, also emit discovered links as graph edges (tagged with same group)
6. Ensure `CrawlResult` struct contains all fields needed for `PageRecord` (URL, title, content, depth, format)

**Acceptance**:
- `cargo test --features surrealdb` passes with crawl integration test
- File output produces same results as before
- Surreal output stores pages and links with correct group field
- Two crawls with different domains coexist in same database, queryable independently or together

---

### WP07: CLI Commands
**Owner**: `crates/vibeeye-cli/`  
**Blocked by**: WP04, WP06

1. Update `crates/vibeeye-cli/src/cli.rs`:
   - Add `#[clap(subcommand)] Query(QueryCmd)` to `Commands` enum
   - Add `QueryCmd` struct with `query: String`, `#[clap(short, long)] group: Option<String>`, `#[clap(short, long, default_value = "10")] limit: usize`, `#[clap(short, long, default_value = "table")] format: OutputFormat`
   - Add `DbCmd` subcommand with `List`, `Reset`, `Status` variants
   - Add `#[clap(short, long)] group: Option<String>` to crawl args (when `-o embed`)
   - Allow `-o embed` as valid output value in crawl args
2. Update `crates/vibeeye-cli/src/commands.rs`:
   - `crawl_command`: derive group from URL or `--group`, pass to `SurrealOutput::new(group)`
   - `query_command`: load config, connect DB, call `db::query(group.as_deref(), ...)`, format results (group = None searches all)
   - `db_list_command`: call `db::list_groups()`, print table
   - `db_reset_command`: require `--group`, prompt for confirmation, call `db::reset_group(group)`
   - `db_status_command`: call `db::stats(group.as_deref())`, print summary (group = None shows totals)
3. Result formatting:
   - Table format: `comfy-table` or simple `println!` aligned columns
   - JSON format: `serde_json::to_string_pretty()`

**Acceptance**:
- `vibe-eye crawl https://surrealdb.com/docs -o embed` uses group `surrealdb_com`
- `vibe-eye crawl https://surrealdb.com/docs -o embed -g docs-v2` uses group `docs_v2`
- `vibe-eye query "test"` searches all groups (cross-domain)
- `vibe-eye query "test" --group surrealdb_com` searches only that group
- `vibe-eye db list` shows all groups
- `vibe-eye db reset --group surrealdb_com` prompts for confirmation
- `vibe-eye db status` shows totals across all groups
- `vibe-eye db status --group surrealdb_com` shows counts for one group
- `cargo clippy --all-targets --all-features -- -D warnings` clean

---

### WP08: Integration & E2E Tests
**Owner**: `crates/vibeeye-app/tests/`  
**Blocked by**: WP03, WP06

1. `db_integration_test.rs`:
   - Spin up `kv-mem` DB, bootstrap schema, store pages, query, assert results
   - Test duplicate URL upsert
   - Test graph edge creation and traversal
   - Test group isolation: store pages in `group_a` and `group_b`, query each independently, then query all
   - Test cross-group query returns results from both groups
2. `group_selection_test.rs`:
   - Test `derive_group()` with various URLs and overrides
   - Test `sanitize_identifier()` edge cases (empty, special chars, too long)
3. `crawl_surreal_e2e.rs`:
   - Crawl stub HTML into group `test_com`, verify DB state
   - Crawl second domain into `example_com`, verify both groups coexist
   - Verify cross-group query returns results from both
   - Verify group-scoped query returns only matching group
4. Update existing tests to use `FileOutput` via the new trait

**Acceptance**:
- All tests pass: `cargo test --workspace --features surrealdb`
- No test regressions without feature: `cargo test --workspace`

---

### WP09: Documentation & Final Review
**Owner**: All crates  
**Blocked by**: WP07, WP08

1. Update `README.md` with:
   - `--features surrealdb` build instructions
   - New CLI commands (`query`, `db`)
   - Config example with `[surrealdb]` section
2. Verify CRAP scores: `cargo crap --workspace` — all new functions ≤ 30
3. Final `cargo clippy` and `cargo test` pass

**Acceptance**:
- README covers new features
- CRAP scores acceptable
- No clippy warnings
- All tests green

---

## Dependency Graph

```
WP01 (Deps)
  │
  ├── WP02 (DB Core)
  │     │
  │     ├── WP03 (DB Ops) ───┐
  │     │                     │
  │     └── WP04 (Config) ────┤
  │                           │
  ├── WP05 (Output Router) ◄──┤
  │                           │
  └── WP06 (Crawl Integration)
        │
        ├── WP07 (CLI)
        │
        └── WP08 (Tests)
              │
              └── WP09 (Docs + Review)
```

---

## Risk Register

| Risk | Mitigation |
|------|------------|
| SurrealDB 3.1 MSRV (1.82) vs project (1.86) | ✅ Already compatible |
| Binary bloat from SurrealDB engine | Use `default-features = false` + `kv-surrealkv` only |
| Schema bootstrap non-idempotency | Test on `kv-mem` multiple times in same test |
| Upsert semantics changing in 3.x | Pin to `3.1.x`, verify with integration test |
| Feature flag accidentally leaking | Code review + `#[cfg(feature = "surrealdb")]` on all new modules |
