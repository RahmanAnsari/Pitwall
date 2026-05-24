//! Session state management.
//!
//! Tracks session lifecycle, accumulates telemetry samples, detects lap completions.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::packets::{CarMotionData, CarStatusData, CarTelemetryData, LapDataCar, MotionExData, SessionDataHeader};

/// Immutable session metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: String,
    pub session_uid: u64,
    pub track_id: i8,
    pub session_type: u8,
    pub weather: u8,
    pub air_temperature: i8,
    pub track_temperature: i8,
    pub total_laps: u8,
    pub start_time: String,
    pub game_version: String,
}

/// Completed lap record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LapRecord {
    pub lap_id: String,
    pub session_id: String,
    pub lap_number: u8,
    pub lap_time_ms: u32,
    pub sector1_ms: f32,
    pub sector2_ms: f32,
    pub sector3_ms: f32,
    pub position: u8,
    pub penalties: u8,
    pub warnings: u8,
}

/// A single telemetry sample (one frame, merged from multiple packets).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetrySample {
    // Timing
    pub timestamp: f32,
    pub frame: u32,
    pub lap_number: u8,
    pub lap_distance: f32,
    pub total_distance: f32,

    // Driver inputs
    pub throttle: f32,
    pub brake: f32,
    pub steer: f32,
    pub clutch: u8,
    pub gear: i8,

    // Vehicle state
    pub speed: u16,
    pub rpm: u16,
    pub drs: u8,
    pub drs_allowed: u8,
    pub ers_store_energy: f32,
    pub ers_deploy_mode: u8,
    pub fuel_in_tank: f32,

    // Position & motion
    pub world_x: f32,
    pub world_y: f32,
    pub world_z: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub velocity_z: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,

    // G-forces
    pub g_lateral: f32,
    pub g_longitudinal: f32,
    pub g_vertical: f32,

    // Tyre temps (surface)
    pub tyre_surface_fl: u8,
    pub tyre_surface_fr: u8,
    pub tyre_surface_rl: u8,
    pub tyre_surface_rr: u8,

    // Tyre temps (inner)
    pub tyre_inner_fl: u8,
    pub tyre_inner_fr: u8,
    pub tyre_inner_rl: u8,
    pub tyre_inner_rr: u8,

    // Brake temps
    pub brake_temp_fl: u16,
    pub brake_temp_fr: u16,
    pub brake_temp_rl: u16,
    pub brake_temp_rr: u16,

    // Suspension
    pub suspension_pos_fl: f32,
    pub suspension_pos_fr: f32,
    pub suspension_pos_rl: f32,
    pub suspension_pos_rr: f32,
    pub suspension_vel_fl: f32,
    pub suspension_vel_fr: f32,
    pub suspension_vel_rl: f32,
    pub suspension_vel_rr: f32,
    pub suspension_acc_fl: f32,
    pub suspension_acc_fr: f32,
    pub suspension_acc_rl: f32,
    pub suspension_acc_rr: f32,

    // Wheel speeds
    pub wheel_speed_fl: f32,
    pub wheel_speed_fr: f32,
    pub wheel_speed_rl: f32,
    pub wheel_speed_rr: f32,
}

impl TelemetrySample {
    pub fn merge_motion(&mut self, m: &CarMotionData) {
        self.world_x = m.world_position_x;
        self.world_y = m.world_position_y;
        self.world_z = m.world_position_z;
        self.velocity_x = m.world_velocity_x;
        self.velocity_y = m.world_velocity_y;
        self.velocity_z = m.world_velocity_z;
        self.g_lateral = m.g_force_lateral;
        self.g_longitudinal = m.g_force_longitudinal;
        self.g_vertical = m.g_force_vertical;
        self.yaw = m.yaw;
        self.pitch = m.pitch;
        self.roll = m.roll;
    }

