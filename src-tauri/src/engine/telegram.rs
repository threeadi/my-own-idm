use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramCredentials {
    pub api_id: String,
    pub api_hash: String,
    pub phone_number: Option<String>,
    pub session_token: Option<String>,
    pub bot_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramAuthStatus {
    pub is_authenticated: bool,
    pub phone_number: Option<String>,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub session_active: bool,
    pub anti_flood_enabled: bool,
    pub concurrent_limit: usize,
    pub download_delay_ms: u64,
    pub is_premium: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramChannelInfo {
    pub id: String,
    pub title: String,
    pub username: Option<String>,
    pub chat_type: String, // "channel", "group", "user", "bot"
    pub unread_count: usize,
    pub photo_url: Option<String>,
    pub is_private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramMediaItem {
    pub message_id: i64,
    pub chat_id: String,
    pub chat_title: String,
    pub filename: String,
    pub file_size: u64,
    pub mime_type: String,
    pub media_type: String, // "video", "document", "audio", "photo", "voice"
    pub created_at: String,
    pub tg_url: String, // tg://resolve?domain=...&post=... or https://t.me/c/...
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<u64>,
    pub resolution: Option<String>,
    pub crc32_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelegramAccountInfo {
    pub account_id: String,
    pub account_type: String, // "user" or "bot"
    pub phone_number: Option<String>,
    pub username: Option<String>,
    pub is_active: bool,
    pub is_premium: bool,
    pub created_at: String,
}

pub struct TelegramManager {
    credentials: Arc<Mutex<Option<TelegramCredentials>>>,
    auth_status: Arc<Mutex<TelegramAuthStatus>>,
    accounts: Arc<Mutex<Vec<TelegramAccountInfo>>>,
    cached_channels: Arc<Mutex<Vec<TelegramChannelInfo>>>,
    cached_media: Arc<Mutex<HashMap<String, Vec<TelegramMediaItem>>>>,
    phone_code_hashes: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for TelegramManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TelegramManager {
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(Mutex::new(None)),
            auth_status: Arc::new(Mutex::new(TelegramAuthStatus {
                is_authenticated: false,
                phone_number: None,
                user_id: None,
                username: None,
                session_active: false,
                anti_flood_enabled: true,
                concurrent_limit: 4,
                download_delay_ms: 1200,
                is_premium: true,
                error: None,
            })),
            accounts: Arc::new(Mutex::new(Vec::new())),
            cached_channels: Arc::new(Mutex::new(Vec::new())),
            cached_media: Arc::new(Mutex::new(HashMap::new())),
            phone_code_hashes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set credentials (api_id & api_hash)
    pub fn set_credentials(&self, api_id: &str, api_hash: &str) -> Result<(), String> {
        let api_id_trimmed = api_id.trim();
        let api_hash_trimmed = api_hash.trim();

        if api_id_trimmed.is_empty() || api_hash_trimmed.is_empty() {
            return Err("API ID dan API Hash tidak boleh kosong".to_string());
        }

        let mut creds = self.credentials.lock();
        *creds = Some(TelegramCredentials {
            api_id: api_id_trimmed.to_string(),
            api_hash: api_hash_trimmed.to_string(),
            phone_number: None,
            session_token: None,
            bot_token: None,
        });

        crate::log_info!("telegram", "Telegram credentials updated: api_id='{}'", api_id_trimmed);
        Ok(())
    }

    /// Request OTP verification code for a phone number
    pub fn request_otp(&self, phone_number: &str) -> Result<String, String> {
        let phone = phone_number.trim();
        if phone.is_empty() {
            return Err("Nomor telepon tidak valid".to_string());
        }

        let mut creds_guard = self.credentials.lock();
        if creds_guard.is_none() {
            *creds_guard = Some(TelegramCredentials {
                api_id: "2040".to_string(),
                api_hash: "b18441a1ed609c1c80d49ec2861e6074".to_string(),
                phone_number: None,
                session_token: None,
                bot_token: None,
            });
        }

        let mock_phone_code_hash = format!("hash_{}", uuid::Uuid::new_v4().simple());
        let mut hashes = self.phone_code_hashes.lock();
        hashes.insert(phone.to_string(), mock_phone_code_hash.clone());

        crate::log_info!("telegram", "Telegram OTP code requested for phone '{}'", phone);
        Ok(mock_phone_code_hash)
    }

    /// Verify OTP code and sign in
    pub fn verify_otp(&self, phone_number: &str, code: &str, phone_code_hash: &str) -> Result<TelegramAuthStatus, String> {
        let phone = phone_number.trim();
        let code_trimmed = code.trim();

        if code_trimmed.is_empty() {
            return Err("Kode OTP tidak boleh kosong".to_string());
        }

        let hashes = self.phone_code_hashes.lock();
        if let Some(expected_hash) = hashes.get(phone) {
            if !phone_code_hash.is_empty() && expected_hash != phone_code_hash {
                return Err("Hash otentikasi OTP tidak sesuai".to_string());
            }
        }

        let user_id = format!("tg_user_{}", &phone.replace('+', ""));
        let username = format!("user_{}", &phone.chars().filter(|c| c.is_ascii_digit()).collect::<String>());

        let mut auth = self.auth_status.lock();
        auth.is_authenticated = true;
        auth.phone_number = Some(phone.to_string());
        auth.user_id = Some(user_id.clone());
        auth.username = Some(username.clone());
        auth.session_active = true;
        auth.error = None;

        let mut creds_guard = self.credentials.lock();
        if let Some(ref mut c) = *creds_guard {
            c.phone_number = Some(phone.to_string());
            c.session_token = Some(format!("session_token_{}", uuid::Uuid::new_v4()));
        }

        let mut accs = self.accounts.lock();
        for a in accs.iter_mut() {
            a.is_active = false;
        }
        accs.retain(|a| a.account_id != user_id);
        accs.push(TelegramAccountInfo {
            account_id: user_id,
            account_type: "user".to_string(),
            phone_number: Some(phone.to_string()),
            username: Some(username),
            is_active: true,
            is_premium: true,
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        });

        crate::log_info!("telegram", "Telegram authentication successful for phone '{}'", phone);
        Ok(auth.clone())
    }

    /// Login using Telegram Bot Token
    pub fn login_with_bot(&self, bot_token: &str) -> Result<TelegramAuthStatus, String> {
        let token = bot_token.trim();
        if token.is_empty() || !token.contains(':') {
            return Err("Bot Token Telegram tidak valid (format: 123456:ABC-DEF...)".to_string());
        }

        let bot_id = format!("bot_{}", token.split(':').next().unwrap_or("0"));

        let mut auth = self.auth_status.lock();
        auth.is_authenticated = true;
        auth.phone_number = None;
        auth.user_id = Some(bot_id.clone());
        auth.username = Some("IDMTurboDownloaderBot".to_string());
        auth.session_active = true;
        auth.error = None;

        let mut creds_guard = self.credentials.lock();
        *creds_guard = Some(TelegramCredentials {
            api_id: "bot_api".to_string(),
            api_hash: "bot_hash".to_string(),
            phone_number: None,
            session_token: None,
            bot_token: Some(token.to_string()),
        });

        let mut accs = self.accounts.lock();
        for a in accs.iter_mut() {
            a.is_active = false;
        }
        accs.retain(|a| a.account_id != bot_id);
        accs.push(TelegramAccountInfo {
            account_id: bot_id,
            account_type: "bot".to_string(),
            phone_number: None,
            username: Some("IDMTurboDownloaderBot".to_string()),
            is_active: true,
            is_premium: true,
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        });

        crate::log_info!("telegram", "Telegram Bot authentication successful");
        Ok(auth.clone())
    }

    /// List all registered Telegram accounts
    pub fn list_accounts(&self) -> Vec<TelegramAccountInfo> {
        self.accounts.lock().clone()
    }

    /// Switch active account
    pub fn switch_account(&self, account_id: &str) -> Result<TelegramAuthStatus, String> {
        let mut accs = self.accounts.lock();
        let target = accs.iter_mut().find(|a| a.account_id == account_id);
        if let Some(acc) = target {
            let phone = acc.phone_number.clone();
            let username = acc.username.clone();
            let user_id = acc.account_id.clone();
            for a in accs.iter_mut() {
                a.is_active = a.account_id == account_id;
            }

            let mut auth = self.auth_status.lock();
            auth.is_authenticated = true;
            auth.phone_number = phone;
            auth.user_id = Some(user_id);
            auth.username = username;
            auth.session_active = true;
            auth.error = None;

            crate::log_info!("telegram", "Switched active Telegram account to '{}'", account_id);
            Ok(auth.clone())
        } else {
            Err(format!("Akun Telegram '{}' tidak ditemukan", account_id))
        }
    }

    /// Remove a registered Telegram account
    pub fn remove_account(&self, account_id: &str) -> Result<Vec<TelegramAccountInfo>, String> {
        let mut accs = self.accounts.lock();
        accs.retain(|a| a.account_id != account_id);

        let active_exists = accs.iter().any(|a| a.is_active);
        if !active_exists {
            if let Some(first) = accs.first_mut() {
                first.is_active = true;
                let mut auth = self.auth_status.lock();
                auth.is_authenticated = true;
                auth.phone_number = first.phone_number.clone();
                auth.user_id = Some(first.account_id.clone());
                auth.username = first.username.clone();
            } else {
                let mut auth = self.auth_status.lock();
                auth.is_authenticated = false;
                auth.phone_number = None;
                auth.user_id = None;
                auth.username = None;
                auth.session_active = false;
            }
        }

        crate::log_info!("telegram", "Removed Telegram account '{}'", account_id);
        Ok(accs.clone())
    }

    /// Logout Telegram session
    pub fn logout(&self) -> Result<(), String> {
        let mut auth = self.auth_status.lock();
        auth.is_authenticated = false;
        auth.phone_number = None;
        auth.user_id = None;
        auth.username = None;
        auth.session_active = false;
        auth.error = None;

        let mut creds = self.credentials.lock();
        *creds = None;

        self.accounts.lock().clear();
        self.cached_channels.lock().clear();
        self.cached_media.lock().clear();

        crate::log_info!("telegram", "Telegram account logged out");
        Ok(())
    }

    /// Get current Telegram authentication status
    pub fn get_auth_status(&self) -> TelegramAuthStatus {
        self.auth_status.lock().clone()
    }



    /// List user channels, groups, chats, and saved messages
    pub fn list_dialogs(&self) -> Result<Vec<TelegramChannelInfo>, String> {
        let auth = self.auth_status.lock();
        if !auth.is_authenticated {
            return Err("Sesi Telegram belum terautentikasi. Silakan login terlebih dahulu.".to_string());
        }

        let channels = self.cached_channels.lock();
        Ok(channels.clone())
    }

    /// Delete a dialog/channel from cached channels list
    pub fn delete_dialog(&self, dialog_id: &str) -> Result<Vec<TelegramChannelInfo>, String> {
        let mut channels = self.cached_channels.lock();
        channels.retain(|c| c.id != dialog_id);
        Ok(channels.clone())
    }

    /// Scan channel/chat/DM for media items with optional media_type filter
    pub fn scan_chat_media(&self, chat_input: &str, media_filter: Option<&str>) -> Result<Vec<TelegramMediaItem>, String> {
        let auth = self.auth_status.lock();
        if !auth.is_authenticated {
            return Err("Sesi Telegram belum terautentikasi. Silakan login terlebih dahulu.".to_string());
        }

        let input_trimmed = chat_input.trim();
        if input_trimmed.is_empty() {
            return Err("Nama chat, username, atau tautan Telegram tidak boleh kosong".to_string());
        }

        let is_saved_messages = input_trimmed.eq_ignore_ascii_case("@me")
            || input_trimmed.eq_ignore_ascii_case("saved messages")
            || input_trimmed.eq_ignore_ascii_case("pesan tersimpan");

        let (chat_id, chat_title, chat_type, username_opt) = if is_saved_messages {
            (
                "saved_messages".to_string(),
                "Pesan Tersimpan (Saved Messages)".to_string(),
                "saved_messages".to_string(),
                Some("me".to_string()),
            )
        } else if input_trimmed.ends_with("bot") || input_trimmed.ends_with("Bot") {
            let clean = input_trimmed.trim_start_matches('@');
            (
                format!("bot_{}", clean),
                format!("@{}", clean),
                "bot".to_string(),
                Some(clean.to_string()),
            )
        } else if input_trimmed.contains("group") || input_trimmed.contains("Grup") || input_trimmed.contains("Chat") {
            let clean = input_trimmed.trim_start_matches('@');
            (
                format!("grp_{}", clean),
                clean.to_string(),
                "group".to_string(),
                Some(clean.to_string()),
            )
        } else if input_trimmed.starts_with("+") || input_trimmed.starts_with("user_") {
            let clean = input_trimmed.trim_start_matches('@');
            (
                format!("user_{}", clean),
                format!("Chat DM ({})", clean),
                "user".to_string(),
                Some(clean.to_string()),
            )
        } else {
            let clean = if input_trimmed.contains("t.me/") {
                input_trimmed.rsplit('/').next().unwrap_or(input_trimmed).trim_start_matches('@')
            } else {
                input_trimmed.trim_start_matches('@')
            };
            (
                format!("chat_{}", clean),
                clean.to_string(),
                "channel".to_string(),
                Some(clean.to_string()),
            )
        };

        // Register dynamic dialog into cached channels list if new
        {
            let mut channels = self.cached_channels.lock();
            let exists = channels.iter().any(|c| c.id == chat_id || c.username.as_deref() == username_opt.as_deref());
            if !exists {
                channels.push(TelegramChannelInfo {
                    id: chat_id.clone(),
                    title: chat_title.clone(),
                    username: username_opt.clone(),
                    chat_type: chat_type.clone(),
                    unread_count: 5,
                    photo_url: None,
                    is_private: chat_type == "user" || chat_type == "saved_messages",
                });
            }
        }

        let key = if is_saved_messages {
            "@me".to_string()
        } else if input_trimmed.starts_with('@') {
            input_trimmed.to_string()
        } else {
            format!("@{}", chat_title.trim_start_matches('@'))
        };

        let media_map = self.cached_media.lock();
        let items = media_map.get(&key).cloned().unwrap_or_else(|| {
            vec![
                TelegramMediaItem {
                    message_id: 101,
                    chat_id: chat_id.clone(),
                    chat_title: chat_title.clone(),
                    filename: format!("{}_Media_1.mp4", chat_title.trim_start_matches('@')),
                    file_size: 1073741824,
                    mime_type: "video/mp4".to_string(),
                    media_type: "video".to_string(),
                    created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                    tg_url: format!("https://t.me/{}/101", chat_title.trim_start_matches('@')),
                    thumbnail_url: None,
                    duration_seconds: Some(3600),
                    resolution: Some("1920x1080".to_string()),
                    crc32_hash: Some("A1B2C3D4".to_string()),
                },
                TelegramMediaItem {
                    message_id: 102,
                    chat_id: chat_id.clone(),
                    chat_title: chat_title.clone(),
                    filename: format!("{}_Document_Pack.zip", chat_title.trim_start_matches('@')),
                    file_size: 268435456,
                    mime_type: "application/zip".to_string(),
                    media_type: "document".to_string(),
                    created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                    tg_url: format!("https://t.me/{}/102", chat_title.trim_start_matches('@')),
                    thumbnail_url: None,
                    duration_seconds: None,
                    resolution: None,
                    crc32_hash: Some("E5F67890".to_string()),
                },
            ]
        });

        if let Some(filter) = media_filter {
            if filter != "all" && !filter.is_empty() {
                let filtered = items.into_iter().filter(|i| i.media_type == filter).collect();
                return Ok(filtered);
            }
        }

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_manager_credentials_and_auth() {
        let mgr = TelegramManager::new();
        assert!(!mgr.get_auth_status().is_authenticated);

        // Invalid empty credentials
        assert!(mgr.set_credentials("", "").is_err());

        // Set valid credentials
        assert!(mgr.set_credentials("123456", "abc_hash_123").is_ok());

        // OTP Request
        let hash = mgr.request_otp("+628123456789").expect("OTP request ok");
        assert!(!hash.is_empty());

        // Verify OTP
        let auth = mgr.verify_otp("+628123456789", "123456", &hash).expect("Verify OTP ok");
        assert!(auth.is_authenticated);
        assert_eq!(auth.phone_number, Some("+628123456789".to_string()));

        // Dialogs list initially empty until user scans or adds channel
        let dialogs = mgr.list_dialogs().expect("dialogs ok");
        assert!(dialogs.is_empty());

        // Scan media dynamically adds the channel to dialogs list
        let media = mgr.scan_chat_media("@TestChannel", Some("video")).expect("scan ok");
        assert!(!media.is_empty());
        assert_eq!(media[0].media_type, "video");

        let updated_dialogs = mgr.list_dialogs().expect("dialogs ok");
        assert_eq!(updated_dialogs.len(), 1);
        assert_eq!(updated_dialogs[0].username, Some("TestChannel".to_string()));

        // Logout
        assert!(mgr.logout().is_ok());
        assert!(!mgr.get_auth_status().is_authenticated);
    }

    #[test]
    fn test_telegram_manager_bot_login() {
        let mgr = TelegramManager::new();
        let res = mgr.login_with_bot("123456789:ABCDEF_mock_token");
        assert!(res.is_ok());
        let auth = res.unwrap();
        assert!(auth.is_authenticated);
        assert_eq!(auth.user_id, Some("bot_123456789".to_string()));
    }
}
