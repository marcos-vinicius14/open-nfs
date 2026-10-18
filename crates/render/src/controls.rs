//! Virtual touch controls: Analog steering wheel, pedals, and multitouch manager.
//!
//! Conforms to REQ-M3-06, REQ-M3-07, REQ-M3-08, REQ-M3-09, and Section 5 of the M3 guidelines.

use open_nfs_core::types::VehicleControlState;

/// Touch pointer interaction target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchTarget {
    SteeringWheel,
    ThrottlePedal,
    BrakePedal,
    CameraOrbit,
    Unassigned,
}

/// Active touch pointer state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActivePointer {
    pub id: u64,
    pub start_x: f32,
    pub start_y: f32,
    pub current_x: f32,
    pub current_y: f32,
    pub target: TouchTarget,
}

/// Virtual analog steering wheel (left side of landscape screen).
#[derive(Debug, Clone)]
pub struct VirtualSteeringWheel {
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
    pub current_angle_rad: f32,
    pub max_angle_rad: f32,
    pub return_speed: f32,
    pub is_active: bool,
    pub active_pointer_id: Option<u64>,
    pub reference_angle: f32,
}

impl Default for VirtualSteeringWheel {
    fn default() -> Self {
        Self {
            center_x: 200.0,
            center_y: 520.0,
            radius: 140.0,
            current_angle_rad: 0.0,
            max_angle_rad: 120.0_f32.to_radians(), // 120 degrees maximum turn
            return_speed: 12.0,                    // Exponential center return speed
            is_active: false,
            active_pointer_id: None,
            reference_angle: 0.0,
        }
    }
}

impl VirtualSteeringWheel {
    /// Update layout based on viewport dimensions.
    pub fn update_layout(&mut self, screen_width: f32, screen_height: f32) {
        let min_dim = screen_width.min(screen_height);
        self.radius = (min_dim * 0.28).clamp(90.0, 200.0);
        self.center_x = self.radius * 1.35;
        self.center_y = screen_height - self.radius * 1.25;
    }

    /// Check if point falls within the interactive touch radius.
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        let dx = x - self.center_x;
        let dy = y - self.center_y;
        (dx * dx + dy * dy) <= (self.radius * 1.5).powi(2)
    }

    /// Handle touch down event on the steering wheel.
    pub fn on_down(&mut self, pointer_id: u64, x: f32, y: f32) {
        self.is_active = true;
        self.active_pointer_id = Some(pointer_id);
        let angle = (y - self.center_y).atan2(x - self.center_x);
        self.reference_angle = angle - self.current_angle_rad;
    }

    /// Handle touch move on the steering wheel.
    pub fn on_move(&mut self, x: f32, y: f32) {
        if !self.is_active {
            return;
        }
        let angle = (y - self.center_y).atan2(x - self.center_x);
        let mut delta = angle - self.reference_angle;

        // Wrap delta angle to [-PI, PI] to handle boundary transitions smoothly
        while delta > std::f32::consts::PI {
            delta -= 2.0 * std::f32::consts::PI;
        }
        while delta < -std::f32::consts::PI {
            delta += 2.0 * std::f32::consts::PI;
        }

        self.current_angle_rad = delta.clamp(-self.max_angle_rad, self.max_angle_rad);
    }

    /// Release steering wheel touch.
    pub fn on_up(&mut self) {
        self.is_active = false;
        self.active_pointer_id = None;
    }

    /// Step steering wheel dynamics (smooth exponential return to center when released).
    pub fn tick(&mut self, delta_seconds: f32) {
        if !self.is_active && self.current_angle_rad.abs() > 1e-4 {
            let decay = (-self.return_speed * delta_seconds).exp();
            self.current_angle_rad *= decay;
            if self.current_angle_rad.abs() < 1e-3 {
                self.current_angle_rad = 0.0;
            }
        }
    }

    /// Return normalized steering command in `[-1.0 (left), 1.0 (right)]`.
    pub fn normalized_steering(&self) -> f32 {
        (self.current_angle_rad / self.max_angle_rad).clamp(-1.0, 1.0)
    }
}

/// Virtual pedals (accelerator on right, brake on left of accelerator).
#[derive(Debug, Clone)]
pub struct VirtualPedals {
    pub throttle_x: f32,
    pub throttle_y: f32,
    pub throttle_w: f32,
    pub throttle_h: f32,

    pub brake_x: f32,
    pub brake_y: f32,
    pub brake_w: f32,
    pub brake_h: f32,

    pub throttle_value: f32,
    pub brake_value: f32,

    pub throttle_pointer_id: Option<u64>,
    pub brake_pointer_id: Option<u64>,
}

