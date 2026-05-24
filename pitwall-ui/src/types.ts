export interface LapSummary {
  lap_number: number;
  lap_time_ms: number;
  sector1_ms: number;
  sector2_ms: number;
  sector3_ms: number;
  position: number;
}

export interface LiveFrame {
  session_active: boolean;
  track: string;
  session_type: string;
  current_lap: number;
  total_laps: number;

  speed: number;
  rpm: number;
  gear: number;
  throttle: number;
  brake: number;
  steer: number;
  drs: number;
  drs_allowed: number;
  ers_deploy_mode: number;
  ers_store_energy: number;
  fuel_in_tank: number;

  tyre_fl: number;
  tyre_fr: number;
  tyre_rl: number;
  tyre_rr: number;

  brake_temp_fl: number;
  brake_temp_fr: number;
  brake_temp_rl: number;
  brake_temp_rr: number;

  lap_distance: number;
  total_distance: number;
  session_time: number;

  g_lateral: number;
  g_longitudinal: number;

  current_sector: number;
  current_sector1_ms: number;
  current_sector2_ms: number;

  laps: LapSummary[];
}
