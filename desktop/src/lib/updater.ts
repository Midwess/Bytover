import { invoke } from "@tauri-apps/api/core";

export interface UpdateStatus {
  available: boolean;
  version: string | null;
  release_notes: string | null;
  is_critical: boolean;
  store_url: string | null;
}

export async function checkForUpdate(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>("check_for_update");
}