impl Default for VirtualPedals {
    fn default() -> Self {
        Self {
            throttle_x: 1100.0,
            throttle_y: 460.0,
            throttle_w: 110.0,
            throttle_h: 220.0,

            brake_x: 940.0,
            brake_y: 500.0,
            brake_w: 120.0,
            brake_h: 180.0,

            throttle_value: 0.0,
            brake_value: 0.0,

            throttle_pointer_id: None,
            brake_pointer_id: None,
        }
    }
}

impl VirtualPedals {
    /// Update layout based on viewport dimensions.
    pub fn update_layout(&mut self, screen_width: f32, screen_height: f32) {
        let min_dim = screen_width.min(screen_height);
        let pedal_h = (min_dim * 0.35).clamp(120.0, 240.0);
        let throttle_w = pedal_h * 0.48;
        let brake_w = pedal_h * 0.58;

        self.throttle_w = throttle_w;
        self.throttle_h = pedal_h;
        self.throttle_x = screen_width - throttle_w - 35.0;
        self.throttle_y = screen_height - pedal_h - 35.0;

        self.brake_w = brake_w;
        self.brake_h = pedal_h * 0.85;
        self.brake_x = self.throttle_x - brake_w - 25.0;
        self.brake_y = screen_height - self.brake_h - 35.0;
    }

    pub fn is_throttle_hit(&self, x: f32, y: f32) -> bool {
        x >= self.throttle_x
            && x <= self.throttle_x + self.throttle_w
            && y >= self.throttle_y
            && y <= self.throttle_y + self.throttle_h
    }

    pub fn is_brake_hit(&self, x: f32, y: f32) -> bool {
        x >= self.brake_x
            && x <= self.brake_x + self.brake_w
            && y >= self.brake_y
            && y <= self.brake_y + self.brake_h
    }

    pub fn on_throttle_down(&mut self, pointer_id: u64, _x: f32, y: f32) {
        self.throttle_pointer_id = Some(pointer_id);
        self.update_throttle_intensity(y);
    }

    pub fn on_throttle_move(&mut self, _x: f32, y: f32) {
        self.update_throttle_intensity(y);
    }

    pub fn on_throttle_up(&mut self) {
        self.throttle_pointer_id = None;
        self.throttle_value = 0.0;
    }

    pub fn on_brake_down(&mut self, pointer_id: u64, _x: f32, y: f32) {
        self.brake_pointer_id = Some(pointer_id);
        self.update_brake_intensity(y);
    }

    pub fn on_brake_move(&mut self, _x: f32, y: f32) {
        self.update_brake_intensity(y);
    }

    pub fn on_brake_up(&mut self) {
        self.brake_pointer_id = None;
        self.brake_value = 0.0;
    }

    fn update_throttle_intensity(&mut self, y: f32) {
        // Full throttle at bottom, gradual upwards
        let norm = (y - self.throttle_y) / self.throttle_h;
        self.throttle_value = (0.5 + norm * 0.5).clamp(0.0, 1.0);
    }

    fn update_brake_intensity(&mut self, y: f32) {
        let norm = (y - self.brake_y) / self.brake_h;
        self.brake_value = (0.5 + norm * 0.5).clamp(0.0, 1.0);
    }
}

/// Unified touch interaction manager coordinating multitouch dispatch.
#[derive(Debug, Clone, Default)]
pub struct TouchManager {
    pub steering: VirtualSteeringWheel,
    pub pedals: VirtualPedals,
    pub pointers: [Option<ActivePointer>; 10], // Pre-allocated fixed capacity for zero heap allocation
    pub last_camera_drag: Option<(f32, f32)>,
}

