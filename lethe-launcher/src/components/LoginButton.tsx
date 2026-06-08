interface LoginButtonProps {
  onClick: () => void;
  isLoading: boolean;
}

export function LoginButton({ onClick, isLoading }: LoginButtonProps) {
  return (
    <button
      onClick={onClick}
      disabled={isLoading}
      className="group relative px-8 py-3 border border-red-900/50 text-red-400/80 hover:text-red-300 hover:border-red-800/70 
        uppercase tracking-[0.2em] text-xs font-medium transition-all duration-300 cursor-pointer
        disabled:opacity-50 disabled:cursor-not-allowed"
      style={{ background: "rgba(127, 29, 29, 0.1)" }}
    >
      {isLoading ? (
        <span className="flex items-center gap-2">
          <span className="inline-block w-3 h-3 border border-red-500 border-t-transparent rounded-full animate-spin" />
          Waiting...
        </span>
      ) : (
        <span className="flex items-center gap-2">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="currentColor"
            className="text-red-400/60 group-hover:text-red-300/80 transition-colors"
          >
            <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.947 2.418-2.157 2.418z" />
          </svg>
          Login with Discord
        </span>
      )}
    </button>
  );
}

interface UserBadgeProps {
  onLogout: () => void;
}

export function UserBadge({ onLogout }: UserBadgeProps) {
  return (
    <div className="flex items-center gap-3 animate-fade-in">
      <div className="flex flex-col">
        <span className="text-xs text-stone-400 uppercase tracking-wider">
          Status
        </span>
        <span className="text-sm text-stone-200 font-medium">Logged In</span>
      </div>
      <button
        onClick={onLogout}
        className="ml-2 text-[10px] text-stone-600 hover:text-red-500 uppercase tracking-wider transition-colors cursor-pointer"
      >
        Logout
      </button>
    </div>
  );
}
