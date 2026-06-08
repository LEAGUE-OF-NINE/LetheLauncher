import type { SyncPhase } from "../types/launcher";

interface StatusPanelProps {
  phase: SyncPhase;
  message: string;
}

export function StatusPanel({ phase, message }: StatusPanelProps) {
  const getPhaseStyles = (): string => {
    switch (phase) {
      case "checking":
      case "downloading":
        return "animate-pulse-glow text-red-400";
      case "complete":
        return "text-amber-400";
      case "error":
        return "text-red-500";
      case "manifest":
        return "text-amber-300/80";
      default:
        return "text-stone-400";
    }
  };

  return (
    <div className="flex flex-col items-center gap-2 animate-fade-in">
      {/* Thin red divider */}
      <div className="w-32 h-px bg-gradient-to-r from-transparent via-red-800 to-transparent" />
      {/* Status message */}
      <p
        className={`text-sm font-medium tracking-wide uppercase transition-colors duration-500 ${getPhaseStyles()}`}
      >
        {message}
      </p>
    </div>
  );
}
