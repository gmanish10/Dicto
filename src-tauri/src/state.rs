use crate::{config::Settings, history::HistoryStore, hotkey::HotkeyEvent};
use crossbeam_channel::{Receiver, Sender};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// What the pipeline is currently doing — drives menubar UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineState {
    Idle,
    Recording,
    Transcribing,
    UpdateAvailable,
}

pub struct AppState {
    pub app_data_dir: PathBuf,
    pub config: RwLock<Settings>,
    pub pipeline_state: RwLock<PipelineState>,
    pub history: HistoryStore,
    /// rdev producer side (set by hotkey::listener when it starts).
    pub hotkey_tx: Sender<HotkeyEvent>,
    pub hotkey_rx: Receiver<HotkeyEvent>,
}

impl AppState {
    pub fn initialize(app: &AppHandle) -> anyhow::Result<Self> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| anyhow::anyhow!("missing app data dir: {e}"))?;
        std::fs::create_dir_all(&app_data_dir)?;

        let settings = crate::config::load(&app_data_dir);
        let history = HistoryStore::open(&app_data_dir.join("dicto.db"))?;
        let (tx, rx) = crossbeam_channel::unbounded();

        Ok(Self {
            app_data_dir,
            config: RwLock::new(settings),
            pipeline_state: RwLock::new(PipelineState::Idle),
            history,
            hotkey_tx: tx,
            hotkey_rx: rx,
        })
    }

    pub fn set_pipeline_state(&self, new: PipelineState) {
        *self.pipeline_state.write() = new;
    }

    pub fn save_settings(&self) -> anyhow::Result<()> {
        let settings = self.config.read().clone();
        crate::config::save(&self.app_data_dir, &settings)
    }
}

pub type SharedState = Arc<AppState>;
