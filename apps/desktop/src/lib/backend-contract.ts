import type { desktopApi as tauriDesktopApi } from "./backend-tauri";

/** Compile-time parity contract shared by the mutually exclusive build adapters. */
export type DesktopApi = typeof tauriDesktopApi;
