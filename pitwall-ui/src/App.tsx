import { useTelemetry } from "./hooks/useTelemetry";
import { SessionHeader } from "./components/SessionHeader";
import { SpeedPanel } from "./components/SpeedPanel";
import { InputBars } from "./components/InputBars";
import { TyreTemps } from "./components/TyreTemps";
import { ErsPanel } from "./components/ErsPanel";
import { LapTable } from "./components/LapTable";
import "./App.css";

function App() {
  const { frame, connected } = useTelemetry();

  if (!connected || !frame) {
    return (
      <div className="app waiting">
        <div className="waiting-content">
          <h1 className="pitwall-logo large">PITWALL</h1>
          <p>Waiting for telemetry connection...</p>
          <p className="hint">
            Run <code>pitwall live f1</code> to start streaming
          </p>
        </div>
      </div>
    );
  }

  if (!frame.session_active) {
    return (
      <div className="app waiting">
        <SessionHeader
          track=""
          sessionType=""
          sessionTime={0}
          connected={connected}
          sessionActive={false}
        />
        <div className="waiting-content">
          <h2>Connected</h2>
          <p>Waiting for session to start...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="app">
      <SessionHeader
        track={frame.track}
        sessionType={frame.session_type}
        sessionTime={frame.session_time}
        connected={connected}
        sessionActive={frame.session_active}
      />
      <main className="dashboard">
        <div className="col-left">
          <SpeedPanel speed={frame.speed} rpm={frame.rpm} gear={frame.gear} />
          <InputBars throttle={frame.throttle} brake={frame.brake} />
          <ErsPanel
            ersEnergy={frame.ers_store_energy}
            ersMode={frame.ers_deploy_mode}
            drs={frame.drs}
            drsAllowed={frame.drs_allowed}
            fuel={frame.fuel_in_tank}
          />
        </div>
        <div className="col-right">
          <TyreTemps
            fl={frame.tyre_fl}
            fr={frame.tyre_fr}
            rl={frame.tyre_rl}
            rr={frame.tyre_rr}
          />
          <LapTable
            laps={frame.laps}
            currentLap={frame.current_lap}
            totalLaps={frame.total_laps}
            liveSector1Ms={frame.current_sector1_ms}
            liveSector2Ms={frame.current_sector2_ms}
          />
        </div>
      </main>
    </div>
  );
}

export default App;