    pub fn merge_car_telemetry(&mut self, t: &CarTelemetryData) {
        self.speed = t.speed;
        self.throttle = t.throttle;
        self.steer = t.steer;
        self.brake = t.brake;
        self.clutch = t.clutch;
        self.gear = t.gear;
        self.rpm = t.engine_rpm;
        self.drs = t.drs;
        // Wheel order in F1 24: [0]=RL, [1]=RR, [2]=FL, [3]=FR
        self.brake_temp_fl = t.brakes_temperature[2];
        self.brake_temp_fr = t.brakes_temperature[3];
        self.brake_temp_rl = t.brakes_temperature[0];
        self.brake_temp_rr = t.brakes_temperature[1];
        self.tyre_surface_fl = t.tyres_surface_temperature[2];
        self.tyre_surface_fr = t.tyres_surface_temperature[3];
        self.tyre_surface_rl = t.tyres_surface_temperature[0];
        self.tyre_surface_rr = t.tyres_surface_temperature[1];
        self.tyre_inner_fl = t.tyres_inner_temperature[2];
        self.tyre_inner_fr = t.tyres_inner_temperature[3];
        self.tyre_inner_rl = t.tyres_inner_temperature[0];
        self.tyre_inner_rr = t.tyres_inner_temperature[1];
    }

    pub fn merge_car_status(&mut self, s: &CarStatusData) {
        self.drs_allowed = s.drs_allowed;
        self.ers_store_energy = s.ers_store_energy;
        self.ers_deploy_mode = s.ers_deploy_mode;
        self.fuel_in_tank = s.fuel_in_tank;
    }

    pub fn merge_motion_ex(&mut self, ex: &MotionExData) {
        // Wheel order in F1 24: [0]=RL, [1]=RR, [2]=FL, [3]=FR
        self.suspension_pos_fl = ex.suspension_position[2];
        self.suspension_pos_fr = ex.suspension_position[3];
        self.suspension_pos_rl = ex.suspension_position[0];
        self.suspension_pos_rr = ex.suspension_position[1];
        self.suspension_vel_fl = ex.suspension_velocity[2];
        self.suspension_vel_fr = ex.suspension_velocity[3];
        self.suspension_vel_rl = ex.suspension_velocity[0];
        self.suspension_vel_rr = ex.suspension_velocity[1];
        self.suspension_acc_fl = ex.suspension_acceleration[2];
        self.suspension_acc_fr = ex.suspension_acceleration[3];
        self.suspension_acc_rl = ex.suspension_acceleration[0];
        self.suspension_acc_rr = ex.suspension_acceleration[1];
        self.wheel_speed_fl = ex.wheel_speed[2];
        self.wheel_speed_fr = ex.wheel_speed[3];
        self.wheel_speed_rl = ex.wheel_speed[0];
        self.wheel_speed_rr = ex.wheel_speed[1];
    }

    pub fn merge_lap_data(&mut self, l: &LapDataCar) {
        self.lap_number = l.current_lap_num;
        self.lap_distance = l.lap_distance;
        self.total_distance = l.total_distance;
    }
}

/// Event record for storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub timestamp: f32,
    pub frame: u32,
    pub event_code: String,
}

