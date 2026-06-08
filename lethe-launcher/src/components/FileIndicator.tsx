import type { SyncPhase } from "../types/launcher";

interface FileIndicatorProps {
  currentFile: string;
  phase: SyncPhase;
}

export function FileIndicator({ currentFile, phase }: FileIndicatorProps) {
  if (!currentFile || phase === "idle" || phase === "complete") {
    return null;
  }

  const displayName = currentFile.includes("/")
    ? currentFile.split("/").pop() || currentFile
    : currentFile.includes("\\")
      ? currentFile.split("\\").pop() || currentFile
      : currentFile;

  const isChecking = phase === "checking";
  const prefix = isChecking ? "CHECKING" : "DOWNLOADING";

  return (
    <div
      key={currentFile}
      className="animate-slide-left text-xs flex items-center gap-2 font-mono"
    >
      <span className="inline-block w-1.5 h-1.5 bg-red-700 animate-pulse" />
      <span className="text-stone-600 tracking-wider">{prefix}</span>
      <span className="text-stone-500 truncate max-w-xs">{displayName}</span>
    </div>
  );
}
