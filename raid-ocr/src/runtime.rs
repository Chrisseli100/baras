//! Process-wide knobs for the inference runtime: how many threads it may use,
//! and giving its memory back once a pass is over.

/// Most threads rten may use. Left alone it takes every physical core, which
/// on a 24-core desktop is 24 threads churning tensors for a 2.5 MB model, and
/// each thread's allocator arena then keeps its share of the peak. Eight stays
/// within about 50 ms of the unbounded pass.
const MAX_RTEN_THREADS: usize = 8;

/// Cap the rten thread pool.
///
/// rten reads `RTEN_NUM_THREADS` once, when its pool is first built, and ocrs
/// exposes no other way to size it. A value already in the environment wins.
///
/// # Safety
///
/// Must run before any other thread starts: setting an environment variable
/// races with readers on other threads.
pub unsafe fn configure_threads() {
    const VAR: &str = "RTEN_NUM_THREADS";
    if std::env::var_os(VAR).is_some() {
        return;
    }
    let threads = num_cpus::get_physical().clamp(1, MAX_RTEN_THREADS);
    // SAFETY: the caller promises no other thread is running yet.
    unsafe { std::env::set_var(VAR, threads.to_string()) };
}

/// Hand freed heap pages back to the kernel after a detection pass.
///
/// A pass allocates and frees a few hundred megabytes of activation tensors
/// across every rten thread. glibc keeps each thread's freed memory in its own
/// arena rather than returning it, so without this the process settles several
/// hundred megabytes above where it started. Windows and macOS release large
/// freed blocks on their own, and musl has no such call.
pub fn release_memory() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // SAFETY: malloc_trim only walks the allocator's own free lists.
    unsafe {
        libc::malloc_trim(0);
    }
}
