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

