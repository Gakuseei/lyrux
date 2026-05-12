use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent};

#[allow(dead_code)]
pub struct WatcherHandle {
    _inner: Box<dyn std::any::Any + Send>,
}

#[allow(dead_code)]
pub fn spawn(root: PathBuf, sink: mpsc::Sender<Vec<PathBuf>>) -> Option<WatcherHandle> {
    if needs_polling(&root) {
        spawn_poll(root, sink)
    } else {
        spawn_inotify(root, sink)
    }
}

fn spawn_inotify(root: PathBuf, sink: mpsc::Sender<Vec<PathBuf>>) -> Option<WatcherHandle> {
    let (tx, rx) = mpsc::channel::<Result<Vec<DebouncedEvent>, notify::Error>>();
    let mut debouncer = match new_debouncer(Duration::from_millis(100), tx) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("limux: file_panel watcher init failed: {e}");
            return None;
        }
    };
    if let Err(e) = debouncer.watcher().watch(&root, RecursiveMode::Recursive) {
        eprintln!("limux: file_panel watcher.watch failed: {e}");
        return None;
    }
    std::thread::spawn(move || {
        while let Ok(batch) = rx.recv() {
            if let Ok(events) = batch {
                let paths: Vec<PathBuf> = events.into_iter().map(|e| e.path).collect();
                if sink.send(paths).is_err() {
                    break;
                }
            }
        }
    });
    Some(WatcherHandle {
        _inner: Box::new(debouncer),
    })
}

fn spawn_poll(root: PathBuf, sink: mpsc::Sender<Vec<PathBuf>>) -> Option<WatcherHandle> {
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let config = notify::Config::default().with_poll_interval(Duration::from_secs(5));
    let mut watcher = match notify::PollWatcher::new(tx, config) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("limux: file_panel poll watcher init failed: {e}");
            return None;
        }
    };
    if let Err(e) = Watcher::watch(&mut watcher, &root, RecursiveMode::Recursive) {
        eprintln!("limux: file_panel poll watch failed: {e}");
        return None;
    }
    std::thread::spawn(move || {
        while let Ok(ev) = rx.recv() {
            if let Ok(ev) = ev {
                if sink.send(ev.paths).is_err() {
                    break;
                }
            }
        }
    });
    Some(WatcherHandle {
        _inner: Box::new(watcher),
    })
}

#[allow(dead_code)]
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
