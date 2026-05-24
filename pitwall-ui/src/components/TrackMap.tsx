import { useEffect, useState } from "react";

interface Props {
  track: string;
}

export function TrackMap({ track }: Props) {
  const [svg, setSvg] = useState<string | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    if (!track) {
      setSvg(null);
      return;
    }

    setError(false);
    const name = track.toLowerCase().replace(/\s+/g, "_");

    fetch(`http://localhost:8765/circuits/${name}`)
      .then((res) => {
        if (!res.ok) throw new Error("not found");
        return res.text();
      })
      .then(setSvg)
      .catch(() => setError(true));
  }, [track]);

  return (
    <div className="panel track-map-panel">
      <div className="panel-title">CIRCUIT</div>
      <div className="track-map-container">
        {svg && (
          <div
            className="track-map-svg"
            dangerouslySetInnerHTML={{ __html: svg }}
          />
        )}
        {error && <span className="track-map-fallback">No map available</span>}
        {!svg && !error && <span className="track-map-fallback">Loading...</span>}
      </div>
    </div>
  );
}
