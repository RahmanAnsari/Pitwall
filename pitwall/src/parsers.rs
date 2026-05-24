//! Packet parsing - zero-copy extraction from raw UDP bytes.

use bytemuck;

use crate::packets::*;

/// Parse the packet header from raw bytes.
pub fn parse_header(data: &[u8]) -> Option<PacketHeader> {
    if data.len() < HEADER_SIZE {
        return None;
    }
    Some(*bytemuck::from_bytes::<PacketHeader>(&data[..HEADER_SIZE]))
}

/// Parse player car motion data.
pub fn parse_player_motion(data: &[u8], player_index: u8) -> Option<CarMotionData> {
    let offset = HEADER_SIZE + (player_index as usize) * std::mem::size_of::<CarMotionData>();
    let end = offset + std::mem::size_of::<CarMotionData>();
    if data.len() < end {
        return None;
    }
    Some(*bytemuck::from_bytes::<CarMotionData>(&data[offset..end]))
}

/// Parse session header data.
pub fn parse_session_data(data: &[u8]) -> Option<SessionDataHeader> {
    let offset = HEADER_SIZE;
    let end = offset + std::mem::size_of::<SessionDataHeader>();
    if data.len() < end {
        return None;
    }
    Some(*bytemuck::from_bytes::<SessionDataHeader>(&data[offset..end]))
}

/// Parse player car lap data.
pub fn parse_player_lap_data(data: &[u8], player_index: u8) -> Option<LapDataCar> {
    let offset = HEADER_SIZE + (player_index as usize) * std::mem::size_of::<LapDataCar>();
    let end = offset + std::mem::size_of::<LapDataCar>();
    if data.len() < end {
        return None;
    }
    Some(*bytemuck::from_bytes::<LapDataCar>(&data[offset..end]))
}

/// Parse player car telemetry data.
pub fn parse_player_car_telemetry(data: &[u8], player_index: u8) -> Option<CarTelemetryData> {
    let offset = HEADER_SIZE + (player_index as usize) * std::mem::size_of::<CarTelemetryData>();
    let end = offset + std::mem::size_of::<CarTelemetryData>();
    if data.len() < end {
        return None;
    }
    Some(*bytemuck::from_bytes::<CarTelemetryData>(&data[offset..end]))
}

/// Parse player car status data.
pub fn parse_player_car_status(data: &[u8], player_index: u8) -> Option<CarStatusData> {
    let offset = HEADER_SIZE + (player_index as usize) * std::mem::size_of::<CarStatusData>();
    let end = offset + std::mem::size_of::<CarStatusData>();
    if data.len() < end {
        return None;
    }
    Some(*bytemuck::from_bytes::<CarStatusData>(&data[offset..end]))
}

/// Parse extended motion data (player car only, no array offset).
pub fn parse_motion_ex(data: &[u8]) -> Option<MotionExData> {
    let offset = HEADER_SIZE;
    let end = offset + std::mem::size_of::<MotionExData>();
    if data.len() < end {
        return None;
    }
    Some(*bytemuck::from_bytes::<MotionExData>(&data[offset..end]))
}

/// Parse event code (4 ASCII bytes after header).
pub fn parse_event_code(data: &[u8]) -> Option<[u8; 4]> {
    let offset = HEADER_SIZE;
    if data.len() < offset + 4 {
        return None;
    }
    let mut code = [0u8; 4];
    code.copy_from_slice(&data[offset..offset + 4]);
    Some(code)
}
