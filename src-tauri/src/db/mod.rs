pub mod schema;

use std::path::PathBuf;
use std::sync::Mutex;
use rusqlite::{params, Connection, Result};
use crate::engine::types::{DownloadCategory, DownloadTask, Segment, TaskStatus};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(schema::CREATE_TABLES)?;
        let _ = conn.execute("ALTER TABLE downloads ADD COLUMN referer TEXT", []);
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(schema::CREATE_TABLES)?;
        let _ = conn.execute("ALTER TABLE downloads ADD COLUMN referer TEXT", []);
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_task(&self, task: &DownloadTask) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let status_str = serde_json::to_string(&task.status).unwrap_or_else(|_| "queued".to_string());
        let category_str = serde_json::to_string(&task.category).unwrap_or_else(|_| "general".to_string());

        conn.execute(
            r#"INSERT OR REPLACE INTO downloads (
                id, url, filename, save_dir, file_path, total_bytes, downloaded_bytes,
                category, status, connections, supports_range, is_hls, created_at,
                completed_at, error_message, referer
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"#,
            params![
                task.id,
                task.url,
                task.filename,
                task.save_dir,
                task.file_path,
                task.total_bytes,
                task.downloaded_bytes,
                category_str,
                status_str,
                task.connections as i64,
                if task.supports_range { 1 } else { 0 },
                if task.is_hls { 1 } else { 0 },
                task.created_at,
                task.completed_at,
                task.error_message,
                task.referer
            ],
        )?;

        // Update segments
        for seg in &task.segments {
            conn.execute(
                r#"INSERT OR REPLACE INTO segments (
                    task_id, segment_index, start_byte, end_byte, downloaded_bytes, is_finished
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
                params![
                    task.id,
                    seg.index as i64,
                    seg.start_byte as i64,
                    seg.end_byte as i64,
                    seg.downloaded_bytes as i64,
                    if seg.is_finished { 1 } else { 0 }
                ],
            )?;
        }

        Ok(())
    }

    pub fn update_task_progress(&self, task_id: &str, downloaded: u64, status: &TaskStatus, segments: &[Segment]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let status_str = serde_json::to_string(status).unwrap_or_else(|_| "downloading".to_string());

        conn.execute(
            "UPDATE downloads SET downloaded_bytes = ?1, status = ?2 WHERE id = ?3",
            params![downloaded, status_str, task_id],
        )?;

        for seg in segments {
            conn.execute(
                "UPDATE segments SET downloaded_bytes = ?1, is_finished = ?2 WHERE task_id = ?3 AND segment_index = ?4",
                params![seg.downloaded_bytes as i64, if seg.is_finished { 1 } else { 0 }, task_id, seg.index as i64],
            )?;
        }

        Ok(())
    }

    pub fn update_task_file_path(&self, task_id: &str, new_save_dir: &str, new_file_path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE downloads SET save_dir = ?1, file_path = ?2 WHERE id = ?3",
            params![new_save_dir, new_file_path, task_id],
        )?;
        Ok(())
    }

    pub fn mark_task_completed(&self, task_id: &str, completed_at: &str, final_bytes: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let status_str = serde_json::to_string(&TaskStatus::Completed).unwrap();
        conn.execute(
            "UPDATE downloads SET status = ?1, completed_at = ?2, downloaded_bytes = ?3, total_bytes = ?3 WHERE id = ?4",
            params![status_str, completed_at, final_bytes as i64, task_id],
        )?;
        Ok(())
    }

    pub fn mark_task_failed(&self, task_id: &str, err_msg: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let status_str = serde_json::to_string(&TaskStatus::Failed(err_msg.to_string())).unwrap();
        conn.execute(
            "UPDATE downloads SET status = ?1, error_message = ?2 WHERE id = ?3",
            params![status_str, err_msg, task_id],
        )?;
        Ok(())
    }

    pub fn delete_task(&self, task_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM segments WHERE task_id = ?1", params![task_id])?;
        conn.execute("DELETE FROM downloads WHERE id = ?1", params![task_id])?;
        Ok(())
    }

    pub fn load_all_tasks(&self) -> Result<Vec<DownloadTask>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT 
                id, url, filename, save_dir, file_path, total_bytes, downloaded_bytes,
                category, status, connections, supports_range, is_hls, created_at,
                completed_at, error_message, referer
            FROM downloads ORDER BY created_at DESC"#,
        )?;

        let mut tasks = Vec::new();
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let url: String = row.get(1)?;
            let filename: String = row.get(2)?;
            let save_dir: String = row.get(3)?;
            let file_path: String = row.get(4)?;
            let total_bytes: Option<i64> = row.get(5)?;
            let downloaded_bytes: i64 = row.get(6)?;
            let category_str: String = row.get(7)?;
            let status_str: String = row.get(8)?;
            let connections: i64 = row.get(9)?;
            let supports_range: i64 = row.get(10)?;
            let is_hls: i64 = row.get(11)?;
            let created_at: String = row.get(12)?;
            let completed_at: Option<String> = row.get(13)?;
            let error_message: Option<String> = row.get(14)?;
            let referer: Option<String> = row.get(15)?;

            let category: DownloadCategory = serde_json::from_str(&category_str).unwrap_or(DownloadCategory::General);
            let mut status: TaskStatus = serde_json::from_str(&status_str).unwrap_or(TaskStatus::Queued);
            
            // If it was downloading when app closed, set to paused so user can resume
            if status == TaskStatus::Downloading {
                status = TaskStatus::Paused;
            }

            Ok(DownloadTask {
                id,
                url,
                filename,
                save_dir,
                file_path,
                total_bytes: total_bytes.map(|b| b as u64),
                downloaded_bytes: downloaded_bytes as u64,
                category,
                status,
                connections: connections as usize,
                supports_range: supports_range == 1,
                is_hls: is_hls == 1,
                created_at,
                completed_at,
                error_message,
                segments: Vec::new(),
                referer,
            })
        })?;

        for r in rows {
            if let Ok(mut task) = r {
                let mut seg_stmt = conn.prepare(
                    "SELECT segment_index, start_byte, end_byte, downloaded_bytes, is_finished FROM segments WHERE task_id = ?1 ORDER BY segment_index ASC"
                )?;
                let seg_rows = seg_stmt.query_map(params![task.id], |s_row| {
                    let idx: i64 = s_row.get(0)?;
                    let start: i64 = s_row.get(1)?;
                    let end: i64 = s_row.get(2)?;
                    let downloaded: i64 = s_row.get(3)?;
                    let is_finished: i64 = s_row.get(4)?;
                    Ok(Segment {
                        index: idx as usize,
                        start_byte: start as u64,
                        end_byte: end as u64,
                        downloaded_bytes: downloaded as u64,
                        is_finished: is_finished == 1,
                    })
                })?;

                for sr in seg_rows {
                    if let Ok(seg) = sr {
                        task.segments.push(seg);
                    }
                }
                tasks.push(task);
            }
        }

        Ok(tasks)
    }

    pub fn update_task_url(&self, task_id: &str, new_url: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE downloads SET url = ?1 WHERE id = ?2",
            params![new_url, task_id],
        )?;
        Ok(())
    }

    pub fn update_task_referer(&self, task_id: &str, referer: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE downloads SET referer = ?1 WHERE id = ?2",
            params![referer, task_id],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)", params![key, value])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_sample_task(id: &str, status: TaskStatus) -> DownloadTask {
        DownloadTask {
            id: id.to_string(),
            url: "https://example.com/test.zip".to_string(),
            filename: "test.zip".to_string(),
            save_dir: "C:\\Downloads".to_string(),
            file_path: "C:\\Downloads\\test.zip".to_string(),
            total_bytes: Some(1000),
            downloaded_bytes: 200,
            category: DownloadCategory::Compressed,
            status,
            connections: 2,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 10:00:00".to_string(),
            completed_at: None,
            error_message: None,
            segments: vec![
                Segment {
                    index: 0,
                    start_byte: 0,
                    end_byte: 499,
                    downloaded_bytes: 200,
                    is_finished: false,
                },
                Segment {
                    index: 1,
                    start_byte: 500,
                    end_byte: 999,
                    downloaded_bytes: 0,
                    is_finished: false,
                },
            ],
            referer: Some("https://example.com/download-page".to_string()),
        }
    }

    #[test]
    fn test_db_insert_and_load_tasks() {
        let db = Database::open_in_memory().expect("open in-memory db");
        let task = create_sample_task("task-1", TaskStatus::Queued);

        db.insert_task(&task).expect("insert task");

        let tasks = db.load_all_tasks().expect("load tasks");
        assert_eq!(tasks.len(), 1);
        let loaded = &tasks[0];
        assert_eq!(loaded.id, "task-1");
        assert_eq!(loaded.filename, "test.zip");
        assert_eq!(loaded.total_bytes, Some(1000));
        assert_eq!(loaded.downloaded_bytes, 200);
        assert_eq!(loaded.segments.len(), 2);
        assert_eq!(loaded.segments[0].start_byte, 0);
        assert_eq!(loaded.segments[1].end_byte, 999);
    }

    #[test]
    fn test_db_update_progress() {
        let db = Database::open_in_memory().expect("open in-memory db");
        let task = create_sample_task("task-prog", TaskStatus::Downloading);
        db.insert_task(&task).expect("insert task");

        let updated_segs = vec![
            Segment {
                index: 0,
                start_byte: 0,
                end_byte: 499,
                downloaded_bytes: 500,
                is_finished: true,
            },
            Segment {
                index: 1,
                start_byte: 500,
                end_byte: 999,
                downloaded_bytes: 100,
                is_finished: false,
            },
        ];

        db.update_task_progress("task-prog", 600, &TaskStatus::Downloading, &updated_segs)
            .expect("update progress");

        let tasks = db.load_all_tasks().expect("load tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].downloaded_bytes, 600);
        assert_eq!(tasks[0].segments[0].is_finished, true);
        assert_eq!(tasks[0].segments[1].downloaded_bytes, 100);
    }

    #[test]
    fn test_db_mark_completed_and_failed() {
        let db = Database::open_in_memory().expect("open in-memory db");
        let task1 = create_sample_task("task-done", TaskStatus::Downloading);
        let task2 = create_sample_task("task-fail", TaskStatus::Downloading);

        db.insert_task(&task1).expect("insert task 1");
        db.insert_task(&task2).expect("insert task 2");

        db.mark_task_completed("task-done", "2026-09-20 10:15:00", 1000).expect("complete");
        db.mark_task_failed("task-fail", "Server returned 404").expect("fail");

        let tasks = db.load_all_tasks().expect("load tasks");
        let t_done = tasks.iter().find(|t| t.id == "task-done").expect("find done");
        let t_fail = tasks.iter().find(|t| t.id == "task-fail").expect("find fail");

        assert_eq!(t_done.status, TaskStatus::Completed);
        assert_eq!(t_done.completed_at, Some("2026-09-20 10:15:00".to_string()));
        assert_eq!(t_done.downloaded_bytes, 1000);
        assert_eq!(t_done.total_bytes, Some(1000));

        assert_eq!(t_fail.status, TaskStatus::Failed("Server returned 404".to_string()));
        assert_eq!(t_fail.error_message, Some("Server returned 404".to_string()));
    }

    #[test]
    fn test_db_delete_task() {
        let db = Database::open_in_memory().expect("open in-memory db");
        let task = create_sample_task("task-del", TaskStatus::Queued);
        db.insert_task(&task).expect("insert");

        assert_eq!(db.load_all_tasks().unwrap().len(), 1);

        db.delete_task("task-del").expect("delete");
        assert_eq!(db.load_all_tasks().unwrap().len(), 0);
    }

    #[test]
    fn test_db_settings() {
        let db = Database::open_in_memory().expect("open in-memory db");

        assert_eq!(db.get_setting("max_connections").unwrap(), None);

        db.set_setting("max_connections", "16").expect("set setting");
        assert_eq!(db.get_setting("max_connections").unwrap(), Some("16".to_string()));

        // Overwrite
        db.set_setting("max_connections", "32").expect("set setting");
        assert_eq!(db.get_setting("max_connections").unwrap(), Some("32".to_string()));
    }

    #[test]
    fn test_downloading_resets_to_paused_on_load() {
        let db = Database::open_in_memory().expect("open in-memory db");
        let task = create_sample_task("task-interrupted", TaskStatus::Downloading);
        db.insert_task(&task).expect("insert");

        let loaded = db.load_all_tasks().expect("load");
        assert_eq!(loaded[0].status, TaskStatus::Paused);
    }

    #[test]
    fn test_db_update_task_file_path() {
        let db = Database::open_in_memory().expect("open in-memory db");
        let task = create_sample_task("task-move", TaskStatus::Completed);
        db.insert_task(&task).expect("insert");

        db.update_task_file_path("task-move", "D:\\NewFolder", "D:\\NewFolder\\test.mp4")
            .expect("update path");

        let loaded = db.load_all_tasks().expect("load");
        assert_eq!(loaded[0].save_dir, "D:\\NewFolder");
        assert_eq!(loaded[0].file_path, "D:\\NewFolder\\test.mp4");
    }

    #[test]
    fn test_db_update_task_url_and_referer() {
        let db = Database::open_in_memory().expect("open in-memory db");
        let task = create_sample_task("task-ref-1", TaskStatus::Paused);
        db.insert_task(&task).expect("insert");

        db.update_task_url("task-ref-1", "https://new-cdn.example.com/test.zip").expect("update url");
        db.update_task_referer("task-ref-1", "https://example.com/fresh-page").expect("update referer");

        let loaded = db.load_all_tasks().expect("load");
        assert_eq!(loaded[0].url, "https://new-cdn.example.com/test.zip");
        assert_eq!(loaded[0].referer, Some("https://example.com/fresh-page".to_string()));
        assert_eq!(loaded[0].downloaded_bytes, 200); // Bytes preserved
    }
}
