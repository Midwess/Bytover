import { useEffect, useState } from "react"
import { openUrl } from "@tauri-apps/plugin-opener"
import { invoke } from "@tauri-apps/api/core"
import { checkForUpdate, UpdateStatus } from "@/lib/updater"

export function useForceUpdateStatus() {
  const [status, setStatus] = useState<UpdateStatus | null>(null)

  useEffect(() => {
    let mounted = true
    checkForUpdate()
      .then((s) => {
        if (mounted) setStatus(s)
      })
      .catch((err) => {
        console.error("[force-update] check failed:", err)
      })
    return () => {
      mounted = false
    }
  }, [])

  return status
}

export function formatUpdateLabel(status: UpdateStatus): string {
  const version = status.version
  if (!version) return "Update to continue"
  const normalized = version.startsWith("v") ? version : `v${version}`
  return `Update to ${normalized}`
}

export async function openForceUpdate(status: UpdateStatus) {
  try {
    if (status.store_url) {
      await openUrl(status.store_url)
    } else {
      await invoke("show_settings_with_tab", { tab: "updates" })
    }
  } catch (err) {
    console.error("[force-update] action failed:", err)
  }
}

