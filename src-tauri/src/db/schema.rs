pub const CREATE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS downloads (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    filename TEXT NOT NULL,
    save_dir TEXT NOT NULL,
    file_path TEXT NOT NULL,
    total_bytes INTEGER,
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    category TEXT NOT NULL,
    status TEXT NOT NULL,
    connections INTEGER NOT NULL,
    supports_range INTEGER NOT NULL DEFAULT 1,
    is_hls INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    error_message TEXT,
    referer TEXT,
    speed_limit_bps INTEGER,
    quality TEXT
);

CREATE TABLE IF NOT EXISTS segments (
    task_id TEXT NOT NULL,
    segment_index INTEGER NOT NULL,
    start_byte INTEGER NOT NULL,
    end_byte INTEGER NOT NULL,
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    is_finished INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (task_id, segment_index),
    FOREIGN KEY(task_id) REFERENCES downloads(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS telegram_accounts (
    account_id TEXT PRIMARY KEY,
    account_type TEXT NOT NULL,
    phone_number TEXT,
    username TEXT,
    is_active INTEGER NOT NULL DEFAULT 0,
    is_premium INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS telegram_dialogs (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    username TEXT,
    chat_type TEXT NOT NULL,
    unread_count INTEGER NOT NULL DEFAULT 0,
    photo_url TEXT,
    is_private INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
"#;
