import { invoke } from "@tauri-apps/api/core";

/** Hides the floating capture widget window entirely. Showing it again
 * is tray-only (see `tray.rs`'s "Toggle Widget" menu item) — the widget
 * never needs to show itself. */
export async function hideWidget(): Promise<void> {
  await invoke("hide_widget");
}

/**
 * Resizes the widget between its two in-window states (TASK-014's
 * bugfix): a small draggable "dot" (the resting default) and the full
 * expanded panel. Resizing is done from Rust (`commands::widget::
 * set_widget_expanded`) rather than calling `@tauri-apps/api/window`'s
 * resize API directly — sidesteps needing a window-resize capability
 * grant, the same reasoning `hide_widget` already established (see
 * DECISIONS.md, TASK-011).
 */
export async function setWidgetExpanded(expanded: boolean): Promise<void> {
  await invoke("set_widget_expanded", { expanded });
}
