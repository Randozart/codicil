use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug, Clone)]
pub enum FileEvent {
    Changed(PathBuf),
    Created(PathBuf),
    Deleted(PathBuf),
}

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    rx: mpsc::Receiver<Result<Event, notify::Error>>,
}

impl FileWatcher {
    pub fn new(paths: &[&Path]) -> Result<Self, notify::Error> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_millis(200)),
        )?;

        for path in paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)?;
            }
        }

        Ok(Self { watcher, rx })
    }

    pub fn watch(&mut self, path: &Path) -> Result<(), notify::Error> {
        self.watcher.watch(path, RecursiveMode::Recursive)
    }

    pub fn poll(&self) -> Vec<FileEvent> {
        let mut events = Vec::new();

        while let Ok(result) = self.rx.try_recv() {
            if let Ok(event) = result {
                for path in event.paths {
                    let file_event = match event.kind {
                        notify::EventKind::Create(_) => FileEvent::Created(path),
                        notify::EventKind::Modify(_) => FileEvent::Changed(path),
                        notify::EventKind::Remove(_) => FileEvent::Deleted(path),
                        _ => continue,
                    };
                    events.push(file_event);
                }
            }
        }

        events
    }
}

pub fn watch_paths(project_root: &Path) -> Result<FileWatcher, notify::Error> {
    let paths = vec![
        project_root.join("routes"),
        project_root.join("lib"),
        project_root.join("middleware"),
        project_root.join("components"),
    ];

    let mut watcher = FileWatcher::new(&[])?;

    for path in &paths {
        if path.exists() {
            watcher.watch(path)?;
        }
    }

    Ok(watcher)
}

pub fn is_relevant_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_str().unwrap_or("");
        matches!(ext, "bv" | "rbv" | "toml")
    } else {
        false
    }
}
