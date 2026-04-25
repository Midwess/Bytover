import { useEffect, useState } from "react"
import { openUrl } from "@tauri-apps/plugin-opener"
import { invoke } from "@tauri-apps/api/core"
import { Button } from "@/components/ui/button"
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

interface ForceUpdateOverlayProps {
  status: UpdateStatus
}

export function ForceUpdateOverlay({ status }: ForceUpdateOverlayProps) {
  const [busy, setBusy] = useState(false)

  const handleUpdate = async () => {
    if (busy) return
    setBusy(true)
    try {
      if (status.store_url) {
        await openUrl(status.store_url)
      } else {
        await invoke("show_settings_with_tab", { tab: "updates" })
      }
    } catch (err) {
      console.error("[force-update] action failed:", err)
    } finally {
      setBusy(false)
    }
  }

  const versionLabel = status.version ?? "the latest version"

  return (
    <div className="absolute inset-0 z-30 bg-card/95 backdrop-blur-sm flex flex-col items-stretch px-4 py-5 select-none pointer-events-auto">
      <div className="flex-1 flex flex-col items-center justify-center gap-2 text-center">
        <p className="text-[13px] text-white/85 font-medium">Update required to send</p>
        <p className="text-[11px] text-white/55 leading-snug">
          A new version ({versionLabel}) is available. Update Bytover to keep sending files.
        </p>
      </div>
      <Button
        onClick={handleUpdate}
        disabled={busy}
        className="w-full h-[28px] px-4 text-[11px] font-semibold bg-bluePrimary text-white hover:bg-bluePrimary/90 border-none rounded-md shadow-none"
      >
        {busy ? "Opening…" : status.store_url ? "Open App Store" : "Download update"}
      </Button>
    </div>
  )
}
