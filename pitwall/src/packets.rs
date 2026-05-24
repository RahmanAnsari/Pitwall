//! F1 24 UDP packet definitions.
//!
//! Based on the official F1 24 UDP specification from EA/Codemasters.
//! All structs are `repr(C, packed)` matching the wire format (little-endian).
//!
//! Reference: https://github.com/MacManley/f1-24-udp

use bytemuck::{Pod, Zeroable};

/// Packet type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketId {
    Motion = 0,
    Session = 1,
    LapData = 2,
    Event = 3,
    Participants = 4,
    CarSetups = 5,
    CarTelemetry = 6,
    CarStatus = 7,
    FinalClassification = 8,
    LobbyInfo = 9,
    CarDamage = 10,
    SessionHistory = 11,
    TyreSets = 12,
    MotionEx = 13,
    TimeTrial = 14,
}

impl PacketId {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Motion),
            1 => Some(Self::Session),
            2 => Some(Self::LapData),
            3 => Some(Self::Event),
            4 => Some(Self::Participants),
            5 => Some(Self::CarSetups),
            6 => Some(Self::CarTelemetry),
            7 => Some(Self::CarStatus),
            8 => Some(Self::FinalClassification),
            9 => Some(Self::LobbyInfo),
            10 => Some(Self::CarDamage),
            11 => Some(Self::SessionHistory),
            12 => Some(Self::TyreSets),
            13 => Some(Self::MotionEx),
            14 => Some(Self::TimeTrial),
            _ => None,
        }
    }
}

/// Common header for all F1 24 UDP packets (29 bytes)
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
pub struct PacketHeader {
    pub packet_format: u16,       // 2024
    pub game_year: u8,            // 24
    pub game_major_version: u8,
    pub game_minor_version: u8,
    pub packet_version: u8,
    pub packet_id: u8,
    pub session_uid: u64,
    pub session_time: f32,
    pub frame_identifier: u32,
    pub overall_frame_identifier: u32,
    pub player_car_index: u8,
    pub secondary_player_car_index: u8,
}

