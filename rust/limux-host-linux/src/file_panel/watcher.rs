use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent};

const EXCLUDED_COMPONENTS: &[&str] = &[
    // Build outputs
    "target",
    "node_modules",
    ".next",
    "dist",
    "build",
    "out",
    // VCS
    ".git",
    ".hg",
    ".svn",
    // OS / Browser caches
    ".cache",
    ".thumbnails",
    ".mozilla",
    ".var",
    // Language tool caches
    ".cargo",
    ".rustup",
    ".npm",
    ".yarn",
    ".pnpm-store",
    ".gradle",
    ".m2",
    ".ivy2",
    ".sbt",
    ".conda",
    // Python
    "__pycache__",
    ".venv",
    "venv",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".eggs",
    // IDE / Tool state
    ".idea",
    ".vscode",
    ".vs",
    // Agent / scratch dirs commonly created by dev tools
    ".superpowers",
    ".todos",
    ".playwright",
    ".claude",
];

fn is_excluded(path: &Path) -> bool {
    path.components()
        .any(|c| EXCLUDED_COMPONENTS.iter().any(|e| c.as_os_str() == *e))
}

pub struct WatcherHandle {
    _inner: Box<dyn std::any::Any + Send>,
}

pub fn spawn(root: PathBuf, sink: mpsc::Sender<Vec<PathBuf>>) -> Option<WatcherHandle> {
    if needs_polling(&root) {
        spawn_poll(root, sink)
    } else {
        spawn_inotify(root, sink)
    }
}

fn spawn_inotify(root: PathBuf, sink: mpsc::Sender<Vec<PathBuf>>) -> Option<WatcherHandle> {
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<Result<Vec<DebouncedEvent>, notify::Error>>();
        let mut debouncer = match new_debouncer(Duration::from_millis(100), tx) {
            Ok(d) => d,
            Err(_) => {
                run_poll_watcher(root, sink);
                return;
            }
        };
        let mut watch_failed = false;
        if debouncer
            .watcher()
            .watch(&root, RecursiveMode::NonRecursive)
            .is_err()
        {
            watch_failed = true;
        }
        if !watch_failed {
            if let Ok(rd) = std::fs::read_dir(&root) {
                for entry in rd.flatten() {
                    let path = entry.path();
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    if !is_dir {
                        continue;
                    }
                    let name = match path.file_name().and_then(|s| s.to_str()) {
                        Some(n) => n,
                        None => continue,
                    };
                    if EXCLUDED_COMPONENTS.contains(&name) {
                        continue;
                    }
                    let _ = debouncer.watcher().watch(&path, RecursiveMode::Recursive);
                }
            }
        }
        if watch_failed {
            run_poll_watcher(root, sink);
            return;
        }
        while let Ok(batch) = rx.recv() {
            if let Ok(events) = batch {
                let paths: Vec<PathBuf> = events
                    .into_iter()
                    .map(|e| e.path)
                    .filter(|p| !is_excluded(p))
                    .collect();
                if paths.is_empty() {
                    continue;
                }
                if sink.send(paths).is_err() {
                    break;
                }
            }
        }
    });
    Some(WatcherHandle {
        _inner: Box::new(()),
    })
}

fn spawn_poll(root: PathBuf, sink: mpsc::Sender<Vec<PathBuf>>) -> Option<WatcherHandle> {
    std::thread::spawn(move || {
        run_poll_watcher(root, sink);
    });
    Some(WatcherHandle {
        _inner: Box::new(()),
    })
}

fn run_poll_watcher(root: PathBuf, sink: mpsc::Sender<Vec<PathBuf>>) {
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let config = notify::Config::default().with_poll_interval(Duration::from_secs(5));
    let mut watcher = match notify::PollWatcher::new(tx, config) {
        Ok(w) => w,
        Err(_) => return,
    };
    if Watcher::watch(&mut watcher, &root, RecursiveMode::NonRecursive).is_err() {
        return;
    }
    if let Ok(rd) = std::fs::read_dir(&root) {
        for entry in rd.flatten() {
            let path = entry.path();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if !is_dir {
                continue;
            }
            let name = match path.file_name().and_then(|s| s.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if EXCLUDED_COMPONENTS.contains(&name) {
                continue;
            }
            let _ = Watcher::watch(&mut watcher, &path, RecursiveMode::Recursive);
        }
    }
    while let Ok(ev) = rx.recv() {
        if let Ok(ev) = ev {
            let paths: Vec<PathBuf> = ev.paths.into_iter().filter(|p| !is_excluded(p)).collect();
            if paths.is_empty() {
                continue;
            }
            if sink.send(paths).is_err() {
                break;
            }
        }
    }
}

pub fn needs_polling(root: &Path) -> bool {
    use nix::sys::statfs::statfs;
    match statfs(root) {
        Ok(s) => {
            let magic = s.filesystem_type();
            const NFS: i64 = 0x6969;
            const SMB: i64 = 0x517B;
            const CIFS: i64 = 0xFF534D42_u32 as i64;
            const FUSE: i64 = 0x65735546;
            #[allow(clippy::unnecessary_cast)]
            let m = magic.0 as i64;
            m == NFS || m == SMB || m == CIFS || m == FUSE
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn module_loads() {
        let _ = ();
    }
}