/// Mutable session state.
pub struct SessionState {
    pub meta: Option<SessionMeta>,
    pub active: bool,
    pub current_lap: u8,
    pub last_completed_lap: u8,
    pub laps: Vec<LapRecord>,
    pub telemetry: Vec<TelemetrySample>,
    pub events: Vec<EventRecord>,
    current_sample: Option<TelemetrySample>,
    // Cached sector times from the previous frame (before lap transition)
    last_sector1_ms: f32,
    last_sector2_ms: f32,
    // Live sector data for the current lap
    pub current_sector: u8,
    pub live_sector1_ms: f32,
    pub live_sector2_ms: f32,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            meta: None,
            active: false,
            current_lap: 0,
            last_completed_lap: 0,
            laps: Vec::new(),
            telemetry: Vec::with_capacity(100_000),
            events: Vec::new(),
            current_sample: None,
            last_sector1_ms: 0.0,
            last_sector2_ms: 0.0,
            current_sector: 0,
            live_sector1_ms: 0.0,
            live_sector2_ms: 0.0,
        }
    }

    pub fn start_session(&mut self, session: &SessionDataHeader, session_uid: u64, game_year: u8, game_major: u8, game_minor: u8) {
        self.meta = Some(SessionMeta {
            session_id: Uuid::new_v4().to_string(),
            session_uid,
            track_id: session.track_id,
            session_type: session.session_type,
            weather: session.weather,
            air_temperature: session.air_temperature,
            track_temperature: session.track_temperature,
            total_laps: session.total_laps,
            start_time: Utc::now().to_rfc3339(),
            game_version: format!("{game_year}.{game_major}.{game_minor}"),
        });
        self.active = true;
        self.current_lap = 0;
        self.last_completed_lap = 0;
        self.laps.clear();
        self.telemetry.clear();
        self.events.clear();
        self.current_sample = None;
    }

    pub fn end_session(&mut self) {
        // Flush any pending sample
        self.flush_sample();
        self.active = false;
    }

    /// Get or create a sample for the given frame. If frame changes, flush previous.
    pub fn get_sample(&mut self, timestamp: f32, frame: u32) -> &mut TelemetrySample {
        if let Some(ref sample) = self.current_sample {
            if sample.frame != frame {
                self.flush_sample();
            }
        }

        if self.current_sample.is_none() {
            self.current_sample = Some(TelemetrySample {
                timestamp,
                frame,
                ..Default::default()
            });
        }

        self.current_sample.as_mut().unwrap()
    }

    fn flush_sample(&mut self) {
        if let Some(sample) = self.current_sample.take() {
            self.telemetry.push(sample);
        }
    }

    /// Detect and record lap completion.
    pub fn check_lap_completion(&mut self, lap_data: &LapDataCar) {
        let current = lap_data.current_lap_num;

        if current > self.current_lap && self.current_lap > 0 {
            // Lap changed - record the completed lap using cached sector times
            if lap_data.last_lap_time_ms > 0 {
                let session_id = self.meta.as_ref().map(|m| m.session_id.clone()).unwrap_or_default();

                let s1_ms = self.last_sector1_ms;
                let s2_ms = self.last_sector2_ms;
                let s3 = if lap_data.last_lap_time_ms as f32 > s1_ms + s2_ms {
                    lap_data.last_lap_time_ms as f32 - s1_ms - s2_ms
                } else {
                    0.0
                };

                self.laps.push(LapRecord {
                    lap_id: Uuid::new_v4().to_string(),
                    session_id,
                    lap_number: self.current_lap,
                    lap_time_ms: lap_data.last_lap_time_ms,
                    sector1_ms: s1_ms,
                    sector2_ms: s2_ms,
                    sector3_ms: s3,
                    position: lap_data.car_position,
                    penalties: lap_data.penalties,
                    warnings: lap_data.total_warnings,
                });
                self.last_completed_lap = self.current_lap;
            }
            // Reset live sectors for the new lap
            self.live_sector1_ms = 0.0;
            self.live_sector2_ms = 0.0;
        }

        self.current_lap = current;

        // Track current sector and live sector times
        self.current_sector = lap_data.sector;

        let s1 = lap_data.sector1_time_minutes_part as f32 * 60_000.0
            + lap_data.sector1_time_ms_part as f32;
        let s2 = lap_data.sector2_time_minutes_part as f32 * 60_000.0
            + lap_data.sector2_time_ms_part as f32;

        // Update live sectors for the current lap (reset on new lap)
        if s1 > 0.0 {
            self.live_sector1_ms = s1;
            self.last_sector1_ms = s1;
        }
        if s2 > 0.0 {
            self.live_sector2_ms = s2;
            self.last_sector2_ms = s2;
        }
    }

    pub fn add_event(&mut self, timestamp: f32, frame: u32, code: [u8; 4]) {
        self.events.push(EventRecord {
            timestamp,
            frame,
            event_code: String::from_utf8_lossy(&code).to_string(),
        });
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Get the most recent telemetry sample (current or last flushed).
    pub fn latest_sample(&self) -> Option<&TelemetrySample> {
        self.current_sample.as_ref().or_else(|| self.telemetry.last())
    }
}
