//! Parallel download workers for translation updates: per-file incremental
//! downloads and bulk ZIP download + extraction.

use std::{
    cmp::max,
    fs,
    io::{Cursor, Read, Write},
    path::Path,
    sync::{
        atomic::{self, AtomicBool, AtomicUsize},
        mpsc, Arc, Mutex,
    },
    thread,
};

use fnv::FnvHashMap;
use once_cell::sync::Lazy;
use thread_priority::{ThreadBuilderExt, ThreadPriority};

use crate::core::{
    http::{self, ureq_config},
    utils, Error,
};

use super::repo::RepoFile;
use super::updater::{UpdateInfo, UpdateProgress, Updater};

const CHUNK_SIZE: usize = 8192; // 8KiB
static NUM_THREADS: Lazy<usize> = Lazy::new(|| {
    let parallelism = thread::available_parallelism().expect("unexpected failure").get();
    max(1, parallelism / 2)
});
const MIN_CHUNK_SIZE: u64 = 1024 * 1024 * 5;

struct DownloadJob {
    agent: ureq::Agent,
    hasher: blake3::Hasher,
    buffer: Vec<u8>,
}

impl DownloadJob {
    fn new(agent1: ureq::Agent) -> DownloadJob {
        DownloadJob {
            agent: agent1,
            hasher: blake3::Hasher::new(),
            buffer: vec![0u8; CHUNK_SIZE],
        }
    }
}

impl Updater {
    pub(super) fn download_incremental(
        self: Arc<Self>,
        update_info: &UpdateInfo,
        localized_data_dir: &Path,
        cached_files: Arc<Mutex<FnvHashMap<String, String>>>,
    ) -> Result<usize, Error> {
        let total_size = update_info.size;
        let current_bytes = Arc::new(AtomicUsize::new(0));
        let non_fatal_error_count = Arc::new(AtomicUsize::new(0));
        let fatal_error = Arc::new(Mutex::new(None::<Error>));
        let stop_signal = Arc::new(AtomicBool::new(false));

        let shared_agent: ureq::Agent = ureq::Agent::new_with_config(ureq_config());

        let (sender, receiver) = mpsc::channel::<RepoFile>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut handles = Vec::with_capacity(*NUM_THREADS);
        for _ in 0..*NUM_THREADS {
            let updater = self.clone();
            let localized_data_dir_clone = localized_data_dir.to_path_buf();
            let base_url_clone = update_info.base_url.clone();
            let cached_files_clone = Arc::clone(&cached_files);
            let current_bytes_clone = Arc::clone(&current_bytes);
            let non_fatal_error_count_clone = Arc::clone(&non_fatal_error_count);
            let fatal_error_clone = Arc::clone(&fatal_error);
            let stop_signal_clone = Arc::clone(&stop_signal);
            let receiver_clone = Arc::clone(&receiver);

            let thread_agent = shared_agent.clone();

            let handle = thread::Builder::new()
                .name("incremental_downloader".into())
                .stack_size(8 * 1024 * 1024)
                .spawn_with_priority(ThreadPriority::Min, move |result| {
                    if result.is_err() {
                        warn!("Failed to set background thread priority for incremental downloader.");
                    }
                    let mut job = DownloadJob::new(thread_agent);

                    while let Ok(repo_file) = receiver_clone.lock().expect("lock poisoned").recv() {
                        if stop_signal_clone.load(atomic::Ordering::Relaxed) {
                            break;
                        }

                        let file_path = repo_file.get_fs_path(&localized_data_dir_clone);
                        let url = utils::concat_unix_path(&base_url_clone, &repo_file.path);

                        let execute_result = (|| -> Result<String, Error> {
                            if let Some(parent) = Path::new(&file_path).parent() {
                                Self::create_dir(parent, false)?;
                            }
                            let mut file = fs::File::create(&file_path)?;
                            let res = job.agent.get(&url).call()?;

                            http::download_file_buffered(res, &mut file, &mut job.buffer, |bytes| {
                                job.hasher.update(bytes);
                                let prev_size = current_bytes_clone.fetch_add(bytes.len(), atomic::Ordering::SeqCst);
                                updater
                                    .progress
                                    .store(Arc::new(Some(UpdateProgress::new(prev_size + bytes.len(), total_size))));
                            })?;

                            let hash = job.hasher.finalize().to_hex().to_string();
                            if hash != repo_file.hash {
                                return Err(Error::FileHashMismatch(file_path.to_str().unwrap_or("").to_string()));
                            }
                            job.hasher.reset();
                            Ok(hash)
                        })();

                        match execute_result {
                            Ok(hash) => {
                                cached_files_clone
                                    .lock()
                                    .expect("lock poisoned")
                                    .insert(repo_file.path.clone(), hash);
                            }
                            Err(e) => {
                                if matches!(e, Error::OutOfDiskSpace | Error::FileHashMismatch(_)) {
                                    error!("Fatal error during incremental download: {}", e);
                                    *fatal_error_clone.lock().expect("lock poisoned") = Some(e);
                                    stop_signal_clone.store(true, atomic::Ordering::Relaxed);
                                    return;
                                } else {
                                    error!("Non-fatal error during incremental download: {}", e);
                                    non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::SeqCst);
                                }
                            }
                        }
                    }
                })
                .expect("unexpected failure");
            handles.push(handle);
        }

