interface Props {
  fl: number;
  fr: number;
  rl: number;
  rr: number;
}

function tempColor(temp: number): string {
  if (temp < 80) return "#3b82f6"; // cold - blue
  if (temp < 100) return "#22c55e"; // optimal - green
  if (temp < 110) return "#eab308"; // warm - yellow
  return "#ef4444"; // hot - red
}

export function TyreTemps({ fl, fr, rl, rr }: Props) {
  return (
    <div className="panel tyre-panel">
      <div className="panel-title">TYRES °C</div>
      <div className="tyre-grid">
        <div className="tyre" style={{ borderColor: tempColor(fl) }}>
          <span>{fl}</span>
          <small>FL</small>
        </div>
        <div className="tyre" style={{ borderColor: tempColor(fr) }}>
          <span>{fr}</span>
          <small>FR</small>
        </div>
        <div className="tyre" style={{ borderColor: tempColor(rl) }}>
          <span>{rl}</span>
          <small>RL</small>
        </div>
        <div className="tyre" style={{ borderColor: tempColor(rr) }}>
          <span>{rr}</span>
          <small>RR</small>
        </div>
      </div>
    </div>
  );
}
