//! Shared driver matrix for the `storage_*_e2e` suites and
//! `cli_storage_url_e2e`.
//!
//! Every suite that exercises the core on a configured storage backend runs
//! the same body on memory and SQLite, each as its own test case named after
//! the driver (`memory::...`, `sqlite::...`), so a
//! failure says which driver broke:
//!
//! - `memory`: always;
//! - `sqlite`: when the `storage-sqlite` feature is compiled in, on a temp
//!   directory (the desktop default, one `.db` file per database);
//!
//! The backend slot is process-wide, so the cases of one binary run one at a
//! time (they share [`Case`]'s lock) and the slot is emptied when a case ends,
//! pass or fail.
//!
//! Include with `#[macro_use] mod storage_drivers;` and declare in-process
//! bodies with `driver_cases!(sync body)` (or `async body`). Suites that spawn
//! `openhuman-core` can call `run_url` for the driver they need.

#![allow(dead_code)]

use std::sync::{Arc, Mutex, MutexGuard};

use openhuman_core::storage::{self, StorageBackend};

/// One storage driver of the matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Driver {
    Memory,
    Sqlite,
}

impl Driver {
    /// The driver's name, as in the test case path.
    pub fn name(self) -> &'static str {
        match self {
            Driver::Memory => "memory",
            Driver::Sqlite => "sqlite",
        }
    }

    /// Whether this build carries the driver.
    pub fn compiled_in(self) -> bool {
        match self {
            Driver::Memory => true,
            Driver::Sqlite => cfg!(feature = "storage-sqlite"),
        }
    }

    /// Whether records outlive the process that wrote them.
    pub fn is_durable(self) -> bool {
        self != Driver::Memory
    }

    /// The storage URL for this driver, using `dir` for the on-disk drivers.
    /// `None` when the driver is not compiled in.
    pub fn url(self, dir: &std::path::Path) -> Option<String> {
        if !self.compiled_in() {
            return None;
        }
        match self {
            Driver::Memory => Some("memory".to_string()),
            Driver::Sqlite => Some(format!("sqlite:{}", dir.join("sqlite").display())),
        }
    }
}

/// The slot is process-wide, so cases of one binary take turns.
static SLOT_LOCK: Mutex<()> = Mutex::new(());

/// One running case: a driver, its backend opened on a fresh directory, and
/// the lock on the process-wide storage slot. Dropping it empties the slot.
pub struct Case {
    pub driver: Driver,
    /// The URL the backend was opened from.
    pub url: String,
    /// The directory the on-disk drivers keep their data in.
    pub data_dir: tempfile::TempDir,
    backend: Arc<dyn StorageBackend>,
    _lock: MutexGuard<'static, ()>,
}

impl Case {
    /// Opens `driver`'s backend. `None` (after saying why on stderr) when the
    /// driver cannot run here.
    pub fn open(driver: Driver) -> Option<Case> {
        let lock = SLOT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let data_dir = tempfile::tempdir().expect("temp dir");
        let Some(url) = driver.url(data_dir.path()) else {
            eprintln!("skipped {}: {}", driver.name(), "driver not compiled in");
            return None;
        };
        let open_url = url.clone();
        // Opened on the storage bridge's runtime, the one the stores use, so
        // the backend's tasks outlive this call.
        let backend = storage::block_on(async move { storage::open(&open_url).await })
            .unwrap_or_else(|error| panic!("open the {} backend: {error}", driver.name()));
        Some(Case {
            driver,
            url,
            data_dir,
            backend,
            _lock: lock,
        })
    }

    /// The opened backend.
    pub fn backend(&self) -> Arc<dyn StorageBackend> {
        Arc::clone(&self.backend)
    }

    /// Makes the backend the process's storage backend.
    pub fn install(&self) {
        storage::install(self.backend());
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        storage::clear();
    }
}

/// A driver's URL for suites that hand it to a spawned `openhuman-core`
/// process instead of opening the backend in the test process.
pub struct UrlCase {
    pub driver: Driver,
    pub url: String,
    pub data_dir: tempfile::TempDir,
}

/// Runs a body that only needs the driver's URL.
pub fn run_url(driver: Driver, body: fn(UrlCase)) {
    // Same lock as the in-process cases: the spawned cores and this process
    // share the process-wide test setup.
    let _lock = SLOT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let data_dir = tempfile::tempdir().expect("temp dir");
    let Some(url) = driver.url(data_dir.path()) else {
        eprintln!("skipped {}: driver not compiled in", driver.name());
        return;
    };
    body(UrlCase {
        driver,
        url,
        data_dir,
    });
}

/// Runs a synchronous case body on `driver`.
pub fn run_sync(driver: Driver, body: fn(Case)) {
    if let Some(case) = Case::open(driver) {
        body(case);
    }
}

/// Runs an asynchronous case body on `driver` inside a multi-thread runtime.
pub fn run_async<F, Fut>(driver: Driver, body: F)
where
    F: FnOnce(Case) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    // Open before entering the runtime: `Case::open` blocks on the storage
    // bridge thread, which is fine here but not inside an async worker.
    if let Some(case) = Case::open(driver) {
        runtime.block_on(body(case));
    }
}

/// Declares one test case per driver for each `body`, in a module named after
/// the driver (`memory`, `sqlite`). SQLite is declared only when its feature
/// is compiled in.
#[allow(unused_macros)]
macro_rules! driver_cases {
    (sync $($body:ident),+ $(,)?) => {
        driver_cases!(@each run_sync $($body),+);
    };
    (async $($body:ident),+ $(,)?) => {
        driver_cases!(@each run_async $($body),+);
    };
    (@each $runner:ident $($body:ident),+) => {
        mod memory {
            $(
                #[test]
                fn $body() {
                    $crate::storage_drivers::$runner(
                        $crate::storage_drivers::Driver::Memory,
                        super::$body,
                    );
                }
            )+
        }
        #[cfg(feature = "storage-sqlite")]
        mod sqlite {
            $(
                #[test]
                fn $body() {
                    $crate::storage_drivers::$runner(
                        $crate::storage_drivers::Driver::Sqlite,
                        super::$body,
                    );
                }
            )+
        }
    };
}