/// Per-car motion data (60 bytes per car)
/// Packet size: 1349 bytes (header + 22 cars)
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
pub struct CarMotionData {
    pub world_position_x: f32,
    pub world_position_y: f32,
    pub world_position_z: f32,
    pub world_velocity_x: f32,
    pub world_velocity_y: f32,
    pub world_velocity_z: f32,
    pub world_forward_dir_x: i16,
    pub world_forward_dir_y: i16,
    pub world_forward_dir_z: i16,
    pub world_right_dir_x: i16,
    pub world_right_dir_y: i16,
    pub world_right_dir_z: i16,
    pub g_force_lateral: f32,
    pub g_force_longitudinal: f32,
    pub g_force_vertical: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

/// Session data header fields (first portion of PacketSessionData after header)
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
pub struct SessionDataHeader {
    pub weather: u8,
    pub track_temperature: i8,
    pub air_temperature: i8,
    pub total_laps: u8,
    pub track_length: u16,
    pub session_type: u8,
    pub track_id: i8,
    pub formula: u8,
    pub session_time_left: u16,
    pub session_duration: u16,
    pub pit_speed_limit: u8,
    pub game_paused: u8,
    pub is_spectating: u8,
    pub spectator_car_index: u8,
    pub sli_pro_native_support: u8,
    pub num_marshal_zones: u8,
}

/// Per-car lap data (F1 24 format)
/// Note: Sector times use ms_part (u16) + minutes_part (u8) format
/// Packet size: 1285 bytes
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
pub struct LapDataCar {
    pub last_lap_time_ms: u32,
    pub current_lap_time_ms: u32,
    pub sector1_time_ms_part: u16,
    pub sector1_time_minutes_part: u8,
    pub sector2_time_ms_part: u16,
    pub sector2_time_minutes_part: u8,
    pub delta_to_car_in_front_ms_part: u16,
    pub delta_to_car_in_front_minutes_part: u8,
    pub delta_to_race_leader_ms_part: u16,
    pub delta_to_race_leader_minutes_part: u8,
    pub lap_distance: f32,
    pub total_distance: f32,
    pub safety_car_delta: f32,
    pub car_position: u8,
    pub current_lap_num: u8,
    pub pit_status: u8,
    pub num_pit_stops: u8,
    pub sector: u8,
    pub current_lap_invalid: u8,
    pub penalties: u8,
    pub total_warnings: u8,
    pub corner_cutting_warnings: u8,
    pub num_unserved_drive_through_pens: u8,
    pub num_unserved_stop_go_pens: u8,
    pub grid_position: u8,
    pub driver_status: u8,
    pub result_status: u8,
    pub pit_lane_timer_active: u8,
    pub pit_lane_time_in_lane_ms: u16,
    pub pit_stop_timer_ms: u16,
    pub pit_stop_should_serve_pen: u8,
    pub speed_trap_fastest_speed: f32,
    pub speed_trap_fastest_lap: u8,
}

/// Per-car telemetry data
/// Packet size: 1352 bytes
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
pub struct CarTelemetryData {
    pub speed: u16,
    pub throttle: f32,
    pub steer: f32,
    pub brake: f32,
    pub clutch: u8,
    pub gear: i8,
    pub engine_rpm: u16,
    pub drs: u8,
    pub rev_lights_percent: u8,
    pub rev_lights_bit_value: u16,
    pub brakes_temperature: [u16; 4],     // RL, RR, FL, FR
    pub tyres_surface_temperature: [u8; 4], // RL, RR, FL, FR
    pub tyres_inner_temperature: [u8; 4],   // RL, RR, FL, FR
    pub engine_temperature: u16,
    pub tyres_pressure: [f32; 4],           // RL, RR, FL, FR
    pub surface_type: [u8; 4],              // RL, RR, FL, FR
}

/// Per-car status data
/// Packet size: 1239 bytes
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
pub struct CarStatusData {
    pub traction_control: u8,
    pub anti_lock_brakes: u8,
    pub fuel_mix: u8,
    pub front_brake_bias: u8,
    pub pit_limiter_status: u8,
    pub fuel_in_tank: f32,
    pub fuel_capacity: f32,
    pub fuel_remaining_laps: f32,
    pub max_rpm: u16,
    pub idle_rpm: u16,
    pub max_gears: u8,
    pub drs_allowed: u8,
    pub drs_activation_distance: u16,
    pub actual_tyre_compound: u8,
    pub visual_tyre_compound: u8,
    pub tyres_age_laps: u8,
    pub vehicle_fia_flags: i8,
    pub engine_power_ice: f32,
    pub engine_power_mguk: f32,
    pub ers_store_energy: f32,
    pub ers_deploy_mode: u8,
    pub ers_harvested_this_lap_mguk: f32,
    pub ers_harvested_this_lap_mguh: f32,
    pub ers_deployed_this_lap: f32,
    pub network_paused: u8,
}

/// Extended motion data (player car only)
/// Packet size: 237 bytes
/// NOTE: Wheel arrays are ordered RL, RR, FL, FR
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
pub struct MotionExData {
    pub suspension_position: [f32; 4],      // RL, RR, FL, FR
    pub suspension_velocity: [f32; 4],      // RL, RR, FL, FR
    pub suspension_acceleration: [f32; 4],  // RL, RR, FL, FR
    pub wheel_speed: [f32; 4],             // RL, RR, FL, FR
    pub wheel_slip_ratio: [f32; 4],        // RL, RR, FL, FR
    pub wheel_slip_angle: [f32; 4],        // RL, RR, FL, FR
    pub wheel_lat_force: [f32; 4],         // RL, RR, FL, FR
    pub wheel_long_force: [f32; 4],        // RL, RR, FL, FR
    pub height_of_cog_above_ground: f32,
    pub local_velocity_x: f32,
    pub local_velocity_y: f32,
    pub local_velocity_z: f32,
    pub angular_velocity_x: f32,
    pub angular_velocity_y: f32,
    pub angular_velocity_z: f32,
    pub angular_acceleration_x: f32,
    pub angular_acceleration_y: f32,
    pub angular_acceleration_z: f32,
    pub front_wheels_angle: f32,
    pub wheel_vert_force: [f32; 4],        // RL, RR, FL, FR
    pub front_aero_height: f32,
    pub rear_aero_height: f32,
    pub front_roll_angle: f32,
    pub rear_roll_angle: f32,
    pub chassis_yaw: f32,
}

/// Event string codes
#[allow(dead_code)]
pub mod event_codes {
    pub const SESSION_STARTED: &[u8; 4] = b"SSTA";
    pub const SESSION_ENDED: &[u8; 4] = b"SEND";
    pub const FASTEST_LAP: &[u8; 4] = b"FTLP";
    pub const DRS_ENABLED: &[u8; 4] = b"DRSE";
    pub const DRS_DISABLED: &[u8; 4] = b"DRSD";
    pub const CHEQUERED_FLAG: &[u8; 4] = b"CHQF";
    pub const LIGHTS_OUT: &[u8; 4] = b"LGOT";
    pub const PENALTY: &[u8; 4] = b"PENA";
    pub const OVERTAKE: &[u8; 4] = b"OVTK";
    pub const SAFETY_CAR: &[u8; 4] = b"SCAR";
    pub const COLLISION: &[u8; 4] = b"COLL";
    pub const RED_FLAG: &[u8; 4] = b"RDFL";
}

#[allow(dead_code)]
pub const NUM_CARS: usize = 22;
pub const HEADER_SIZE: usize = std::mem::size_of::<PacketHeader>();
