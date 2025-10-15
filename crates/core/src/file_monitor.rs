use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use notify::event::{EventKind, ModifyKind, RemoveKind};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;

/// 監控檔案變更時可能回傳的錯誤。 / Error type for file monitoring operations.
#[derive(Debug, Error)]
pub enum FileMonitorError {
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("monitor channel disconnected")]
    ChannelDisconnected,
}

/// 監控到的事件種類。 / Classifies observed file system changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileMonitorEventKind {
    Modified,
    Removed,
    Created,
    Renamed { from: PathBuf, to: PathBuf },
    Other,
}

/// 檔案事件的詳細資料。 / File event payload with resolved path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEvent {
    pub path: PathBuf,
    pub kind: FileMonitorEventKind,
}

/// 封裝 `notify` 監視器以提供跨平台介面。 / Thin wrapper around `notify` for cross-platform monitoring.
pub struct FileMonitor {
    watcher: RecommendedWatcher,
    rx: Receiver<FileEvent>,
}

impl FileMonitor {
    /// 建立新的監視器實例。 / Creates a new monitor instance.
    pub fn new() -> Result<Self, FileMonitorError> {
        let (tx, rx) = mpsc::channel();
        let watcher = RecommendedWatcher::new(
            move |res| {
                if let Ok(event) = res {
                    if let Some(mapped) = map_event(event) {
                        let _ = tx.send(mapped);
                    }
                }
            },
            Config::default(),
        )?;

        Ok(Self { watcher, rx })
    }

    /// 開始監看指定路徑。 / Starts watching the provided path.
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<(), FileMonitorError> {
        self.watcher
            .watch(path.as_ref(), RecursiveMode::NonRecursive)
            .map_err(FileMonitorError::from)
    }

    /// 停止監看指定路徑。 / Stops watching the provided path.
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<(), FileMonitorError> {
        self.watcher
            .unwatch(path.as_ref())
            .map_err(FileMonitorError::from)
    }

    /// 嘗試取得下一個事件（非阻塞）。 / Attempts to fetch the next event without blocking.
    pub fn try_next(&self) -> Option<FileEvent> {
        self.rx.try_recv().ok()
    }

    /// 阻塞直到收到事件。 / Blocks until the next event arrives.
    pub fn recv(&self) -> Result<FileEvent, FileMonitorError> {
        self.rx
            .recv()
            .map_err(|_| FileMonitorError::ChannelDisconnected)
    }

    /// 在期限內等待事件，逾時回傳 `None`。 / Waits for an event until the timeout, returning `None` on timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Option<FileEvent>, FileMonitorError> {
        match self.rx.recv_timeout(timeout) {
            Ok(event) => Ok(Some(event)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(FileMonitorError::ChannelDisconnected),
        }
    }
}

fn map_event(event: notify::Event) -> Option<FileEvent> {
    if event.paths.is_empty() {
        return None;
    }

    let primary = event.paths[0].clone();
    let kind = match event.kind {
        EventKind::Modify(ModifyKind::Name(_)) if event.paths.len() >= 2 => {
            let to = event.paths[1].clone();
            FileMonitorEventKind::Renamed {
                from: primary.clone(),
                to,
            }
        }
        EventKind::Modify(ModifyKind::Data(_)) | EventKind::Modify(ModifyKind::Metadata(_)) => {
            FileMonitorEventKind::Modified
        }
        EventKind::Modify(ModifyKind::Any) => FileMonitorEventKind::Modified,
        EventKind::Create(_) => FileMonitorEventKind::Created,
        EventKind::Remove(RemoveKind::File) | EventKind::Remove(RemoveKind::Any) => {
            FileMonitorEventKind::Removed
        }
        _ => FileMonitorEventKind::Other,
    };

    let mut path = primary;
    if let FileMonitorEventKind::Renamed { to, .. } = &kind {
        path = to.clone();
    }

    Some(FileEvent { path, kind })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use tempfile::tempdir;

    #[test]
    fn detect_file_modification() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("watch.txt");
        fs::write(&file_path, "initial").unwrap();

        let mut monitor = FileMonitor::new().unwrap();
        monitor.watch(&file_path).unwrap();

        // 等待 watcher 啟動。 / Allow watcher to settle.
        thread::sleep(Duration::from_millis(100));

        fs::write(&file_path, "updated").unwrap();
        let event = monitor
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .expect("expected an event");

        assert_eq!(event.path, file_path);
        assert!(matches!(
            event.kind,
            FileMonitorEventKind::Modified | FileMonitorEventKind::Other
        ));
    }
}
