//! Use-case tests exercised entirely against mock ports — no ALSA device,
//! Chromecast, or sockets. This is the hermetic control-plane coverage the
//! hexagonal design buys us; it runs on any machine, including CI.

use std::sync::Arc;
use std::time::Duration;

use application::{
    AdjustVolume, Direction, GetStatus, RefreshDevices, SetMuted, SetVolume, StartCasting,
    StopCasting,
};
use domain::{
    AudioFormat, CastSession, CastState, DeviceId, DiscoveredDevice, DomainError,
    MockCastController, MockDeviceDiscovery, StreamUrl, Volume,
};

fn device(id: &str) -> DiscoveredDevice {
    DiscoveredDevice {
        id: DeviceId::new(id),
        name: format!("Speaker {id}"),
        ip: "192.168.1.10".to_string(),
        port: 8009,
        model: None,
    }
}

fn stream_url() -> StreamUrl {
    StreamUrl::new("http://192.168.1.5:80/audio-stream")
}

#[tokio::test]
async fn set_volume_clamps_above_max() {
    let mut cast = MockCastController::new();
    // The clamped value (1.0), not 1.5, must reach the device.
    cast.expect_set_volume()
        .withf(|v: &Volume| v.as_f32() == 1.0)
        .times(1)
        .returning(|_| Ok(()));

    let uc = SetVolume::new(Arc::new(cast));
    let mut session = CastSession::new();

    let result = uc.execute(&mut session, Volume::new(1.5)).await.unwrap();
    assert_eq!(result, Volume::new(1.0));
    assert_eq!(session.volume(), Volume::new(1.0));
}

#[tokio::test]
async fn mute_then_unmute_restores_prior_volume() {
    let mut cast = MockCastController::new();
    cast.expect_set_volume().returning(|_| Ok(()));
    cast.expect_set_muted().returning(|_| Ok(()));
    let cast: Arc<dyn domain::CastController> = Arc::new(cast);

    let set_volume = SetVolume::new(cast.clone());
    let set_muted = SetMuted::new(cast.clone());
    let mut session = CastSession::new();

    set_volume
        .execute(&mut session, Volume::new(0.6))
        .await
        .unwrap();

    set_muted.execute(&mut session, true).await.unwrap();
    assert!(session.is_muted());
    assert_eq!(session.effective_volume(), Volume::new(0.0));

    set_muted.execute(&mut session, false).await.unwrap();
    assert!(!session.is_muted());
    assert_eq!(session.effective_volume(), Volume::new(0.6));
}

#[tokio::test]
async fn setting_volume_clears_mute() {
    let mut cast = MockCastController::new();
    cast.expect_set_muted().returning(|_| Ok(()));
    cast.expect_set_volume().returning(|_| Ok(()));
    let cast: Arc<dyn domain::CastController> = Arc::new(cast);

    let set_muted = SetMuted::new(cast.clone());
    let set_volume = SetVolume::new(cast.clone());
    let mut session = CastSession::new();

    set_muted.execute(&mut session, true).await.unwrap();
    assert!(session.is_muted());

    set_volume
        .execute(&mut session, Volume::new(0.3))
        .await
        .unwrap();
    assert!(!session.is_muted());
    assert_eq!(session.volume(), Volume::new(0.3));
}

#[tokio::test]
async fn start_casting_unknown_device_errors_without_calling_cast() {
    // No expectations: if connect/play were called, mockall would panic.
    let cast = MockCastController::new();
    let uc = StartCasting::new(Arc::new(cast));
    let mut session = CastSession::new();
    let catalog = vec![device("known")];

    let err = uc
        .execute(
            &mut session,
            &catalog,
            &DeviceId::new("unknown"),
            &stream_url(),
            AudioFormat::CD,
        )
        .await
        .unwrap_err();

    assert_eq!(err, DomainError::DeviceNotFound(DeviceId::new("unknown")));
    assert_eq!(session.state(), CastState::Inactive);
}

#[tokio::test]
async fn start_casting_success_connects_plays_and_marks_playing() {
    let mut cast = MockCastController::new();
    cast.expect_connect().times(1).returning(|_| Ok(()));
    cast.expect_play().times(1).returning(|_, _| Ok(()));

    let uc = StartCasting::new(Arc::new(cast));
    let mut session = CastSession::new();
    let catalog = vec![device("known")];

    uc.execute(
        &mut session,
        &catalog,
        &DeviceId::new("known"),
        &stream_url(),
        AudioFormat::CD,
    )
    .await
    .unwrap();

    assert_eq!(session.state(), CastState::Playing);
    assert_eq!(session.device().unwrap().id, DeviceId::new("known"));
}

#[tokio::test]
async fn stop_casting_marks_inactive() {
    let mut cast = MockCastController::new();
    cast.expect_stop().times(1).returning(|| Ok(()));

    let uc = StopCasting::new(Arc::new(cast));
    let mut session = CastSession::new();
    session.select(device("known"));
    session.start_playing().unwrap();

    uc.execute(&mut session).await.unwrap();
    assert_eq!(session.state(), CastState::Inactive);
}

#[tokio::test]
async fn volume_up_is_clamped_at_max() {
    let mut cast = MockCastController::new();
    cast.expect_set_volume().returning(|_| Ok(()));

    let uc = AdjustVolume::new(Arc::new(cast));
    let mut session = CastSession::new();
    session.set_volume(Volume::new(1.0));

    let result = uc.execute(&mut session, Direction::Up).await.unwrap();
    assert_eq!(result, Volume::new(1.0));
}

#[tokio::test]
async fn refresh_devices_returns_discovered() {
    let mut discovery = MockDeviceDiscovery::new();
    discovery
        .expect_discover()
        .times(1)
        .returning(|_| Ok(vec![device("a"), device("b")]));

    let uc = RefreshDevices::new(Arc::new(discovery));
    let found = uc.execute(Duration::from_secs(3)).await.unwrap();

    assert_eq!(found.len(), 2);
    assert_eq!(found[0].id, DeviceId::new("a"));
}

#[tokio::test]
async fn get_status_reflects_session() {
    let mut session = CastSession::new();
    session.select(device("a"));
    session.start_playing().unwrap();
    session.set_volume(Volume::new(0.4));

    let report = GetStatus::execute(&session);
    assert_eq!(report.state, CastState::Playing);
    assert_eq!(report.volume, Volume::new(0.4));
    assert!(!report.muted);
    assert_eq!(report.device.unwrap().id, DeviceId::new("a"));
}