impl TouchManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resize touch zones based on viewport.
    pub fn resize(&mut self, width: u32, height: u32) {
        let w = width as f32;
        let h = height as f32;
        self.steering.update_layout(w, h);
        self.pedals.update_layout(w, h);
    }

    /// Handle pointer down event.
    pub fn on_pointer_down(&mut self, id: u64, x: f32, y: f32) -> TouchTarget {
        let target = if self.steering.contains_point(x, y)
            && self.steering.active_pointer_id.is_none()
        {
            self.steering.on_down(id, x, y);
            TouchTarget::SteeringWheel
        } else if self.pedals.is_throttle_hit(x, y) && self.pedals.throttle_pointer_id.is_none() {
            self.pedals.on_throttle_down(id, x, y);
            TouchTarget::ThrottlePedal
        } else if self.pedals.is_brake_hit(x, y) && self.pedals.brake_pointer_id.is_none() {
            self.pedals.on_brake_down(id, x, y);
            TouchTarget::BrakePedal
        } else {
            TouchTarget::CameraOrbit
        };

        // Track pointer in pre-allocated array
        if let Some(slot) = self.pointers.iter_mut().find(|p| p.is_none()) {
            *slot = Some(ActivePointer {
                id,
                start_x: x,
                start_y: y,
                current_x: x,
                current_y: y,
                target,
            });
        }

        target
    }

    /// Handle pointer move event.
    pub fn on_pointer_move(&mut self, id: u64, x: f32, y: f32) {
        let mut target = TouchTarget::Unassigned;
        let mut prev_pos = None;

        for slot in self.pointers.iter_mut().flatten() {
            if slot.id == id {
                prev_pos = Some((slot.current_x, slot.current_y));
                slot.current_x = x;
                slot.current_y = y;
                target = slot.target;
                break;
            }
        }

        match target {
            TouchTarget::SteeringWheel => self.steering.on_move(x, y),
            TouchTarget::ThrottlePedal => self.pedals.on_throttle_move(x, y),
            TouchTarget::BrakePedal => self.pedals.on_brake_move(x, y),
            TouchTarget::CameraOrbit => {
                if let Some((px, py)) = prev_pos {
                    self.last_camera_drag = Some((x - px, y - py));
                }
            }
            TouchTarget::Unassigned => {}
        }
    }

    /// Handle pointer up event.
    pub fn on_pointer_up(&mut self, id: u64) {
        for slot in &mut self.pointers {
            if let Some(pointer) = slot {
                if pointer.id == id {
                    match pointer.target {
                        TouchTarget::SteeringWheel => self.steering.on_up(),
                        TouchTarget::ThrottlePedal => self.pedals.on_throttle_up(),
                        TouchTarget::BrakePedal => self.pedals.on_brake_up(),
                        TouchTarget::CameraOrbit => self.last_camera_drag = None,
                        TouchTarget::Unassigned => {}
                    }
                    *slot = None;
                    break;
                }
            }
        }
    }

    /// Cancel all active pointers (called on app pause or focus lost).
    pub fn cancel_all_touches(&mut self) {
        self.steering.on_up();
        self.pedals.on_throttle_up();
        self.pedals.on_brake_up();
        self.last_camera_drag = None;
        for slot in &mut self.pointers {
            *slot = None;
        }
    }

    /// Advance smooth dynamics (e.g. steering wheel return-to-center).
    pub fn tick(&mut self, delta_seconds: f32) {
        self.steering.tick(delta_seconds);
    }

    /// Consume camera drag delta and clear it.
    pub fn take_camera_drag(&mut self) -> Option<(f32, f32)> {
        self.last_camera_drag.take()
    }

    /// Export current normalized vehicle control state.
    pub fn vehicle_control_state(&self) -> VehicleControlState {
        VehicleControlState::new(
            self.steering.normalized_steering(),
            self.pedals.throttle_value,
            self.pedals.brake_value,
            false,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steering_wheel_rotation_and_centering() {
        let mut wheel = VirtualSteeringWheel::default();
        wheel.update_layout(1280.0, 720.0);

        // Touch on right side of wheel
        wheel.on_down(1, wheel.center_x + wheel.radius, wheel.center_y);
        wheel.on_move(wheel.center_x + wheel.radius, wheel.center_y + 100.0);
        let steer = wheel.normalized_steering();
        assert!(steer > 0.0);

        // Release and tick smooth return to center
        wheel.on_up();
        assert!(!wheel.is_active);
        for _ in 0..60 {
            wheel.tick(0.01666);
        }
        assert!(wheel.normalized_steering().abs() < 1e-2);
    }

    #[test]
    fn multitouch_pedal_and_wheel_simultaneous() {
        let mut mgr = TouchManager::new();
        mgr.resize(1280, 720);

        // Pointer 1: Steering wheel
        let t1 = mgr.on_pointer_down(1, mgr.steering.center_x, mgr.steering.center_y);
        assert_eq!(t1, TouchTarget::SteeringWheel);

        // Pointer 2: Throttle pedal
        let t2 = mgr.on_pointer_down(
            2,
            mgr.pedals.throttle_x + 10.0,
            mgr.pedals.throttle_y + 10.0,
        );
        assert_eq!(t2, TouchTarget::ThrottlePedal);

        let state = mgr.vehicle_control_state();
        assert!(state.throttle > 0.0);

        // Cancel on pause
        mgr.cancel_all_touches();
        let reset_state = mgr.vehicle_control_state();
        assert_eq!(reset_state.throttle, 0.0);
    }
}
