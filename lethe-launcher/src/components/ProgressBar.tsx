interface ProgressBarProps {
  percent: number;
  bytesProcessed: number;
  totalBytes: number;
  label?: string;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

export function ProgressBar({
  percent,
  bytesProcessed,
  totalBytes,
  label,
}: ProgressBarProps) {
  const isActive = percent > 0 && percent < 100;

  return (
    <div className="w-full max-w-lg animate-fade-in">
      {label && (
        <p className="text-xs text-stone-500 mb-2 text-center uppercase tracking-wider">
          {label}
        </p>
      )}
      <div className="relative h-10 bg-stone-900 rounded-sm overflow-hidden border border-stone-800">
        {/* Fill bar - crimson gradient */}
        <div
          className={`absolute inset-y-0 left-0 transition-all duration-500 ease-out ${
            isActive ? "animate-industrial-shimmer" : ""
          }`}
          style={{
            width: `${Math.min(percent, 100)}%`,
            background:
              "linear-gradient(90deg, #7F1D1D 0%, #991B1B 30%, #DC2626 60%, #EF4444 100%)",
            boxShadow:
              percent > 0 ? "inset 0 1px 0 rgba(255,255,255,0.05)" : "none",
          }}
        />
        {/* Percentage */}
        <span className="absolute inset-0 flex items-center justify-center text-sm font-bold text-stone-200 drop-shadow-[0_1px_3px_rgba(0,0,0,0.8)] z-10">
          {percent.toFixed(1)}%
        </span>
      </div>
      {(bytesProcessed > 0 || totalBytes > 0) && (
        <p className="text-xs text-stone-600 mt-2 text-center tabular-nums font-mono">
          {formatBytes(bytesProcessed)} / {formatBytes(totalBytes)}
        </p>
      )}
    </div>
  );
}
