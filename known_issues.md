# Known Issues & Research TODO

## Missing Crawl Pages — SurrealDB Docs

**Status:** Open — needs investigation  
**Discovered:** 2026-05-27

### Symptom
BFS crawl of `https://surrealdb.com/docs` did not discover all pages. Individual `extract` commands on specific docs URLs work correctly, but the crawler missed many sub-pages.

### Pages Confirmed Working (extract)
- `https://surrealdb.com/docs/learn/data-models/full-text-search/overview`
- `https://surrealdb.com/docs/learn/data-models/full-text-search/analyzers-and-tokenizers`

### Likely Causes to Investigate
1. **Client-side navigation (Astro SPA)** — Docs may use Astro with partial hydration; links might be injected by JS after initial HTML load.
2. **Link format / normalization** — Relative paths like `/docs/learn/...` vs absolute, query params, fragments.
3. **Same-origin edge cases** — Subdomain variations (`www.` vs bare, `docs.`).
4. **robots.txt restrictions** — Crawler user-agent may be blocked from certain paths.
5. **Link extraction selector** — `scraper` CSS selector may miss links inside Astro-rendered components or specific DOM structures.
6. **Servo stub backend** — If the real engine failed to initialize, stub HTML may not contain the actual page links.

### Next Steps
- [ ] Inspect crawl output directory (`SurrealDocs/`) to see what *was* found
- [ ] Check `robots.txt` at `https://surrealdb.com/robots.txt`
- [ ] Review link extraction in `crates/vibeeye-app/src/crawl/links.rs`
- [ ] Test with `--max-depth` higher than default
- [ ] Try crawling a simpler static site to verify BFS logic is sound
- [ ] Consider if docs site requires JS execution for link discovery

### Related Files
- `crates/vibeeye-app/src/crawl/links.rs` — Link extraction logic
- `crates/vibeeye-app/src/crawl/robots.rs` — robots.txt parser
- `crates/vibeeye-app/src/crawl/mod.rs` — BFS orchestration


### other sites with similar issues

Github and 

crates didn't get to the main content area
 ./target/release/vibe-eye extract -f markdown https://crates.io/crates/surrealdb-migrations  2>&1
{
  "url": "https://crates.io/crates/surrealdb-migrations",
  "content": "---\nmeta-apple-mobile-web-app-capable: yes\nmeta-apple-mobile-web-app-status-bar-style: default\nmeta-apple-mobile-web-app-title: crates.io: Rust Package Registry\nmeta-description: An awesome SurrealDB migration tool, with a user-friendly CLI and a versatile Rust library that enables seamless integration into any project.\nmeta-og:description: crates.io serves as a central registry for sharing crates, which are packages or libraries written in Rust that you can use to enhance your projects\nmeta-og:image: https://static.crates.io/og-images/surrealdb-migrations.png\nmeta-og:title: crates.io: Rust Package Registry\nmeta-theme-color: var(--header-bg-color)\nmeta-twitter:card: summary_large_image\nmeta-viewport: width=device-width, initial-scale=1\ntitle: surrealdb-migrations - crates.io: Rust Package Registry\n---\n\n\n\n\n\n\n  \n\n\n\n# surrealdb-migrations v2.4.0\n\n\n\nAn awesome SurrealDB migration tool, with a user-friendly CLI and a versatile Rust library that enables seamless integration into any project.\n\n\n\n- [#cli](/keywords/cli)\n- [#migrations](/keywords/migrations)\n- [#surrealdb](/keywords/surrealdb)\n\n\n\n\n\n\n\n\n\n\n\n## Metadata\n\n  Release date: 6 months ago\n\n 2024 edition\n\n\n\n License: [MIT](https://choosealicense.com/licenses/mit)\n\n\n\n 5.9K SLoC\n\n\n\n Size: 163 KiB\n\n\n\n Package URL: pkg:cargo/surrealdb-migrations@2.4.0  \n\n [https://github.com/package-url/purl-spec](https://github.com/package-url/purl-spec)\n\n\n\n## Install\n\n\n\n`cargo install surrealdb-migrations`\n\nRunning the above command will globally install the surrealdb-migrations binary.\n\n\n\n### Install as library\n\n\n\nRun the following Cargo command in your project directory:\n\n `cargo add surrealdb-migrations`\n\nOr add the following line to your Cargo.toml:\n\n `surrealdb-migrations = \"2.4.0\"`\n\n\n\n\n\n## Documentation\n\n\n\n [docs.rs/surrealdb-migrations/2.4.0](https://docs.rs/surrealdb-migrations/2.4.0)\n\n\n\n## Browse source\n\n\n\n [docs.rs/crate/surrealdb-migrations/2.4.0/source](https://docs.rs/crate/surrealdb-migrations/2.4.0/source/)\n\n\n\n## Repository\n\n\n\n [github.com/Odonno/surrealdb-migrations](https://github.com/Odonno/surrealdb-migrations/)\n\n\n\n## Owners\n\n\n\n- [![David Bottiau (Odonno)](https://avatars.githubusercontent.com/u/6053067?v=4&amp;s=64 \"David Bottiau\") David Bottiau](/users/Odonno)\n\n\n\n## Categories\n\n\n\n- [Command line utilities](/categories/command-line-utilities)\n\n\n\n [Report crate](/support?crate=surrealdb-migrations&inquire=crate-violation)\n\n\n\n### Stats Overview\n\n\n\n 65,164 Downloads all time\n\n\n\n 44 Versions published\n\n\n\n#### Downloads over the last 90 days\n\n\n\nDisplay as\n\nStacked\n\n\n\nLoading…\n",
  "format": "markdown",
  "title": "surrealdb-migrations - crates.io: Rust Package Registry"
}

