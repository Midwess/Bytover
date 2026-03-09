import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export interface UpdateStatus {
  available: boolean;
  version: string | null;
  release_notes: string | null;
}

export interface UpdateProgress {
  downloaded: number;
  total: number;
}

export async function checkForUpdate(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>("check_for_update");
}

export async function installUpdate(): Promise<void> {
  return invoke("install_update");
}

export function onUpdateProgress(
  callback: (progress: UpdateProgress) => void
): Promise<UnlistenFn> {
  return listen<UpdateProgress>("update-progress", (event) => {
    callback(event.payload);
  });
}

export function onUpdateFinished(callback: () => void): Promise<UnlistenFn> {
  return listen("update-finished", () => {
    callback();
  });
}

export function onUpdateError(callback: (error: string) => void): Promise<UnlistenFn> {
  return listen<string>("update-error", (event) => {
    callback(event.payload);
  });
}
