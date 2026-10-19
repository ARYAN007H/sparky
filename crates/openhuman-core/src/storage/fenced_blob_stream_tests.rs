use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tinystoragedrivers::BlobReader;

/// A native streaming driver that records whether the fenced view reaches its
/// streaming entry point instead of collecting the body and calling `put`.
#[derive(Debug, Default)]
struct CountingBlobs {
    puts: AtomicU64,
    expiring_puts: AtomicU64,
    stream_puts: AtomicU64,
    expiring_stream_puts: AtomicU64,
    stream_bytes: AtomicU64,
    stream_gets: AtomicU64,
    presigned_gets: AtomicU64,
    presigned_puts: AtomicU64,
}

#[async_trait]
impl BlobStore for CountingBlobs {
    fn capabilities(&self) -> Capabilities {
        Capabilities::none()
            .with(Capability::StreamingBlobs)
            .with(Capability::BlobTtl)
            .with(Capability::PresignedUrls)
    }

    async fn put(&self, key: &str, bytes: Vec<u8>, content_type: Option<&str>) -> Result<BlobMeta> {
        self.puts.fetch_add(1, Ordering::SeqCst);
        Ok(BlobMeta {
            key: key.to_string(),
            len: bytes.len() as u64,
            content_type: content_type.map(str::to_string),
        })
    }

    async fn put_stream(
        &self,
        key: &str,
        mut reader: BlobReader,
        len: u64,
        content_type: Option<&str>,
    ) -> Result<BlobMeta> {
        use tokio::io::AsyncReadExt;

        self.stream_puts.fetch_add(1, Ordering::SeqCst);
        let mut buf = [0u8; 8 * 1024];
        let mut read = 0u64;
        loop {
            let n = reader
                .read(&mut buf)
                .await
                .map_err(|error| StorageError::backend("test reader failed").with_source(error))?;
            if n == 0 {
                break;
            }
            read += n as u64;
        }
        self.stream_bytes.store(read, Ordering::SeqCst);
        if read != len {
            return Err(StorageError::invalid_input("stream length mismatch"));
        }
        Ok(BlobMeta {
            key: key.to_string(),
            len,
            content_type: content_type.map(str::to_string),
        })
    }

    async fn put_expiring(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: Option<&str>,
        _: Duration,
    ) -> Result<BlobMeta> {
        self.expiring_puts.fetch_add(1, Ordering::SeqCst);
        Ok(BlobMeta {
            key: key.to_string(),
            len: bytes.len() as u64,
            content_type: content_type.map(str::to_string),
        })
    }

    async fn put_stream_expiring(
        &self,
        key: &str,
        mut reader: BlobReader,
        len: u64,
        content_type: Option<&str>,
        _: Duration,
    ) -> Result<BlobMeta> {
        use tokio::io::AsyncReadExt;

        self.expiring_stream_puts.fetch_add(1, Ordering::SeqCst);
        let mut buf = [0u8; 8 * 1024];
        let mut read = 0u64;
        loop {
            let n = reader
                .read(&mut buf)
                .await
                .map_err(|error| StorageError::backend("test reader failed").with_source(error))?;
            if n == 0 {
                break;
            }
            read += n as u64;
        }
        self.stream_bytes.store(read, Ordering::SeqCst);
        if read != len {
            return Err(StorageError::invalid_input("stream length mismatch"));
        }
        Ok(BlobMeta {
            key: key.to_string(),
            len,
            content_type: content_type.map(str::to_string),
        })
    }

    async fn get_stream(&self, key: &str) -> Result<Option<BlobStream>> {
        self.stream_gets.fetch_add(1, Ordering::SeqCst);
        Ok(Some(BlobStream {
            meta: BlobMeta {
                key: key.to_string(),
                len: 1,
                content_type: None,
            },
            reader: Box::pin(std::io::Cursor::new(vec![7])),
        }))
    }

    async fn presign_get(&self, _: &str, _: Duration) -> Result<String> {
        self.presigned_gets.fetch_add(1, Ordering::SeqCst);
        Ok("https://example.test/read".to_string())
    }

    async fn presign_put(&self, _: &str, _: Duration) -> Result<String> {
        self.presigned_puts.fetch_add(1, Ordering::SeqCst);
        Ok("https://example.test/write".to_string())
    }

    async fn get(&self, _: &str) -> Result<Option<Blob>> {
        Ok(None)
    }

    async fn get_range(&self, _: &str, _: Range<u64>) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    async fn head(&self, _: &str) -> Result<Option<BlobMeta>> {
        Ok(None)
    }

    async fn delete(&self, _: &str) -> Result<bool> {
        Ok(false)
    }

    async fn list(&self, _: &str) -> Result<Vec<BlobMeta>> {
        Ok(Vec::new())
    }
}

#[derive(Debug)]
struct LogicalReader {
    remaining: u64,
    bytes_read: Arc<AtomicU64>,
}

impl tokio::io::AsyncRead for LogicalReader {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let n = self.remaining.min(buf.remaining() as u64) as usize;
        if n > 0 {
            buf.put_slice(&vec![3; n]);
            self.remaining -= n as u64;
            self.bytes_read.fetch_add(n as u64, Ordering::SeqCst);
        }
        std::task::Poll::Ready(Ok(()))
    }
}

