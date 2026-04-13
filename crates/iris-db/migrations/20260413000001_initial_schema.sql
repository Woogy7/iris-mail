-- Initial schema for Iris Mail.
--
-- All identifiers are UUIDs stored as TEXT. Timestamps are RFC 3339 TEXT.
-- Complex fields (sync_preferences, address lists) are JSON TEXT.

CREATE TABLE IF NOT EXISTS accounts (
    id             TEXT    PRIMARY KEY,
    provider       TEXT    NOT NULL CHECK (provider IN ('m365', 'gmail', 'imap_generic')),
    display_name   TEXT    NOT NULL,
    email_address  TEXT    NOT NULL,
    keychain_ref   TEXT    NOT NULL,
    accent_colour  TEXT    NOT NULL DEFAULT 'mauve',
    sync_preferences TEXT  NOT NULL DEFAULT '{}',
    is_enabled     INTEGER NOT NULL DEFAULT 1,
    created_at     TEXT    NOT NULL,
    updated_at     TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS folders (
    id             TEXT    PRIMARY KEY,
    account_id     TEXT    NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name           TEXT    NOT NULL,
    full_path      TEXT    NOT NULL DEFAULT '',
    parent_id      TEXT             REFERENCES folders(id) ON DELETE SET NULL,
    special        TEXT    NOT NULL DEFAULT 'other'
                          CHECK (special IN ('inbox', 'sent', 'drafts', 'trash', 'archive', 'other')),
    uid_validity   INTEGER,
    last_seen_uid  INTEGER,
    last_synced_at TEXT,
    message_count  INTEGER NOT NULL DEFAULT 0,
    unread_count   INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT    NOT NULL,
    updated_at     TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
    id              TEXT    PRIMARY KEY,
    folder_id       TEXT    NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    account_id      TEXT    NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    subject         TEXT    NOT NULL DEFAULT '',
    from_name       TEXT,
    from_address    TEXT,
    to_addresses    TEXT    NOT NULL DEFAULT '[]',
    cc_addresses    TEXT    NOT NULL DEFAULT '[]',
    bcc_addresses   TEXT    NOT NULL DEFAULT '[]',
    date            TEXT,
    size_bytes      INTEGER,
    is_read         INTEGER NOT NULL DEFAULT 0,
    is_flagged      INTEGER NOT NULL DEFAULT 0,
    is_answered     INTEGER NOT NULL DEFAULT 0,
    thread_id       TEXT,
    message_id_header TEXT,
    imap_uid        INTEGER,
    stored_local    INTEGER NOT NULL DEFAULT 0,
    stored_remote   INTEGER NOT NULL DEFAULT 1,
    has_attachment  INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_folder_date
    ON messages (account_id, folder_id, date DESC);

CREATE INDEX IF NOT EXISTS idx_messages_thread
    ON messages (account_id, thread_id);

CREATE TABLE IF NOT EXISTS message_bodies (
    message_id     TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
    html           TEXT,
    sanitised_html TEXT,
    plain_text     TEXT
);

CREATE TABLE IF NOT EXISTS attachments (
    id         TEXT    PRIMARY KEY,
    sha256     TEXT    NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL,
    mime_type  TEXT    NOT NULL DEFAULT 'application/octet-stream'
);

CREATE TABLE IF NOT EXISTS message_attachments (
    message_id    TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    attachment_id TEXT NOT NULL REFERENCES attachments(id) ON DELETE CASCADE,
    filename      TEXT,
    mime_type     TEXT NOT NULL,
    PRIMARY KEY (message_id, attachment_id)
);

-- Full-text search index for messages.
-- Standalone (no content= param) so we manage it entirely via triggers.
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    subject,
    body,
    from_name,
    from_address,
    to_addresses,
    attachment_filenames
);

-- Keep FTS in sync via triggers. FTS5 does not support UPDATE, so we
-- always DELETE + INSERT.

-- When a new message is inserted, add a skeleton FTS row.
CREATE TRIGGER IF NOT EXISTS trg_messages_ai AFTER INSERT ON messages
BEGIN
    INSERT INTO messages_fts (rowid, subject, body, from_name, from_address, to_addresses, attachment_filenames)
    VALUES (NEW.rowid, COALESCE(NEW.subject, ''), '', COALESCE(NEW.from_name, ''), COALESCE(NEW.from_address, ''), COALESCE(NEW.to_addresses, ''), '');
END;

-- When a message is deleted, remove its FTS row.
CREATE TRIGGER IF NOT EXISTS trg_messages_ad AFTER DELETE ON messages
BEGIN
    DELETE FROM messages_fts WHERE rowid = OLD.rowid;
END;

-- When message header fields change, refresh the FTS row.
CREATE TRIGGER IF NOT EXISTS trg_messages_au AFTER UPDATE OF subject, from_name, from_address, to_addresses ON messages
BEGIN
    DELETE FROM messages_fts WHERE rowid = OLD.rowid;
    INSERT INTO messages_fts (rowid, subject, body, from_name, from_address, to_addresses, attachment_filenames)
    VALUES (
        NEW.rowid,
        COALESCE(NEW.subject, ''),
        COALESCE((SELECT plain_text FROM message_bodies WHERE message_id = NEW.id), ''),
        COALESCE(NEW.from_name, ''),
        COALESCE(NEW.from_address, ''),
        COALESCE(NEW.to_addresses, ''),
        COALESCE((SELECT GROUP_CONCAT(filename, ' ') FROM message_attachments WHERE message_id = NEW.id), '')
    );
END;

-- When a message body is inserted, re-index with the body text.
CREATE TRIGGER IF NOT EXISTS trg_message_bodies_ai AFTER INSERT ON message_bodies
BEGIN
    DELETE FROM messages_fts WHERE rowid = (SELECT rowid FROM messages WHERE id = NEW.message_id);
    INSERT INTO messages_fts (rowid, subject, body, from_name, from_address, to_addresses, attachment_filenames)
    VALUES (
        (SELECT rowid FROM messages WHERE id = NEW.message_id),
        COALESCE((SELECT subject FROM messages WHERE id = NEW.message_id), ''),
        COALESCE(NEW.plain_text, ''),
        COALESCE((SELECT from_name FROM messages WHERE id = NEW.message_id), ''),
        COALESCE((SELECT from_address FROM messages WHERE id = NEW.message_id), ''),
        COALESCE((SELECT to_addresses FROM messages WHERE id = NEW.message_id), ''),
        COALESCE((SELECT GROUP_CONCAT(filename, ' ') FROM message_attachments WHERE message_id = NEW.message_id), '')
    );
END;

-- When a message body's plain_text is updated, re-index.
CREATE TRIGGER IF NOT EXISTS trg_message_bodies_au AFTER UPDATE OF plain_text ON message_bodies
BEGIN
    DELETE FROM messages_fts WHERE rowid = (SELECT rowid FROM messages WHERE id = NEW.message_id);
    INSERT INTO messages_fts (rowid, subject, body, from_name, from_address, to_addresses, attachment_filenames)
    VALUES (
        (SELECT rowid FROM messages WHERE id = NEW.message_id),
        COALESCE((SELECT subject FROM messages WHERE id = NEW.message_id), ''),
        COALESCE(NEW.plain_text, ''),
        COALESCE((SELECT from_name FROM messages WHERE id = NEW.message_id), ''),
        COALESCE((SELECT from_address FROM messages WHERE id = NEW.message_id), ''),
        COALESCE((SELECT to_addresses FROM messages WHERE id = NEW.message_id), ''),
        COALESCE((SELECT GROUP_CONCAT(filename, ' ') FROM message_attachments WHERE message_id = NEW.message_id), '')
    );
END;

-- Sync job tracking.
CREATE TABLE IF NOT EXISTS sync_jobs (
    id            TEXT    PRIMARY KEY,
    account_id    TEXT    NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    job_type      TEXT    NOT NULL
                         CHECK (job_type IN ('initial_sync', 'backfill', 'idle', 'tiering', 'attachment_download')),
    status        TEXT    NOT NULL DEFAULT 'pending'
                         CHECK (status IN ('pending', 'running', 'completed', 'failed')),
    priority      INTEGER NOT NULL DEFAULT 0,
    payload       TEXT,
    error_message TEXT,
    created_at    TEXT    NOT NULL,
    started_at    TEXT,
    completed_at  TEXT
);
