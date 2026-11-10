-- Idempotent relations: prevent duplicate discovered edges on retry
-- This allows RELATE to be safely retried without creating duplicate edges
DEFINE INDEX idx_discovered_unique ON discovered FIELDS in, out UNIQUE;
