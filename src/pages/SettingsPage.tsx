import { useEffect, useState, type FormEvent } from "react";
import { EmptyState } from "../components/ui/EmptyState";
import { StatusPill } from "../components/ui/StatusPill";
import { getLocalStorageStatus } from "../services/storage";
import { clearApiKey, hasApiKey, saveApiKey, testApiKeyConnection } from "../services/settings";
import { getErrorMessage } from "../utils/errorMessage";

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

// Whether a key is currently saved — the frontend never learns the key
// itself, only this (see docs/architecture.md §12, services/settings.ts).
type KeyState = { state: "loading" } | { state: "unset" } | { state: "saved" };

// Independent of `KeyState`: the last "Test connection" attempt's
// outcome, cleared whenever the saved key changes (a stale "it works"
// after clearing/replacing the key would be misleading).
type TestState = { state: "idle" } | { state: "testing" } | { state: "ok" } | { state: "error"; message: string };

/** Settings: local-storage status (TASK-004) plus the OpenAI API key
 * (TASK-016) — save/replace/clear it and test that it actually works.
 * No other settings exist yet. */
export function SettingsPage() {
  const [storage, setStorage] = useState<StorageState>({ state: "checking" });
  const [keyState, setKeyState] = useState<KeyState>({ state: "loading" });
  const [keyInput, setKeyInput] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [clearing, setClearing] = useState(false);
  const [testState, setTestState] = useState<TestState>({ state: "idle" });

  useEffect(() => {
    getLocalStorageStatus()
      .then(() => setStorage({ state: "ok" }))
      .catch((error) => setStorage({ state: "error", message: String(error) }));
  }, []);

  useEffect(() => {
    void refreshKeyState();
  }, []);

  async function refreshKeyState() {
    try {
      const saved = await hasApiKey();
      setKeyState({ state: saved ? "saved" : "unset" });
    } catch {
      // A failure here means the credential store itself is unavailable
      // (see AppError::Credential) — treat as "unset" so the user can
      // still see the form and retry, rather than a dead page.
      setKeyState({ state: "unset" });
    }
  }

  async function handleSave(event: FormEvent) {
    event.preventDefault();
    if (saving) return;
    setSaving(true);
    setSaveError(null);
    try {
      await saveApiKey(keyInput);
      setKeyInput("");
      setTestState({ state: "idle" });
      setKeyState({ state: "saved" });
    } catch (error) {
      setSaveError(getErrorMessage(error));
    } finally {
      setSaving(false);
    }
  }

  async function handleClear() {
    if (clearing) return;
    setClearing(true);
    try {
      await clearApiKey();
      setTestState({ state: "idle" });
      setKeyState({ state: "unset" });
    } catch (error) {
      setSaveError(getErrorMessage(error));
    } finally {
      setClearing(false);
    }
  }

  async function handleTest() {
    setTestState({ state: "testing" });
    try {
      await testApiKeyConnection();
      setTestState({ state: "ok" });
    } catch (error) {
      setTestState({ state: "error", message: getErrorMessage(error) });
    }
  }

  return (
    <div className="settings-page">
      <EmptyState
        title="Settings"
        description="More application settings will be available here in a future update."
      />

      <section className="settings-section" aria-label="AI">
        <h2 className="settings-section__title">AI</h2>
        <p className="field__hint settings-section__intro">
          Your OpenAI API key is stored in the Windows Credential Manager —
          never in GoLive's own database or any file it writes. It's used to
          generate and export each process's structured documentation.
        </p>

        {keyState.state === "loading" && <p className="projects-status">Checking…</p>}

        {keyState.state === "unset" && (
          <form className="dialog__body" onSubmit={handleSave}>
            <div className="field">
              <label className="field__label" htmlFor="api-key">
                OpenAI API key
              </label>
              <input
                id="api-key"
                type="password"
                autoComplete="off"
                className="field__input"
                value={keyInput}
                onChange={(event) => setKeyInput(event.target.value)}
                placeholder="sk-…"
                disabled={saving}
                required
              />
            </div>
            {saveError && (
              <p className="dialog__error" role="alert">
                {saveError}
              </p>
            )}
            <div className="dialog__footer">
              <button type="submit" className="button button--primary" disabled={saving}>
                {saving ? "Saving…" : "Save"}
              </button>
            </div>
          </form>
        )}

        {keyState.state === "saved" && (
          <div className="settings-key-saved">
            <StatusPill tone="ok" label="API key saved" />
            <div className="settings-key-saved__actions">
              <button type="button" className="button" onClick={() => void handleTest()} disabled={testState.state === "testing"}>
                {testState.state === "testing" ? "Testing…" : "Test connection"}
              </button>
              <button
                type="button"
                className="button button--danger"
                onClick={() => void handleClear()}
                disabled={clearing}
              >
                {clearing ? "Clearing…" : "Clear"}
              </button>
            </div>
            {testState.state === "ok" && (
              <p className="settings-key-saved__feedback settings-key-saved__feedback--ok">
                Connected — the key works.
              </p>
            )}
            {testState.state === "error" && (
              <p className="settings-key-saved__feedback settings-key-saved__feedback--error" role="alert">
                {testState.message}
              </p>
            )}
            {saveError && (
              <p className="settings-key-saved__feedback settings-key-saved__feedback--error" role="alert">
                {saveError}
              </p>
            )}
          </div>
        )}
      </section>

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
