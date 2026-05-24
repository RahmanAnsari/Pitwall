import type { LapSummary } from "../types";

interface Props {
  laps: LapSummary[];
  currentLap: number;
  totalLaps: number;
  liveSector1Ms: number;
  liveSector2Ms: number;
}

function formatTime(ms: number): string {
  const totalSec = ms / 1000;
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return min > 0 ? `${min}:${sec.toFixed(3).padStart(6, "0")}` : sec.toFixed(3);
}

function formatSector(ms: number): string {
  return (ms / 1000).toFixed(3);
}

export function LapTable({ laps, currentLap, totalLaps, liveSector1Ms, liveSector2Ms }: Props) {
  // Include all laps that have a valid lap time (sectors may be 0 from old data)
  const completedLaps = laps.filter((l) => l.lap_time_ms > 0);

  // Collect all valid sector times (completed + live) to find the overall best
  const allS1: number[] = completedLaps.filter((l) => l.sector1_ms > 0).map((l) => l.sector1_ms);
  const allS2: number[] = completedLaps.filter((l) => l.sector2_ms > 0).map((l) => l.sector2_ms);
  const allS3: number[] = completedLaps.filter((l) => l.sector3_ms > 0).map((l) => l.sector3_ms);

  if (liveSector1Ms > 0) allS1.push(liveSector1Ms);
  if (liveSector2Ms > 0) allS2.push(liveSector2Ms);

  const fastestLapTime = completedLaps.length > 0 ? Math.min(...completedLaps.map((l) => l.lap_time_ms)) : 0;
  const fastestS1 = allS1.length > 0 ? Math.min(...allS1) : 0;
  const fastestS2 = allS2.length > 0 ? Math.min(...allS2) : 0;
  const fastestS3 = allS3.length > 0 ? Math.min(...allS3) : 0;

  // Check if current live sectors are the best
  const liveS1Purple = liveSector1Ms > 0 && liveSector1Ms <= fastestS1;
  const liveS2Purple = liveSector2Ms > 0 && liveSector2Ms <= fastestS2;

  return (
    <div className="panel lap-panel">
      <div className="panel-title">
        LAP {currentLap}/{totalLaps}
      </div>
      <table className="lap-table">
        <thead>
          <tr>
            <th>LAP</th>
            <th>S1</th>
            <th>S2</th>
            <th>S3</th>
            <th>TIME</th>
          </tr>
        </thead>
        <tbody>
          {/* Current lap row - live sectors */}
          {currentLap > 0 && (
            <tr className="current-lap-row">
              <td>{currentLap}</td>
              <td className={liveSector1Ms > 0 ? (liveS1Purple ? "purple" : "") : "dim"}>
                {liveSector1Ms > 0 ? formatSector(liveSector1Ms) : "—"}
              </td>
              <td className={liveSector2Ms > 0 ? (liveS2Purple ? "purple" : "") : "dim"}>
                {liveSector2Ms > 0 ? formatSector(liveSector2Ms) : "—"}
              </td>
              <td className="dim">—</td>
              <td className="dim">—</td>
            </tr>
          )}
          {/* Completed laps */}
          {completedLaps.map((lap) => {
            const isFastestLap = lap.lap_time_ms === fastestLapTime;
            const isFastestS1 = lap.sector1_ms > 0 && lap.sector1_ms === fastestS1;
            const isFastestS2 = lap.sector2_ms > 0 && lap.sector2_ms === fastestS2;
            const isFastestS3 = lap.sector3_ms > 0 && lap.sector3_ms === fastestS3;

            return (
              <tr key={lap.lap_number} className={isFastestLap ? "fastest-lap" : ""}>
                <td>{lap.lap_number}</td>
                <td className={isFastestS1 ? "purple" : ""}>{formatSector(lap.sector1_ms)}</td>
                <td className={isFastestS2 ? "purple" : ""}>{formatSector(lap.sector2_ms)}</td>
                <td className={isFastestS3 ? "purple" : ""}>{formatSector(lap.sector3_ms)}</td>
                <td className={isFastestLap ? "purple" : ""}>{formatTime(lap.lap_time_ms)}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
