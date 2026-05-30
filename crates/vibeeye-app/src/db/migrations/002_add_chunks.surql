-- Metadata table for schema versioning and config tracking
DEFINE TABLE db_metadata TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD key ON db_metadata TYPE string ASSERT $value != NONE PERMISSIONS FULL;
DEFINE FIELD value ON db_metadata TYPE string PERMISSIONS FULL;
DEFINE FIELD updated_at ON db_metadata TYPE datetime DEFAULT time::now() PERMISSIONS FULL;

DEFINE INDEX idx_metadata_key ON db_metadata FIELDS key UNIQUE;

-- Chunk table (single table, dimension tracked in field)
DEFINE TABLE chunk TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD group ON chunk TYPE string ASSERT $value != NONE PERMISSIONS FULL;
DEFINE FIELD page ON chunk TYPE record<page> ASSERT $value != NONE PERMISSIONS FULL;
DEFINE FIELD chunk_index ON chunk TYPE int PERMISSIONS FULL;
DEFINE FIELD chunk_text ON chunk TYPE string PERMISSIONS FULL;
DEFINE FIELD heading_path ON chunk TYPE array<string> PERMISSIONS FULL;
DEFINE FIELD embedding ON chunk TYPE array<float> PERMISSIONS FULL;
DEFINE FIELD model ON chunk TYPE string PERMISSIONS FULL;
DEFINE FIELD dimensions ON chunk TYPE int PERMISSIONS FULL;
DEFINE FIELD created_at ON chunk TYPE datetime DEFAULT time::now() PERMISSIONS FULL;

-- Index for group-scoped chunk queries
DEFINE INDEX idx_chunk_group ON chunk FIELDS group;

-- Index for expansion queries (adjacent chunks by page)
DEFINE INDEX idx_chunk_page_index ON chunk FIELDS page, chunk_index;
