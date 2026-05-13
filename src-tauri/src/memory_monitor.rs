use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::ipc::Channel;

const SENTINEL_NONE: u64 = u64::MAX;

static ASR_FOOTPRINT_MB: AtomicU64 = AtomicU64::new(SENTINEL_NONE);
static LLM_FOOTPRINT_MB: AtomicU64 = AtomicU64::new(SENTINEL_NONE);

static SYS: Mutex<Option<System>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize)]
pub struct MemorySnapshot {
    pub app_rss_mb: u64,
    pub system_total_mb: u64,
    pub system_used_mb: u64,
    pub gpu_used_mb: Option<u64>,
    pub gpu_total_mb: Option<u64>,
    pub asr_mb: Option<u64>,
    pub llm_mb: Option<u64>,
    pub backend: String,
    pub unified_memory: bool,
}

fn pid() -> Pid {
    Pid::from_u32(std::process::id())
}

/// Returns current process RSS in MB.
pub fn current_app_rss_mb() -> u64 {
    let mut guard = match SYS.lock() {
        Ok(g) => g,
        Err(_) => return 0,
    };
    let sys = guard.get_or_insert_with(System::new);
    let pid = pid();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::new().with_memory(),
    );
    sys.process(pid).map(|p| p.memory() / 1024 / 1024).unwrap_or(0)
}

fn system_totals_mb() -> (u64, u64) {
    let mut guard = match SYS.lock() {
        Ok(g) => g,
        Err(_) => return (0, 0),
    };
    let sys = guard.get_or_insert_with(System::new);
    sys.refresh_memory();
    (sys.total_memory() / 1024 / 1024, sys.used_memory() / 1024 / 1024)
}

/// Measure the RSS delta around loading the ASR (whisper) model and store as footprint.
pub fn record_asr_load<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let before = current_app_rss_mb();
    let out = f();
    let after = current_app_rss_mb();
    let delta = after.saturating_sub(before);
    ASR_FOOTPRINT_MB.store(delta, Ordering::Relaxed);
    log::info!(
        "memory_monitor: ASR load delta = {} MB (before {} MB, after {} MB)",
        delta,
        before,
        after
    );
    out
}

/// Measure the RSS delta around loading the LLM model and store as footprint.
pub fn record_llm_load<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let before = current_app_rss_mb();
    let out = f();
    let after = current_app_rss_mb();
    let delta = after.saturating_sub(before);
    LLM_FOOTPRINT_MB.store(delta, Ordering::Relaxed);
    log::info!(
        "memory_monitor: LLM load delta = {} MB (before {} MB, after {} MB)",
        delta,
        before,
        after
    );
    out
}

/// Clear stored ASR footprint (call when model is unloaded).
#[allow(dead_code)]
pub fn clear_asr_footprint() {
    ASR_FOOTPRINT_MB.store(SENTINEL_NONE, Ordering::Relaxed);
}

/// Clear stored LLM footprint (call when model is unloaded).
pub fn clear_llm_footprint() {
    LLM_FOOTPRINT_MB.store(SENTINEL_NONE, Ordering::Relaxed);
}

fn asr_footprint() -> Option<u64> {
    let v = ASR_FOOTPRINT_MB.load(Ordering::Relaxed);
    if v == SENTINEL_NONE {
        None
    } else {
        Some(v)
    }
}

fn llm_footprint() -> Option<u64> {
    let v = LLM_FOOTPRINT_MB.load(Ordering::Relaxed);
    if v == SENTINEL_NONE {
        None
    } else {
        Some(v)
    }
}

/// Returns (used_mb, total_mb) of discrete GPU memory if available.
/// On Apple Silicon (unified memory) returns None — caller signals via `unified_memory`.
fn current_gpu_mem(backend: &str) -> (Option<u64>, Option<u64>) {
    if backend == "CUDA" {
        if let Ok(out) = std::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=memory.used,memory.total",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                if let Some(line) = s.lines().next() {
                    let parts: Vec<&str> = line.split(',').map(|p| p.trim()).collect();
                    if parts.len() == 2 {
                        let used = parts[0].parse::<u64>().ok();
                        let total = parts[1].parse::<u64>().ok();
                        return (used, total);
                    }
                }
            }
        }
    }
    (None, None)
}

fn snapshot(backend: &str) -> MemorySnapshot {
    let app_rss_mb = current_app_rss_mb();
    let (system_total_mb, system_used_mb) = system_totals_mb();
    let (gpu_used_mb, gpu_total_mb) = current_gpu_mem(backend);
    let unified_memory = backend == "Metal";
    MemorySnapshot {
        app_rss_mb,
        system_total_mb,
        system_used_mb,
        gpu_used_mb,
        gpu_total_mb,
        asr_mb: asr_footprint(),
        llm_mb: llm_footprint(),
        backend: backend.to_string(),
        unified_memory,
    }
}

/// Spawn a background task that pushes a MemorySnapshot through `on_update` every second.
/// Stops automatically when `on_update.send()` fails (channel dropped by frontend).
pub fn spawn_monitor(on_update: Channel<MemorySnapshot>, backend: String) {
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tick.tick().await;
            let backend_clone = backend.clone();
            let snap =
                tauri::async_runtime::spawn_blocking(move || snapshot(&backend_clone)).await;
            let Ok(snap) = snap else {
                continue;
            };
            if on_update.send(snap).is_err() {
                log::info!("memory_monitor: channel closed, stopping poller");
                break;
            }
        }
    });
}
