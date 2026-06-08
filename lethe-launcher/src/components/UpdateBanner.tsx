import type { UpdateInfo } from "../types/updater";

interface UpdateBannerProps {
  updateInfo: UpdateInfo;
  isDownloading: boolean;
  downloadError: string | null;
  onApplyUpdate: () => void;
  onDismiss: () => void;
}

export function UpdateBanner({
  updateInfo,
  isDownloading,
  downloadError,
  onApplyUpdate,
  onDismiss,
}: UpdateBannerProps) {
  return (
    <div
      className="animate-fade-in border border-amber-900/50 p-5 max-w-md text-center"
      style={{ background: "rgba(120, 53, 15, 0.12)" }}
    >
      <p className="text-amber-400 font-medium mb-1 uppercase tracking-wider text-xs">
        Update Available
      </p>
      <p className="text-amber-300/80 text-sm mb-1">v{updateInfo.version}</p>
      {updateInfo.notes && (
        <p className="text-amber-500/40 text-xs mb-3 whitespace-pre-line max-h-20 overflow-y-auto">
          {updateInfo.notes}
        </p>
      )}

      {downloadError && (
        <p className="text-red-400/60 text-xs mb-3 font-mono">
          {downloadError}
        </p>
      )}

      <div className="flex items-center justify-center gap-4">
        <button
          onClick={onApplyUpdate}
          disabled={isDownloading}
          className="px-6 py-1.5 border border-amber-700/50 text-amber-400 hover:bg-amber-900/20 
            uppercase tracking-widest text-xs font-medium transition-colors duration-300 cursor-pointer
            disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isDownloading ? (
            <span className="flex items-center gap-2">
              <span className="inline-block w-3 h-3 border border-amber-500 border-t-transparent rounded-full animate-spin" />
              Downloading...
            </span>
          ) : (
            "Update"
          )}
        </button>
        <button
          onClick={onDismiss}
          disabled={isDownloading}
          className="text-stone-600 hover:text-stone-400 text-xs uppercase tracking-wider transition-colors cursor-pointer"
        >
          Later
        </button>
      </div>
    </div>
  );
}
