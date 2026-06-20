import { useState } from 'react';
import { StatusPanel } from "./components/StatusPanel";
import { ProgressBar } from "./components/ProgressBar";
import { FileIndicator } from "./components/FileIndicator";
import { LoginButton, UserBadge } from "./components/LoginButton";
import { UpdateBanner } from "./components/UpdateBanner";
import { SettingsPanel, SettingsButton } from "./components/SettingsPanel";
import { ModList } from "./components/ModList";
import { useLauncher } from "./hooks/useLauncher";
import { useAuth } from "./hooks/useAuth";
import { useUpdater } from "./hooks/useUpdater";
import { useSettings } from "./hooks/useSettings";
import { useEffect, useRef } from "react";

function App() {
  const [modsOpen, setModsOpen] = useState(false);
  const {
    auth,
    isAuthenticated,
    isLoading: authLoading,
    loginError,
    startLogin,
    logout,
  } = useAuth();
  const {
    updateAvailable,
    updateInfo,
    isDownloading: updateDownloading,
    downloadError: updateDownloadError,
    applyUpdate,
    dismissUpdate,
  } = useUpdater();
  const {
    settings,
    mods,
    isOpen: settingsOpen,
    setIsOpen: setSettingsOpen,
    toggleValidation,
    toggleMod,
    enableAllMods,
    disableAllMods,
  } = useSettings();
  const syncStarted = useRef(false);
  const {
    phase,
    statusMessage,
    checkProgress,
    downloadProgress,
    currentFile,
    error,
    startSync,
    launchGame,
  } = useLauncher(auth?.token ?? null);

  // Start sync once authenticated (unless validation is skipped)
  useEffect(() => {
    if (isAuthenticated && !syncStarted.current && !settings.skipValidation) {
      syncStarted.current = true;
      const timer = setTimeout(() => {
        startSync();
      }, 800);
      return () => clearTimeout(timer);
    }
  }, [isAuthenticated, startSync, settings.skipValidation]);

  const isDownloadPhase = phase === "downloading" && downloadProgress;
  const isCheckPhase = phase === "checking" && checkProgress;

  const progressPercent = isDownloadPhase
    ? downloadProgress.percent
    : isCheckPhase
      ? checkProgress.percent
      : 0;

  const bytesProcessed = isDownloadPhase
    ? downloadProgress.bytes_downloaded
    : isCheckPhase
      ? checkProgress.bytes_processed
      : 0;

  const totalBytes = isDownloadPhase
    ? downloadProgress.total_bytes
    : isCheckPhase
      ? checkProgress.total_bytes
      : 0;

  const isActive =
    phase === "checking" || phase === "downloading" || phase === "manifest";

  const displayMessage =
    phase === "checking"
      ? checkProgress
        ? `Checking files (${checkProgress.files_checked} of ${checkProgress.total_files})`
        : "Checking files..."
      : phase === "downloading"
        ? "Downloading files..."
        : statusMessage;

  const enabledCount = mods.filter((m) => m.enabled).length;

  return (
    <div
      className="min-h-screen flex flex-col items-center justify-center p-8 gap-8 select-none"
      style={{
        background:
          "linear-gradient(135deg, #0a0a0b 0%, #111114 30%, #0d0d10 60%, #0a0a0c 100%)",
      }}
    >
      {/* Subtle noise/grunge overlay */}
      <div
        className="fixed inset-0 pointer-events-none opacity-[0.03]"
        style={{
          backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E")`,
        }}
      />

      {/* Top-right buttons: Mods pill + Settings gear */}
      <div className="fixed top-4 right-4 z-40 flex items-center gap-3">
        {mods.length > 0 && (
          <button
            onClick={() => setModsOpen(true)}
            className="flex items-center gap-1.5 px-2.5 py-1 border border-stone-800 hover:border-red-900/50 transition-colors cursor-pointer"
            style={{ background: "rgba(0,0,0,0.3)" }}
          >
            <span className="text-stone-500 text-[10px] uppercase tracking-[0.15em]">
              Mods
            </span>
            <span className="text-stone-700 text-[10px]">
              {enabledCount}/{mods.length}
            </span>
          </button>
        )}
        <SettingsButton onClick={() => setSettingsOpen(true)} />
      </div>

      {/* Mods modal */}
      {modsOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm animate-fade-in">
          <div
            className="border border-stone-800 p-6 w-[28rem] max-h-[85vh] flex flex-col"
            style={{ background: "#111114" }}
          >
            <div className="flex items-center justify-between mb-4 shrink-0">
              <h2 className="text-stone-300 text-sm uppercase tracking-[0.2em] font-medium">
                Mods
              </h2>
              <button
                onClick={() => setModsOpen(false)}
                className="text-stone-600 hover:text-stone-400 transition-colors cursor-pointer text-lg leading-none"
              >
                ×
              </button>
            </div>

            <div className="w-full h-px bg-stone-800 mb-4 shrink-0" />

            <div className="flex-1 overflow-hidden">
              <ModList
                mods={mods}
                onToggleMod={toggleMod}
                onEnableAll={enableAllMods}
                onDisableAll={disableAllMods}
              />
            </div>

            <div className="w-full h-px bg-stone-800 my-4 shrink-0" />

            <button
              onClick={() => setModsOpen(false)}
              className="w-full py-2 border border-stone-800 text-stone-500 hover:text-stone-300 hover:border-stone-700
                uppercase tracking-[0.15em] text-[10px] transition-colors cursor-pointer shrink-0"
            >
              Close
            </button>
          </div>
        </div>
      )}

      {/* Settings panel modal */}
      <SettingsPanel
        isOpen={settingsOpen}
        skipValidation={settings.skipValidation}
        onToggleValidation={toggleValidation}
        onClose={() => setSettingsOpen(false)}
      />

      <div className="fixed top-0 left-0 w-24 h-px bg-gradient-to-r from-red-900/40 to-transparent" />
      <div className="fixed top-0 left-0 w-px h-24 bg-gradient-to-b from-red-900/40 to-transparent" />
      <div className="fixed bottom-0 right-0 w-24 h-px bg-gradient-to-l from-red-900/40 to-transparent" />
      <div className="fixed bottom-0 right-0 w-px h-24 bg-gradient-to-t from-red-900/40 to-transparent" />

      {/* Title always visible */}
      <div className="flex flex-col items-center gap-4 animate-fade-in">
        <h1
          className="text-4xl font-extrabold tracking-widest uppercase"
          style={{
            color: "#DC2626",
            letterSpacing: "0.15em",
            textShadow:
              "0 0 20px rgba(220, 38, 38, 0.3), 0 2px 4px rgba(0,0,0,0.8)",
          }}
        >
          Lethe Launcher
        </h1>
        <div className="w-32 h-px bg-gradient-to-r from-transparent via-red-800 to-transparent" />
      </div>

      {/* Login section */}
      {!isAuthenticated ? (
        <div className="flex flex-col items-center gap-4 animate-fade-in">
          <p className="text-stone-400 text-sm uppercase tracking-wider">
            Sign in to continue
          </p>
          <LoginButton onClick={startLogin} isLoading={authLoading} />
          {loginError && (
            <p className="text-red-500/60 text-xs mt-2 max-w-xs text-center font-mono">
              {loginError}
            </p>
          )}
        </div>
      ) : (
        <>
          {/* Update banner */}
          {updateAvailable && updateInfo && (
            <UpdateBanner
              updateInfo={updateInfo}
              isDownloading={updateDownloading}
              downloadError={updateDownloadError}
              onApplyUpdate={applyUpdate}
              onDismiss={dismissUpdate}
            />
          )}

          {/* User badge */}
          <UserBadge onLogout={logout} />

          {/* Show launch button immediately when validation is skipped */}
          {isAuthenticated && settings.skipValidation && phase === "idle" && (
            <div className="flex flex-col items-center gap-4 animate-fade-in">
              <div className="w-32 h-px bg-gradient-to-r from-transparent via-amber-800 to-transparent" />
              <p className="text-amber-500/60 text-[10px] uppercase tracking-[0.2em]">
                File validation disabled
              </p>
              <button
                onClick={() => launchGame()}
                className="px-10 py-3 border border-red-800/60 text-red-400 hover:bg-red-900/20 hover:border-red-700/60
                  uppercase tracking-[0.2em] text-sm font-medium transition-all duration-300 cursor-pointer"
                style={{ background: "rgba(127, 29, 29, 0.1)" }}
              >
                Launch Game
              </button>
            </div>
          )}
          {isActive && (
            <>
              <StatusPanel phase={phase} message={displayMessage} />
              <ProgressBar
                percent={progressPercent}
                bytesProcessed={bytesProcessed}
                totalBytes={totalBytes}
              />
              <FileIndicator currentFile={currentFile} phase={phase} />
            </>
          )}

          {/* Ready state after sync */}
          {phase === "complete" && (
            <div className="flex flex-col items-center gap-4 animate-fade-in">
              <div className="w-32 h-px bg-gradient-to-r from-transparent via-emerald-800 to-transparent" />
              <p className="text-emerald-400/80 text-sm uppercase tracking-wider">
                {statusMessage}
              </p>
              <button
                onClick={() => launchGame()}
                className="px-10 py-3 border border-emerald-800/60 text-emerald-400 hover:bg-emerald-900/20 hover:border-emerald-700/60
                  uppercase tracking-[0.2em] text-sm font-medium transition-all duration-300 cursor-pointer"
                style={{ background: "rgba(6, 78, 59, 0.1)" }}
              >
                Launch Game
              </button>
            </div>
          )}

          {/* Launch button during sync - subtle, secondary option */}
          {isActive && (
            <>
              <button
                onClick={() => launchGame()}
                className="text-stone-600 hover:text-stone-400 text-[10px] uppercase tracking-[0.2em] transition-colors cursor-pointer"
              >
                Skip check &amp; launch
              </button>
            </>
          )}

          {/* Launching state */}
          {phase === "launching" && (
            <p className="text-red-400/60 text-xs uppercase tracking-widest animate-fade-in">
              Launching...
            </p>
          )}

          {/* Error state */}
          {phase === "error" && (
            <div
              className="animate-fade-in border border-red-900/50 p-6 max-w-md text-center"
              style={{ background: "rgba(127, 29, 29, 0.15)" }}
            >
              <p className="text-red-400 font-medium mb-2 uppercase tracking-wider text-sm">
                Error
              </p>
              <p className="text-red-300/60 text-xs mb-4 break-all font-mono">
                {error}
              </p>
              <div className="flex items-center justify-center gap-3">
                <button
                  onClick={startSync}
                  className="px-6 py-1.5 border border-red-800/50 text-red-400 hover:bg-red-900/30 uppercase tracking-widest text-xs font-medium transition-colors cursor-pointer"
                >
                  Retry
                </button>
                <button
                  onClick={() => launchGame()}
                  className="text-stone-600 hover:text-stone-400 text-[10px] uppercase tracking-[0.2em] transition-colors cursor-pointer"
                >
                  Launch anyway
                </button>
              </div>
            </div>
          )}
        </>
      )}

      {/* Footer */}
      <div className="absolute bottom-4 text-[10px] text-stone-800 uppercase tracking-[0.2em]">
        Lethe Launcher v0.1.0
      </div>
    </div>
  );
}

export default App;