this page did not match after extract and different results for markdown and text

:~/Projects/VibeEye ‹main›
$ ./target/release/vibe-eye extract https://surrealdb.com/blog/enhancing-retrieval-augmented-generation-with-surrealdb
{
  "url": "https://surrealdb.com/blog/enhancing-retrieval-augmented-generation-with-surrealdb",
  "content": "---\ncanonical: https://surrealdb.com/blog/enhancing-retrieval-augmented-generation-with-surrealdb\nmeta-article:author: Tobie Morgan Hitchcock\nmeta-article:modified_time: 2025-01-31T00:00:00.000Z\nmeta-article:published_time: 2025-01-31T00:00:00.000Z\nmeta-article:section: tutorials\nmeta-description: GraphRAG: Enhancing Retrieval-Augmented Generation with SurrealDB, Gemini and DeepSeek\nmeta-og:description: GraphRAG: Enhancing Retrieval-Augmented Generation with SurrealDB, Gemini and DeepSeek\nmeta-og:image: https://cdn.surrealdb.com/cueao4i8p6is73fqbiig.auto\nmeta-og:site_name: SurrealDB\nmeta-og:title: Enhancing retrieval-augmented generation with SurrealDB | Blog | SurrealDB\nmeta-og:type: article\nmeta-og:url: https://surrealdb.com/blog/enhancing-retrieval-augmented-generation-with-surrealdb\nmeta-referrer: origin-when-cross-origin\nmeta-robots: index, follow\nmeta-twitter:card: summary_large_image\nmeta-twitter:description: GraphRAG: Enhancing Retrieval-Augmented Generation with SurrealDB, Gemini and DeepSeek\nmeta-twitter:domain: surrealdb.com\nmeta-twitter:image: https://cdn.surrealdb.com/cueao4i8p6is73fqbiig.auto\nmeta-twitter:site: @surrealdb\nmeta-twitter:title: Enhancing retrieval-augmented generation with SurrealDB | Blog | SurrealDB\nmeta-twitter:url: https://surrealdb.com/blog/enhancing-retrieval-augmented-generation-with-surrealdb\nmeta-viewport: width=device-width,initial-scale=1\ntitle: Enhancing retrieval-augmented generation with SurrealDB | Blog | SurrealDB\n---\n",
  "format": "markdown",
  "title": "Enhancing retrieval-augmented generation with SurrealDB | Blog | SurrealDB"
}
2026-05-29 10:43-42
:~/Projects/VibeEye ‹main›
$ ./target/release/vibe-eye extract https://surrealdb.com/blog/enhancing-retrieval-augmented-generation-with-surrealdb -f text
{