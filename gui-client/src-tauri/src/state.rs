use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub is_syncing: bool,
    pub sync_mode: Option<String>,
    pub last_sync_time: Option<DateTime<Utc>>,
    pub synced_count: usize,
    pub failed_count: usize,
    pub progress: f64,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            is_syncing: false,
            sync_mode: None,
            last_sync_time: None,
            synced_count: 0,
            failed_count: 0,
            progress: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.is_syncing = false;
        self.sync_mode = None;
        self.progress = 0.0;
    }

    pub fn update_progress(&mut self, progress: f64) {
        self.progress = progress.min(100.0).max(0.0);
    }

    pub fn increment_synced(&mut self) {
        self.synced_count += 1;
    }

    pub fn increment_failed(&mut self) {
        self.failed_count += 1;
    }

    /// 加载持久化的状态
    pub fn load_persistent(&mut self, last_sync_time: Option<DateTime<Utc>>, synced_count: usize) {
        self.last_sync_time = last_sync_time;
        self.synced_count = synced_count;
    }
}

impl Default for SyncState {
    fn default() -> Self {
        Self::new()
    }
}
