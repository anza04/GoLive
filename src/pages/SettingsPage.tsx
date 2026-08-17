import { useEffect, useState } from "react";
import { EmptyState } from "../components/ui/EmptyState";
import { StatusPill } from "../components/ui/StatusPill";
import { getLocalStorageStatus } from "../services/storage";

type StorageState =
  | { state: "checking" }
  | { state: "ok" }
  | { state: "error"; message: string };

const LABEL: Record<StorageState["state"], string> = {
  checking: "Local storage: Checking…",
  ok: "Local storage: Ready",
  error: "Local storage: Unavailable",
};

const TONE: Record<StorageState["state"], "neutral" | "ok" | "error"> = {
  checking: "neutral",
  ok: "ok",
  error: "error",
};

/** Placeholder Settings area. No settings are implemented or persisted
 * yet — the one real thing here is a system-status readout proving local
 * SQLite persistence is working (TASK-004). */
export function SettingsPage() {
  const [storage, setStorage] = useState<StorageState>({ state: "checking" });

  useEffect(() => {
    getLocalStorageStatus()
      .then(() => setStorage({ state: "ok" }))
      .catch((error) => setStorage({ state: "error", message: String(error) }));
  }, []);

  return (
    <div className="settings-page">
      <EmptyState
        title="Settings"
        description="Application settings will be available here in a future update."
      />
      <section className="system-status" aria-label="System status">
        <h2 className="system-status__title">System</h2>
        <StatusPill
          tone={TONE[storage.state]}
          label={LABEL[storage.state]}
          detail={storage.state === "error" ? storage.message : undefined}
        />
      </section>
    </div>
  );
}