        for repo_file in update_info.files.iter() {
            if sender.send(repo_file.clone()).is_err() {
                break;
            }
        }
        drop(sender);

        for handle in handles {
            handle.join().expect("thread join failed");
        }

        if let Some(err) = fatal_error.lock().expect("lock poisoned").take() {
            return Err(err);
        }

        Ok(non_fatal_error_count.load(atomic::Ordering::Relaxed))
    }

    pub(super) fn download_zip(
        self: Arc<Self>,
        update_info: &UpdateInfo,
        localized_data_dir: &Path,
        cached_files: Arc<Mutex<FnvHashMap<String, String>>>,
    ) -> Result<usize, Error> {
        let zip_path = localized_data_dir.join(".tmp.zip");
        // idk compiler going monkey mode unless i add this
        #[allow(unused_assignments)]
        let mut error_count = 0;

        {
            let total_size_header = ureq::agent().head(&update_info.zip_url).call().ok().and_then(|res| {
                res.headers()
                    .get("Content-Length")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<usize>().ok())
            });

            let progress_total = match total_size_header {
                Some(size) if size > 0 => {
                    debug!("Using Content-Length from header for progress bar: {}", size);
                    size
                }
                _ => {
                    debug!(
                        "Server did not provide a valid Content-Length. Using fallback size from index: {}",
                        update_info.size
                    );
                    update_info.size
                }
            };

            let downloaded = Arc::new(AtomicUsize::new(0));
            let self_clone = self.clone();
            let downloaded_clone = downloaded;

            let progress_bar = Arc::new(move |bytes_read: usize| {
                let prev_size = downloaded_clone.fetch_add(bytes_read, atomic::Ordering::Relaxed);
                let current = prev_size + bytes_read;
                self_clone
                    .progress
                    .store(Arc::new(Some(UpdateProgress::new(current, progress_total))));
            });

            http::download_file_parallel(
                &update_info.zip_url,
                &zip_path,
                *NUM_THREADS,
                MIN_CHUNK_SIZE,
                CHUNK_SIZE,
                progress_bar,
            )?;

            let files_to_extract = Arc::new(
                update_info
                    .files
                    .iter()
                    .map(|f| (utils::concat_unix_path(&update_info.zip_dir, &f.path), f.clone()))
                    .collect::<FnvHashMap<_, _>>(),
            );

            let zip_file = fs::File::open(&zip_path)?;
            // SAFETY: FFI / raw pointer operation required by IL2CPP interop
            let mmap = Arc::new(unsafe { memmap2::Mmap::map(&zip_file)? });

            let total_size = update_info.size;
            let current_bytes = Arc::new(AtomicUsize::new(0));
            let non_fatal_error_count = Arc::new(AtomicUsize::new(0));
            let fatal_error = Arc::new(Mutex::new(None::<Error>));
            let stop_signal = Arc::new(AtomicBool::new(false));

            let (sender, receiver) = mpsc::channel::<usize>();
            let receiver = Arc::new(Mutex::new(receiver));
            let mut handles = Vec::with_capacity(*NUM_THREADS);

            for _ in 0..*NUM_THREADS {
                let updater = self.clone();
                let mmap_thread = Arc::clone(&mmap);
                let files_to_extract_clone = Arc::clone(&files_to_extract);
                let localized_data_dir_clone = localized_data_dir.to_path_buf();
                let cached_files_clone = Arc::clone(&cached_files);
                let current_bytes_clone = Arc::clone(&current_bytes);
                let non_fatal_error_count_clone = Arc::clone(&non_fatal_error_count);
                let fatal_error_clone = Arc::clone(&fatal_error);
                let stop_signal_clone = Arc::clone(&stop_signal);
                let receiver_clone = Arc::clone(&receiver);

                let handle = thread::Builder::new()
                    .name("zip_extractor".into())
                    .stack_size(8 * 1024 * 1024)
                    .spawn_with_priority(ThreadPriority::Min, move |result| {
                        if result.is_err() {
                            warn!("Failed to set background thread priority for zip extractor.");
                        }

                        let Ok(mut archive) = zip::ZipArchive::new(Cursor::new(&mmap_thread[..])) else {
                            return;
                        };

                        let mut buffer = vec![0u8; CHUNK_SIZE];
                        let mut hasher = blake3::Hasher::new();

                        while let Ok(i) = receiver_clone.lock().expect("lock poisoned").recv() {
                            if stop_signal_clone.load(atomic::Ordering::Relaxed) {
                                break;
                            }

                            let Ok(mut zip_entry) = archive.by_index(i) else {
                                non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::SeqCst);
                                continue;
                            };

                            let Some(repo_file) = files_to_extract_clone.get(zip_entry.name()) else {
                                continue;
                            };
                            let repo_file = repo_file.clone();

                            let path = repo_file.get_fs_path(&localized_data_dir_clone);
                            if let Some(parent) = path.parent() {
                                if Self::create_dir(parent, false).is_err() {
                                    non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::SeqCst);
                                    continue;
                                }
                            }

                            let Ok(mut out_file) = fs::File::create(&path) else {
                                non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::SeqCst);
                                continue;
                            };

                            loop {
                                match zip_entry.read(&mut buffer) {
                                    Ok(0) => break,
                                    Ok(read_bytes) => {
                                        let data_slice = &buffer[..read_bytes];
                                        if out_file.write_all(data_slice).is_err() {
                                            *fatal_error_clone.lock().expect("lock poisoned") =
                                                Some(Error::OutOfDiskSpace);
                                            stop_signal_clone.store(true, atomic::Ordering::Relaxed);
                                            return;
                                        }
                                        hasher.update(data_slice);
                                        let prev_size =
                                            current_bytes_clone.fetch_add(read_bytes, atomic::Ordering::SeqCst);
                                        updater.progress.store(Arc::new(Some(UpdateProgress::new(
                                            prev_size + read_bytes,
                                            total_size,
                                        ))));
                                    }
                                    Err(_) => {
                                        non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::SeqCst);
                                        break;
                                    }
                                }
                            }

                            let hash = hasher.finalize().to_hex().to_string();
                            if hash != repo_file.hash {
                                let path_str = path.to_str().unwrap_or("").to_string();
                                *fatal_error_clone.lock().expect("lock poisoned") =
                                    Some(Error::FileHashMismatch(path_str));
                                stop_signal_clone.store(true, atomic::Ordering::Relaxed);
                                return;
                            }

                            cached_files_clone
                                .lock()
                                .expect("lock poisoned")
                                .insert(repo_file.path.clone(), hash);
                            hasher.reset();
                        }
                    })
                    .expect("unexpected failure");
                handles.push(handle);
            }

            let zip_len = zip::ZipArchive::new(Cursor::new(&mmap[..]))?.len();
            for i in 0..zip_len {
                if sender.send(i).is_err() {
                    break;
                }
            }
            drop(sender);

            for handle in handles {
                handle.join().expect("thread join failed");
            }

            if let Some(err) = fatal_error.lock().expect("lock poisoned").take() {
                return Err(err);
            }
            error_count = non_fatal_error_count.load(atomic::Ordering::Relaxed);
        }

        if let Err(e) = fs::remove_file(&zip_path) {
            error!("Failed to remove temporary file '{}': {}", zip_path.display(), e);
            error_count += 1;
        }

        Ok(error_count)
    }
}
