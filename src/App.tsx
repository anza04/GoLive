import { useEffect, useState } from "react";
import { checkFoundationStatus } from "./services/foundation";
import "./App.css";

type BackendStatus =
  | { state: "checking" }
  | { state: "ok"; message: string }
  | { state: "error"; message: string };

function App() {
  const [status, setStatus] = useState<BackendStatus>({ state: "checking" });

  useEffect(() => {
    checkFoundationStatus()
      .then((message) => setStatus({ state: "ok", message }))
      .catch((error) =>
        setStatus({ state: "error", message: String(error) }),
      );
  }, []);

  return (
    <main className="shell">
      <div className="card">
        <h1>GoLive</h1>
        <p className="tagline">Project foundation ready.</p>
        <div className={`status status-${status.state}`}>
          {status.state === "checking" && "Checking backend connection…"}
          {status.state === "ok" && status.message}
          {status.state === "error" &&
            `Backend connection failed: ${status.message}`}
        </div>
      </div>
    </main>
  );
}

export default App;
