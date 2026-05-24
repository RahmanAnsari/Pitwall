interface Props {
  speed: number;
  rpm: number;
  gear: number;
}

export function SpeedPanel({ speed, rpm, gear }: Props) {
  const maxRpm = 15000;
  const rpmPct = Math.min((rpm / maxRpm) * 100, 100);

  return (
    <div className="panel speed-panel">
      <div className="speed-value">{speed}</div>
      <div className="speed-unit">KM/H</div>
      <div className="gear-display">
        {gear === 0 ? "N" : gear === -1 ? "R" : gear}
      </div>
      <div className="rpm-bar-container">
        <div
          className="rpm-bar"
          style={{ width: `${rpmPct}%` }}
          data-high={rpmPct > 90 ? "true" : undefined}
        />
        <span className="rpm-label">{rpm} RPM</span>
      </div>
    </div>
  );
}
