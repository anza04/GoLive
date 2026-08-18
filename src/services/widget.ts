import { invoke } from "@tauri-apps/api/core";

/** Hides the floating capture widget window (its own "×" button — see
 * `src/widget/Widget.tsx`). Showing it again is tray-only (see
 * `tray.rs`'s "Toggle Widget" menu item) — the widget never needs to
 * show itself. */
export async function hideWidget(): Promise<void> {
  await invoke("hide_widget");
}
