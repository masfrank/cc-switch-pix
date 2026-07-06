//! PxPipe bridge Tauri commands.

use crate::services::pxpipe::{PxpipeConfig, PxpipeStatus};
use crate::store::AppState;

#[tauri::command]
pub async fn get_pxpipe_config(state: tauri::State<'_, AppState>) -> Result<PxpipeConfig, String> {
    state.pxpipe_service.get_config()
}

#[tauri::command]
pub fn update_pxpipe_config(
    state: tauri::State<'_, AppState>,
    config: PxpipeConfig,
) -> Result<(), String> {
    state.pxpipe_service.update_config(config)
}

#[tauri::command]
pub async fn get_pxpipe_status(state: tauri::State<'_, AppState>) -> Result<PxpipeStatus, String> {
    state.pxpipe_service.get_status().await
}

#[tauri::command]
pub async fn start_pxpipe_bridge(
    state: tauri::State<'_, AppState>,
) -> Result<PxpipeStatus, String> {
    state.pxpipe_service.start().await
}

#[tauri::command]
pub async fn stop_pxpipe_bridge(state: tauri::State<'_, AppState>) -> Result<PxpipeStatus, String> {
    state.pxpipe_service.stop().await
}
