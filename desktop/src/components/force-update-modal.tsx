import { openUrl } from "@tauri-apps/plugin-opener";

interface ForceUpdateModalProps {
  version: string | null;
  storeUrl: string | null;
}

export function ForceUpdateModal({ version, storeUrl }: ForceUpdateModalProps) {
  const handleOpenStore = async () => {
    if (!storeUrl) return;
    try {
      await openUrl(storeUrl);
    } catch (err) {
      console.error("[force-update] failed to open store url:", err);
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="force-update-title"
      className="fixed inset-0 z-[9999] flex items-center justify-center bg-black/80 backdrop-blur-sm"
      data-tauri-drag-region
    >
      <div className="max-w-md mx-4 rounded-2xl bg-zinc-900 border border-white/10 p-8 shadow-2xl text-white">
        <div className="flex flex-col items-center text-center gap-4">
          <div className="w-14 h-14 rounded-full bg-white/10 flex items-center justify-center">
            <svg
              className="w-7 h-7 text-white/80"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" />
            </svg>
          </div>
          <h2 id="force-update-title" className="text-xl font-semibold tracking-tight">
            Update required
          </h2>
          <p className="text-sm text-white/60 leading-relaxed">
            Bytover
            {version ? ` ${version}` : ""} is required to continue. Please update via the App Store to keep using the app.
          </p>
          <button
            onClick={handleOpenStore}
            disabled={!storeUrl}
            className="mt-2 w-full h-11 rounded-full bg-white text-black font-semibold text-[15px] transition-colors hover:bg-white/90 active:scale-[0.98] disabled:opacity-40 disabled:cursor-not-allowed"
          >
            Open App Store
          </button>
          {!storeUrl && (
            <p className="text-[11px] text-white/40">
              No store link provided. Please close and reopen the app once your connection is restored.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
