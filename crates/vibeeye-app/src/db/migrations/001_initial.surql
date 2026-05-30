-- Table: page (crawled documents)
DEFINE TABLE page TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD group ON page TYPE string ASSERT $value != NONE PERMISSIONS FULL;
DEFINE FIELD url ON page TYPE string ASSERT $value != NONE PERMISSIONS FULL;
DEFINE FIELD title ON page TYPE string PERMISSIONS FULL;
DEFINE FIELD content ON page TYPE string PERMISSIONS FULL;
DEFINE FIELD depth ON page TYPE int PERMISSIONS FULL;
DEFINE FIELD format ON page TYPE string PERMISSIONS FULL;
DEFINE FIELD crawled_at ON page TYPE datetime DEFAULT time::now() PERMISSIONS FULL;

-- Composite unique: same URL can exist in different groups
DEFINE INDEX idx_url_group ON page FIELDS url, group UNIQUE;

-- Fast group filtering and deletion
DEFINE INDEX idx_page_group ON page FIELDS group;

-- Analyzer for documentation text
DEFINE ANALYZER doc_analyzer
    TOKENIZERS BLANK,CLASS,CAMEL
    FILTERS LOWERCASE,SNOWBALL(ENGLISH);

-- BM25 full-text index on page content
DEFINE INDEX idx_content ON page
    FIELDS content
    FULLTEXT ANALYZER doc_analyzer BM25(1.2,0.75);

-- Table: discovered (graph edges for link topology)
DEFINE TABLE discovered TYPE RELATION IN page OUT page SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD group ON discovered TYPE string ASSERT $value != NONE PERMISSIONS FULL;
DEFINE FIELD anchor_text ON discovered TYPE none | string PERMISSIONS FULL;
DEFINE FIELD discovered_at ON discovered TYPE datetime DEFAULT time::now() PERMISSIONS FULL;
