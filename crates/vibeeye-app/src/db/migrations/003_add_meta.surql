-- Add meta field to page table for structured metadata (JSON-LD, Open Graph, IndexedDB).
-- FLEXIBLE is required in v3 so that nested object fields are not rejected by SCHEMAFULL.
DEFINE FIELD meta ON page TYPE option<object> FLEXIBLE PERMISSIONS FULL;