fn no_fence_guard() -> Guard {
    Guard {
        scope: Scope::local(),
        fences: Arc::new(FenceRegistry::new(|| 0)),
        backend: None,
    }
}

#[tokio::test]
async fn fenced_blobs_keeps_large_uploads_on_the_native_stream_path() {
    let inner = Arc::new(CountingBlobs::default());
    let blobs = FencedBlobs {
        inner: Arc::clone(&inner) as Arc<dyn BlobStore>,
        guard: no_fence_guard(),
    };
    let len = 32 * 1024 * 1024;
    let bytes_read = Arc::new(AtomicU64::new(0));
    let reader = LogicalReader {
        remaining: len,
        bytes_read: Arc::clone(&bytes_read),
    };

    let meta = blobs
        .put_stream(
            "large",
            Box::pin(reader),
            len,
            Some("application/octet-stream"),
        )
        .await
        .unwrap();

    assert_eq!(meta.len, len);
    assert_eq!(bytes_read.load(Ordering::SeqCst), len);
    assert_eq!(inner.stream_puts.load(Ordering::SeqCst), 1);
    assert_eq!(inner.stream_bytes.load(Ordering::SeqCst), len);
    assert_eq!(inner.puts.load(Ordering::SeqCst), 0);
    assert!(blobs.capabilities().contains(Capability::StreamingBlobs));

    let expiring_len = 32 * 1024 * 1024;
    let expiring_reader = LogicalReader {
        remaining: expiring_len,
        bytes_read: Arc::clone(&bytes_read),
    };
    let meta = blobs
        .put_stream_expiring(
            "large-expiring",
            Box::pin(expiring_reader),
            expiring_len,
            Some("application/octet-stream"),
            Duration::from_secs(60),
        )
        .await
        .unwrap();

    assert_eq!(meta.len, expiring_len);
    assert_eq!(bytes_read.load(Ordering::SeqCst), 2 * len);
    assert_eq!(inner.expiring_stream_puts.load(Ordering::SeqCst), 1);
    assert_eq!(inner.expiring_puts.load(Ordering::SeqCst), 0);
    assert_eq!(inner.stream_bytes.load(Ordering::SeqCst), expiring_len);
}

#[tokio::test]
async fn fenced_blobs_delegates_stream_reads_and_does_not_presign_writes() {
    let inner = Arc::new(CountingBlobs::default());
    let blobs = FencedBlobs {
        inner: Arc::clone(&inner) as Arc<dyn BlobStore>,
        guard: no_fence_guard(),
    };

    assert!(blobs.get_stream("read").await.unwrap().is_some());
    assert_eq!(inner.stream_gets.load(Ordering::SeqCst), 1);
    assert_eq!(
        blobs
            .presign_get("read", Duration::from_secs(30))
            .await
            .unwrap(),
        "https://example.test/read"
    );
    assert_eq!(inner.presigned_gets.load(Ordering::SeqCst), 1);
    assert!(!blobs.capabilities().contains(Capability::PresignedUrls));
    assert_eq!(
        blobs
            .presign_put("write", Duration::from_secs(30))
            .await
            .unwrap_err()
            .kind(),
        ErrorKind::Unsupported(Capability::PresignedUrls)
    );
    assert_eq!(inner.presigned_puts.load(Ordering::SeqCst), 0);
}

async fn stale_blob_stream_is_refused_before_the_reader_is_polled(expiring: bool) {
    let backend = shared();
    let (a, b) = (Node::new("node-a", &backend), Node::new("node-b", &backend));
    a.take("alice").await;
    b.set_clock(TTL.as_millis() as u64 + 1);
    b.take("alice").await;
    a.set_clock(1_000);

    let bytes_read = Arc::new(AtomicU64::new(0));
    let reader = LogicalReader {
        remaining: 1024 * 1024,
        bytes_read: Arc::clone(&bytes_read),
    };
    let scoped = a.scoped("profile:alice");
    let blobs = scoped.blobs();
    let outcome = if expiring {
        blobs
            .put_stream_expiring(
                "stale",
                Box::pin(reader),
                1024 * 1024,
                None,
                Duration::from_secs(60),
            )
            .await
    } else {
        blobs
            .put_stream("stale", Box::pin(reader), 1024 * 1024, None)
            .await
    };

    assert!(
        outcome.is_err(),
        "a stale epoch must refuse the stream write"
    );
    assert_eq!(
        bytes_read.load(Ordering::SeqCst),
        0,
        "a stale epoch must be refused before the upload reader is consumed"
    );
}

#[tokio::test]
async fn fenced_blob_stream_upload_is_refused_before_reading_when_epoch_is_stale() {
    stale_blob_stream_is_refused_before_the_reader_is_polled(false).await;
}

#[tokio::test]
async fn fenced_expiring_blob_stream_upload_is_refused_before_reading_when_epoch_is_stale() {
    stale_blob_stream_is_refused_before_the_reader_is_polled(true).await;
}
