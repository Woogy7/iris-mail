-- Add a provider-specific remote message identifier.
-- For M365 Graph accounts this stores the opaque Graph message ID string.
-- For IMAP accounts this is NULL (the imap_uid column is used instead).
ALTER TABLE messages ADD COLUMN remote_id TEXT;

CREATE INDEX IF NOT EXISTS idx_messages_remote_id
    ON messages (account_id, remote_id);
