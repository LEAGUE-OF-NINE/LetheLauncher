import type { ModInfo } from '../types/launcher';

interface ModListProps {
  mods: ModInfo[];
  onToggleMod: (name: string) => void;
  onEnableAll: () => void;
  onDisableAll: () => void;
}

export function ModList({ mods, onToggleMod, onEnableAll, onDisableAll }: ModListProps) {
  const enabledCount = mods.filter((m) => m.enabled).length;

  return (
    <div className="flex flex-col gap-4 animate-fade-in">
      {/* Batch controls */}
      <div className="flex items-center justify-between">
        <p className="text-stone-500 text-[10px] uppercase tracking-[0.2em]">
          {mods.length} mod{mods.length !== 1 ? 's' : ''} ({enabledCount} enabled)
        </p>
        <div className="flex gap-2">
          <button
            onClick={onEnableAll}
            className="px-3 py-1 text-[10px] uppercase tracking-[0.15em] text-stone-500 border border-stone-800 hover:border-emerald-800 hover:text-emerald-400 transition-colors cursor-pointer"
            disabled={enabledCount === mods.length}
          >
            Enable All
          </button>
          <button
            onClick={onDisableAll}
            className="px-3 py-1 text-[10px] uppercase tracking-[0.15em] text-stone-500 border border-stone-800 hover:border-red-800 hover:text-red-400 transition-colors cursor-pointer"
            disabled={enabledCount === 0}
          >
            Disable All
          </button>
        </div>
      </div>

      {mods.length === 0 ? (
        <p className="text-stone-700 text-xs italic py-4 text-center">
          No mods found in BepInEx/plugins/Lethe/mods/
        </p>
      ) : (
        <div className="space-y-1.5 max-h-[60vh] overflow-y-auto pr-1">
          {mods.map((mod) => (
            <div
              key={mod.name}
              className="flex items-center justify-between py-2 px-3 rounded"
              style={{
                background: mod.enabled
                  ? 'rgba(6, 78, 59, 0.08)'
                  : 'rgba(120, 113, 108, 0.03)',
              }}
            >
              <span
                className={`text-sm ${
                  mod.enabled ? 'text-stone-300' : 'text-stone-600'
                }`}
              >
                {mod.name}
              </span>
              <button
                onClick={() => onToggleMod(mod.name)}
                className={`relative w-10 h-5 rounded-full transition-colors duration-200 cursor-pointer ${
                  mod.enabled ? 'bg-emerald-800' : 'bg-stone-700'
                }`}
              >
                <div
                  className={`absolute top-0.5 w-4 h-4 rounded-full bg-stone-200 transition-transform duration-200 ${
                    mod.enabled ? 'translate-x-5' : 'translate-x-0.5'
                  }`}
                />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
