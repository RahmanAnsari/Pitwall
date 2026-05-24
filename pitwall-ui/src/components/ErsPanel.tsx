interface Props {
  ersEnergy: number;
  ersMode: number;
  drs: number;
  drsAllowed: number;
  fuel: number;
}

const ERS_MODES = ["None", "Medium", "Hotlap", "Overtake"];

export function ErsPanel({ ersEnergy, ersMode, drs, drsAllowed, fuel }: Props) {
  const maxErs = 4_000_000; // 4 MJ
  const ersPct = Math.min((ersEnergy / maxErs) * 100, 100);

  return (
    <div className="panel ers-panel">
      <div className="ers-row">
        <span className="ers-label">ERS</span>
        <div className="bar-track">
          <div className="bar-fill ers" style={{ width: `${ersPct}%` }} />
        </div>
        <span className="ers-mode">{ERS_MODES[ersMode] ?? "?"}</span>
      </div>
      <div className="ers-row">
        <span className="ers-label">DRS</span>
        <span className={`drs-status ${drs ? "active" : drsAllowed ? "available" : "off"}`}>
          {drs ? "OPEN" : drsAllowed ? "READY" : "OFF"}
        </span>
      </div>
      <div className="ers-row">
        <span className="ers-label">FUEL</span>
        <span className="fuel-value">{fuel.toFixed(2)} kg</span>
      </div>
    </div>
  );
}
