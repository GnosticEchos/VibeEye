-- Migration 004: Ensure doc_analyzer is repaired on existing databases
-- This is idempotent due to OVERWRITE
DEFINE ANALYZER OVERWRITE doc_analyzer
    TOKENIZERS BLANK,CLASS,CAMEL
    FILTERS LOWERCASE,SNOWBALL(ENGLISH);
