-- Add server configuration (IMAP/SMTP settings) to accounts table.
-- Stored as JSON matching the iris_core::ServerConfig type.
ALTER TABLE accounts ADD COLUMN server_config TEXT NOT NULL DEFAULT '{}';

-- Add unique index on (account_id, full_path) for stable folder identity.
-- This allows folder upsert to match by path instead of primary key,
-- preventing cascade-deletion of messages during sync.
CREATE UNIQUE INDEX IF NOT EXISTS idx_folders_account_full_path
    ON folders (account_id, full_path);
