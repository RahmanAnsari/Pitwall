interface Props {
  track: string;
  sessionType: string;
  sessionTime: number;
  connected: boolean;
  sessionActive: boolean;
}

function formatSessionTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function SessionHeader({ track, sessionType, sessionTime, connected, sessionActive }: Props) {
  return (
    <header className="session-header">
      <div className="header-left">
        <span className="pitwall-logo">PITWALL</span>
        <span className={`status-dot ${connected ? "connected" : "disconnected"}`} />
        <span className="status-text">{connected ? "LIVE" : "OFFLINE"}</span>
      </div>
      {sessionActive && (
        <div className="header-center">
          <span className="track-name">{track.toUpperCase().replace("_", " ")}</span>
          <span className="session-type">{sessionType}</span>
        </div>
      )}
      <div className="header-right">
        <span className="session-time">{formatSessionTime(sessionTime)}</span>
      </div>
    </header>
  );
}
