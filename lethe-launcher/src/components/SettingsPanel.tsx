interface SettingsPanelProps {
  isOpen: boolean;
  skipValidation: boolean;
  onToggleValidation: () => void;
  onClose: () => void;
}

export function SettingsPanel({
  isOpen,
  skipValidation,
  onToggleValidation,
  onClose,
}: SettingsPanelProps) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm animate-fade-in">
      <div
        className="border border-stone-800 p-6 w-80"
        style={{ background: "#111114" }}
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-stone-300 text-sm uppercase tracking-[0.2em] font-medium">
            Settings
          </h2>
          <button
            onClick={onClose}
            className="text-stone-600 hover:text-stone-400 transition-colors cursor-pointer text-lg leading-none"
          >
            ×
          </button>
        </div>

        <div className="w-full h-px bg-stone-800 mb-4" />

        {/* File validation toggle */}
        <div className="flex items-center justify-between mb-2">
          <div>
            <p className="text-stone-300 text-xs">Validate files on startup</p>
            <p className="text-stone-600 text-[10px] mt-0.5">
              {skipValidation
                ? "Files will not be checked. Game launches immediately."
                : "Checks all game files and downloads updates."}
            </p>
          </div>
          <button
            onClick={onToggleValidation}
            className={`relative w-10 h-5 rounded-full transition-colors duration-200 cursor-pointer ${
              skipValidation ? "bg-stone-700" : "bg-red-800"
            }`}
          >
            <div
              className={`absolute top-0.5 w-4 h-4 rounded-full bg-stone-200 transition-transform duration-200 ${
                skipValidation ? "translate-x-0.5" : "translate-x-5"
              }`}
            />
          </button>
        </div>

        <div className="w-full h-px bg-stone-800 my-4" />

        <button
          onClick={onClose}
          className="w-full py-2 border border-stone-800 text-stone-500 hover:text-stone-300 hover:border-stone-700 
            uppercase tracking-[0.15em] text-[10px] transition-colors cursor-pointer"
        >
          Close
        </button>
      </div>
    </div>
  );
}

interface SettingsButtonProps {
  onClick: () => void;
}

export function SettingsButton({ onClick }: SettingsButtonProps) {
  return (
    <button
      onClick={onClick}
      className="text-stone-700 hover:text-stone-400 transition-colors cursor-pointer"
      title="Settings"
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
      >
        <circle cx="12" cy="12" r="3" />
        <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
      </svg>
    </button>
  );
}
