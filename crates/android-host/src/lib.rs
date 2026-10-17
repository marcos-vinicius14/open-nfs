//! `open-nfs-android-host` — Android host integration and platform isolation layer.
//!
//! Conforms to ADR-002 and NFR-006 by isolating Android platform specifics behind
//! platform-agnostic lifecycle and input traits.

#![deny(unsafe_code)]

use open_nfs_core::types::FixedDelta;

/// Surface lifecycle state for graphics pipeline recreation and presentation pacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceState {
    /// No native window/surface is currently attached.
    Detached,
    /// Native surface is active with positive dimensions.
    Active { width: u32, height: u32 },
    /// Surface is paused or suspended (app in background).
    Suspended,
}

/// Normalized touch input event originating from mobile touchscreen or desktop simulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchEvent {
    Down { id: u64, x: f32, y: f32 },
    Move { id: u64, x: f32, y: f32 },
    Up { id: u64, x: f32, y: f32 },
    Cancel { id: u64 },
}

/// Platform-agnostic engine host contract.
///
/// Implementations handle native platform dispatch without exposing OS-specific
/// types to the simulation or rendering crates.
pub trait PlatformHost {
    /// Query the current surface state.
    fn surface_state(&self) -> SurfaceState;

    /// Process a lifecycle event (e.g. pause/resume).
    fn on_lifecycle_change(&mut self, state: SurfaceState);

    /// Dispatch incoming touch interaction.
    fn on_touch(&mut self, event: TouchEvent);

    /// Step simulation or presentation by fixed delta.
    fn tick(&mut self, delta: FixedDelta);
}

/// Baseline mock host implementation used for unit testing and desktop smoke runs.
#[derive(Debug, Default)]
pub struct MockHost {
    pub surface: Option<SurfaceState>,
    pub last_touch: Option<TouchEvent>,
    pub tick_count: u64,
}

impl MockHost {
    pub fn new() -> Self {
        Self {
            surface: Some(SurfaceState::Detached),
            last_touch: None,
            tick_count: 0,
        }
    }
}

impl PlatformHost for MockHost {
    fn surface_state(&self) -> SurfaceState {
        self.surface.unwrap_or(SurfaceState::Detached)
    }

    fn on_lifecycle_change(&mut self, state: SurfaceState) {
        tracing::debug!(?state, "Host surface state changed");
        self.surface = Some(state);
    }

    fn on_touch(&mut self, event: TouchEvent) {
        self.last_touch = Some(event);
    }

    fn tick(&mut self, _delta: FixedDelta) {
        self.tick_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_lifecycle_transitions() {
        let mut host = MockHost::new();
        assert_eq!(host.surface_state(), SurfaceState::Detached);

        host.on_lifecycle_change(SurfaceState::Active {
            width: 1920,
            height: 1080,
        });
        assert_eq!(
            host.surface_state(),
            SurfaceState::Active {
                width: 1920,
                height: 1080
            }
        );

        host.on_lifecycle_change(SurfaceState::Suspended);
        assert_eq!(host.surface_state(), SurfaceState::Suspended);
    }

    #[test]
    fn touch_event_dispatch() {
        let mut host = MockHost::new();
        let touch = TouchEvent::Down {
            id: 1,
            x: 100.0,
            y: 200.0,
        };
        host.on_touch(touch);
        assert_eq!(host.last_touch, Some(touch));
    }

    #[test]
    fn fixed_step_tick() {
        let mut host = MockHost::new();
        host.tick(FixedDelta::STEP_60HZ);
        host.tick(FixedDelta::STEP_60HZ);
        assert_eq!(host.tick_count, 2);
    }
}
