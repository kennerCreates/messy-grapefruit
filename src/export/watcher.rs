//! File watcher for auto-export on save.
//!
//! Watches `.sprite` files for changes and triggers re-export of only
//! the changed sprite using last-used export settings.

use notify::{Event, EventKind, RecommendedWatcher, Watcher, RecursiveMode};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// A file change event that the main loop should process.
#[derive(Debug, Clone)]
pub struct SpriteChangeEvent {
    /// Path to the changed `.sprite` file.
    pub path: PathBuf,
}

/// File watcher state, held by the application.
pub struct FileWatcher {
    /// The underlying watcher (kept alive so it continues watching).
    _watcher: RecommendedWatcher,
    /// Receiver for change events.
    receiver: mpsc::Receiver<SpriteChangeEvent>,
    /// Whether the watcher is active.
    active: bool,
}

/// Pending export requests accumulated from the watcher.
/// Shared between the watcher callback and the main loop.
pub struct WatcherState {
    /// Pending sprite file paths that need re-export.
    pub pending_exports: Arc<Mutex<Vec<PathBuf>>>,
    /// Watcher instance (None if not started).
    watcher: Option<FileWatcher>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self {
            pending_exports: Arc::new(Mutex::new(Vec::new())),
            watcher: None,
        }
    }

    /// Start watching a directory for `.sprite` file changes.
    pub fn start_watching(&mut self, watch_dir: &Path) -> Result<(), String> {
        if self.watcher.is_some() {
            self.stop_watching();
        }

        let (tx, rx) = mpsc::channel();
        let pending = Arc::clone(&self.pending_exports);

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    // Only react to modification/creation events
                    let dominated = matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_)
                    );
                    if !dominated {
                        return;
                    }

                    for path in &event.paths {
                        if path.extension().and_then(|e| e.to_str()) == Some("sprite") {
                            let change = SpriteChangeEvent {
                                path: path.clone(),
                            };
                            // Send via channel for the main loop
                            let _ = tx.send(change.clone());
                            // Also add to pending exports
                            if let Ok(mut pending) = pending.lock()
                                && !pending.iter().any(|p| p == path) {
                                    pending.push(path.clone());
                                }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("File watcher error: {}", e);
                }
            }
        })
        .map_err(|e| format!("Failed to create file watcher: {}", e))?;

        watcher
            .watch(watch_dir, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to start watching: {}", e))?;

        self.watcher = Some(FileWatcher {
            _watcher: watcher,
            receiver: rx,
            active: true,
        });

        Ok(())
    }

    /// Stop watching.
    pub fn stop_watching(&mut self) {
        self.watcher = None;
    }

    /// Check if the watcher is active.
    pub fn is_watching(&self) -> bool {
        self.watcher.as_ref().map(|w| w.active).unwrap_or(false)
    }

    /// Drain pending export events (non-blocking).
    /// Returns paths to `.sprite` files that have changed.
    pub fn drain_pending(&self) -> Vec<PathBuf> {
        // Drain from channel first
        if let Some(ref watcher) = self.watcher {
            while let Ok(event) = watcher.receiver.try_recv() {
                if let Ok(mut pending) = self.pending_exports.lock()
                    && !pending.iter().any(|p| p == &event.path) {
                        pending.push(event.path);
                    }
            }
        }

        // Take all pending paths
        if let Ok(mut pending) = self.pending_exports.lock() {
            std::mem::take(&mut *pending)
        } else {
            Vec::new()
        }
    }
}

impl Default for WatcherState {
    fn default() -> Self {
        Self::new()
    }
}
