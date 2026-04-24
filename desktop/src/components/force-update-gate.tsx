import { useEffect, useState } from "react";
import { checkForUpdate, UpdateStatus } from "@/lib/updater";
import { ForceUpdateModal } from "./force-update-modal";

interface ForceUpdateGateProps {
  children: React.ReactNode;
}

export function ForceUpdateGate({ children }: ForceUpdateGateProps) {
  const [status, setStatus] = useState<UpdateStatus | null>(null);

  useEffect(() => {
    let mounted = true;
    checkForUpdate()
      .then((s) => {
        if (mounted) setStatus(s);
      })
      .catch((err) => {
        console.error("[force-update-gate] check failed:", err);
      });
    return () => {
      mounted = false;
    };
  }, []);

  return (
    <>
      {children}
      {status?.is_critical && (
        <ForceUpdateModal version={status.version} storeUrl={status.store_url} />
      )}
    </>
  );
}
