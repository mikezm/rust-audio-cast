use application::{
    AdjustVolume, Direction, GetStatus, RefreshDevices, SetMuted, SetVolume, StartCasting,
    StopCasting,
};
use axum::extract::{Path, State};
use axum::Json;
use domain::{DeviceId, Volume};

use crate::dto::{DeviceDto, StatusDto, VolumeDto};
use crate::error::ApiError;
use crate::state::AppState;

pub async fn status(State(state): State<AppState>) -> Json<StatusDto> {
    let session = state.session.lock().await;
    Json(GetStatus::execute(&session).into())
}

pub async fn list_casts(State(state): State<AppState>) -> Json<Vec<DeviceDto>> {
    let catalog = state.catalog.lock().await;
    Json(catalog.iter().map(DeviceDto::from).collect())
}

pub async fn refresh_casts(
    State(state): State<AppState>,
) -> Result<Json<Vec<DeviceDto>>, ApiError> {
    let uc = RefreshDevices::new(state.discovery.clone());
    let devices = uc.execute(state.discovery_timeout).await?;

    let dtos = devices.iter().map(DeviceDto::from).collect();
    *state.catalog.lock().await = devices;
    Ok(Json(dtos))
}

pub async fn start_cast(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<StatusDto>, ApiError> {
    let catalog = state.catalog.lock().await.clone();
    let mut session = state.session.lock().await;

    let uc = StartCasting::new(state.cast.clone());
    uc.execute(
        &mut session,
        &catalog,
        &DeviceId::new(uuid),
        &state.stream_url,
        state.format,
    )
    .await?;

    Ok(Json(GetStatus::execute(&session).into()))
}

pub async fn stop_cast(State(state): State<AppState>) -> Result<Json<StatusDto>, ApiError> {
    let mut session = state.session.lock().await;
    let uc = StopCasting::new(state.cast.clone());
    uc.execute(&mut session).await?;
    Ok(Json(GetStatus::execute(&session).into()))
}

pub async fn set_volume(
    State(state): State<AppState>,
    Path(volume): Path<f32>,
) -> Result<Json<VolumeDto>, ApiError> {
    let mut session = state.session.lock().await;
    let uc = SetVolume::new(state.cast.clone());
    let applied = uc.execute(&mut session, Volume::new(volume)).await?;
    Ok(Json(VolumeDto {
        volume: applied.as_f32(),
    }))
}

pub async fn volume_up(State(state): State<AppState>) -> Result<Json<VolumeDto>, ApiError> {
    adjust(state, Direction::Up).await
}

pub async fn volume_down(State(state): State<AppState>) -> Result<Json<VolumeDto>, ApiError> {
    adjust(state, Direction::Down).await
}

async fn adjust(state: AppState, direction: Direction) -> Result<Json<VolumeDto>, ApiError> {
    let mut session = state.session.lock().await;
    let uc = AdjustVolume::new(state.cast.clone());
    let applied = uc.execute(&mut session, direction).await?;
    Ok(Json(VolumeDto {
        volume: applied.as_f32(),
    }))
}

pub async fn volume_mute(State(state): State<AppState>) -> Result<Json<StatusDto>, ApiError> {
    set_muted(state, true).await
}

pub async fn volume_unmute(State(state): State<AppState>) -> Result<Json<StatusDto>, ApiError> {
    set_muted(state, false).await
}

async fn set_muted(state: AppState, muted: bool) -> Result<Json<StatusDto>, ApiError> {
    let mut session = state.session.lock().await;
    let uc = SetMuted::new(state.cast.clone());
    uc.execute(&mut session, muted).await?;
    Ok(Json(GetStatus::execute(&session).into()))
}
