use std::io::SeekFrom;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;

pub struct FileWriter {
    pub file_path: String,
    pub total_bytes: Option<u64>,
}

impl FileWriter {
    pub async fn create_or_open(file_path: &str, total_bytes: Option<u64>) -> Result<Self, String> {
        let path = Path::new(file_path);
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        // Open or create file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .await
            .map_err(|e| format!("Failed to create/open file: {}", e))?;

        // Pre-allocate if size known
        if let Some(total) = total_bytes {
            let current_len = file.metadata().await.map(|m| m.len()).unwrap_or(0);
            if current_len < total {
                file.set_len(total)
                    .await
                    .map_err(|e| format!("Failed to pre-allocate file: {}", e))?;
            }
        }

        Ok(Self {
            file_path: file_path.to_string(),
            total_bytes,
        })
    }

    /// Creates an independent worker writer handle with its own seek pointer
    pub async fn open_worker_handle(&self) -> Result<WorkerHandle, String> {
        let file = OpenOptions::new()
            .write(true)
            .open(&self.file_path)
            .await
            .map_err(|e| format!("Worker failed to open file handle: {}", e))?;

        Ok(WorkerHandle {
            file: Arc::new(Mutex::new(file)),
        })
    }
}

pub struct WorkerHandle {
    file: Arc<Mutex<File>>,
}

impl WorkerHandle {
    pub async fn write_at(&self, offset: u64, data: &[u8]) -> Result<(), std::io::Error> {
        let mut file = self.file.lock().await;
        file.seek(SeekFrom::Start(offset)).await?;
        file.write_all(data).await?;
        Ok(())
    }

    pub async fn flush(&self) -> Result<(), std::io::Error> {
        let mut file = self.file.lock().await;
        file.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_writer_preallocation() {
        let dir = tempdir().expect("create temp dir");
        let file_path = dir.path().join("prealloc_test.bin");
        let path_str = file_path.to_string_lossy().to_string();

        let total_size = 1024 * 64; // 64 KB
        let writer = FileWriter::create_or_open(&path_str, Some(total_size))
            .await
            .expect("create writer");

        assert_eq!(writer.total_bytes, Some(total_size));

        // Check metadata on disk
        let metadata = tokio::fs::metadata(&file_path).await.expect("file metadata");
        assert_eq!(metadata.len(), total_size);
    }

    #[tokio::test]
    async fn test_file_writer_concurrent_chunks() {
        let dir = tempdir().expect("create temp dir");
        let file_path = dir.path().join("concurrent_test.bin");
        let path_str = file_path.to_string_lossy().to_string();

        let total_size = 3000;
        let writer = FileWriter::create_or_open(&path_str, Some(total_size))
            .await
            .expect("create writer");

        let handle1 = writer.open_worker_handle().await.expect("worker 1");
        let handle2 = writer.open_worker_handle().await.expect("worker 2");
        let handle3 = writer.open_worker_handle().await.expect("worker 3");

        let chunk1 = vec![1u8; 1000];
        let chunk2 = vec![2u8; 1000];
        let chunk3 = vec![3u8; 1000];

        // Concurrent writes at offsets 0, 1000, 2000
        let (r1, r2, r3) = tokio::join!(
            async {
                handle1.write_at(0, &chunk1).await.expect("write 1");
                handle1.flush().await.expect("flush 1");
            },
            async {
                handle2.write_at(1000, &chunk2).await.expect("write 2");
                handle2.flush().await.expect("flush 2");
            },
            async {
                handle3.write_at(2000, &chunk3).await.expect("write 3");
                handle3.flush().await.expect("flush 3");
            }
        );

        let _ = (r1, r2, r3);

        // Verify final file contents on disk
        let disk_bytes = tokio::fs::read(&file_path).await.expect("read file");
        assert_eq!(disk_bytes.len(), 3000);
        assert_eq!(&disk_bytes[0..1000], &chunk1[..]);
        assert_eq!(&disk_bytes[1000..2000], &chunk2[..]);
        assert_eq!(&disk_bytes[2000..3000], &chunk3[..]);
    }

    #[tokio::test]
    async fn test_file_writer_unknown_size() {
        let dir = tempdir().expect("create temp dir");
        let file_path = dir.path().join("unknown_size.bin");
        let path_str = file_path.to_string_lossy().to_string();

        let writer = FileWriter::create_or_open(&path_str, None)
            .await
            .expect("create writer");

        assert_eq!(writer.total_bytes, None);

        let handle = writer.open_worker_handle().await.expect("worker handle");
        handle.write_at(0, b"Hello World").await.expect("write");
        handle.flush().await.expect("flush");

        let disk_bytes = tokio::fs::read(&file_path).await.expect("read file");
        assert_eq!(disk_bytes, b"Hello World");
    }
}
