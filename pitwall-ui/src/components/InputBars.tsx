interface Props {
  throttle: number;
  brake: number;
}

export function InputBars({ throttle, brake }: Props) {
  const throttlePct = Math.min(throttle * 100, 100);
  const brakePct = Math.min(brake * 100, 100);

  return (
    <div className="panel input-bars">
      <div className="input-row">
        <span className="input-label">THR</span>
        <div className="bar-track">
          <div className="bar-fill throttle" style={{ width: `${throttlePct}%` }} />
        </div>
        <span className="input-value">{Math.round(throttlePct)}%</span>
      </div>
      <div className="input-row">
        <span className="input-label">BRK</span>
        <div className="bar-track">
          <div className="bar-fill brake" style={{ width: `${brakePct}%` }} />
        </div>
        <span className="input-value">{Math.round(brakePct)}%</span>
      </div>
    </div>
  );
}
