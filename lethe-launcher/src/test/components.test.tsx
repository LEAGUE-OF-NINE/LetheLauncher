import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StatusPanel } from '../components/StatusPanel';
import { ProgressBar } from '../components/ProgressBar';
import { FileIndicator } from '../components/FileIndicator';
import { LoginButton, UserBadge } from '../components/LoginButton';
import { UpdateBanner } from '../components/UpdateBanner';

describe('StatusPanel', () => {
  it('renders the title', () => {
    render(<StatusPanel phase="idle" message="Initializing..." />);
    expect(screen.getByText('Initializing...')).toBeInTheDocument();
  });

  it('shows checking message with pulse class', () => {
    render(<StatusPanel phase="checking" message="Checking 100 files..." />);
    const msg = screen.getByText('Checking 100 files...');
    expect(msg).toBeInTheDocument();
    expect(msg.className).toContain('animate-pulse-glow');
  });

  it('shows complete in green', () => {
    render(<StatusPanel phase="complete" message="Update complete!" />);
    const msg = screen.getByText('Update complete!');
    expect(msg.className).toContain('text-amber-400');
  });

  it('shows error in red', () => {
    render(<StatusPanel phase="error" message="Something failed" />);
    const msg = screen.getByText('Something failed');
    expect(msg.className).toContain('text-red-500');
  });
});

describe('ProgressBar', () => {
  it('renders percentage correctly', () => {
    render(<ProgressBar percent={42.5} bytesProcessed={425000} totalBytes={1000000} />);
    expect(screen.getByText('42.5%')).toBeInTheDocument();
  });

  it('formats bytes correctly for small sizes', () => {
    render(<ProgressBar percent={10} bytesProcessed={500} totalBytes={5000} />);
    expect(screen.getByText('500 B / 4.9 KB')).toBeInTheDocument();
  });

  it('formats bytes correctly for MB', () => {
    render(<ProgressBar percent={50} bytesProcessed={5242880} totalBytes={10485760} />);
    expect(screen.getByText('5.0 MB / 10.0 MB')).toBeInTheDocument();
  });

  it('hides byte text when both are zero', () => {
    render(<ProgressBar percent={0} bytesProcessed={0} totalBytes={0} />);
    expect(screen.queryByText(/B/)).not.toBeInTheDocument();
  });

  it('shows 100% at completion', () => {
    render(<ProgressBar percent={100} bytesProcessed={1000000} totalBytes={1000000} />);
    expect(screen.getByText('100.0%')).toBeInTheDocument();
  });

  it('renders label when provided', () => {
    render(<ProgressBar percent={30} bytesProcessed={300} totalBytes={1000} label="Downloading..." />);
    expect(screen.getByText('Downloading...')).toBeInTheDocument();
  });
});

describe('FileIndicator', () => {
  it('renders nothing when idle', () => {
    const { container } = render(<FileIndicator currentFile="" phase="idle" />);
    expect(container.firstChild).toBeNull();
  });

  it('renders nothing when complete', () => {
    const { container } = render(<FileIndicator currentFile="some/file.dll" phase="complete" />);
    expect(container.firstChild).toBeNull();
  });

  it('shows file name when checking', () => {
    render(<FileIndicator currentFile="BepInEx/core/0Harmony.dll" phase="checking" />);
    expect(screen.getByText('CHECKING')).toBeInTheDocument();
    expect(screen.getByText('0Harmony.dll')).toBeInTheDocument();
  });

  it('shows downloading prefix when downloading', () => {
    render(<FileIndicator currentFile="data/assets.bundle" phase="downloading" />);
    expect(screen.getByText('DOWNLOADING')).toBeInTheDocument();
    expect(screen.getByText('assets.bundle')).toBeInTheDocument();
  });

  it('handles backslash paths', () => {
    render(<FileIndicator currentFile="sub\\folder\\file.dll" phase="checking" />);
    expect(screen.getByText('file.dll')).toBeInTheDocument();
  });
});

describe('LoginButton', () => {
  it('renders login text', () => {
    render(<LoginButton onClick={() => {}} isLoading={false} />);
    expect(screen.getByText('Login with Discord')).toBeInTheDocument();
  });

  it('shows spinner when loading', () => {
    render(<LoginButton onClick={() => {}} isLoading={true} />);
    expect(screen.getByText('Waiting...')).toBeInTheDocument();
    expect(screen.queryByText('Login with Discord')).not.toBeInTheDocument();
  });

  it('disables button when loading', () => {
    render(<LoginButton onClick={() => {}} isLoading={true} />);
    expect(screen.getByRole('button')).toBeDisabled();
  });
});

describe('UserBadge', () => {
  it('shows logged in status', () => {
    render(<UserBadge onLogout={() => {}} />);
    expect(screen.getByText('Logged In')).toBeInTheDocument();
    expect(screen.getByText('Status')).toBeInTheDocument();
  });
});

describe('UpdateBanner', () => {
  const updateInfo = {
    version: '0.2.0',
    notes: 'Bug fixes and improvements',
    download_url: 'https://example.com/LimbusCompany.exe',
    sha256: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
  };

  it('shows update available', () => {
    render(
      <UpdateBanner
        updateInfo={updateInfo}
        isDownloading={false}
        downloadError={null}
        onApplyUpdate={() => {}}
        onDismiss={() => {}}
      />
    );
    expect(screen.getByText('Update Available')).toBeInTheDocument();
    expect(screen.getByText('v0.2.0')).toBeInTheDocument();
    expect(screen.getByText('Bug fixes and improvements')).toBeInTheDocument();
  });

  it('shows update and later buttons', () => {
    render(
      <UpdateBanner
        updateInfo={updateInfo}
        isDownloading={false}
        downloadError={null}
        onApplyUpdate={() => {}}
        onDismiss={() => {}}
      />
    );
    expect(screen.getByText('Update')).toBeInTheDocument();
    expect(screen.getByText('Later')).toBeInTheDocument();
  });

  it('shows downloading state', () => {
    render(
      <UpdateBanner
        updateInfo={updateInfo}
        isDownloading={true}
        downloadError={null}
        onApplyUpdate={() => {}}
        onDismiss={() => {}}
      />
    );
    expect(screen.getByText('Downloading...')).toBeInTheDocument();
    expect(screen.queryByText('Update')).not.toBeInTheDocument();
  });

  it('shows error message', () => {
    render(
      <UpdateBanner
        updateInfo={updateInfo}
        isDownloading={false}
        downloadError="Network error"
        onApplyUpdate={() => {}}
        onDismiss={() => {}}
      />
    );
    expect(screen.getByText('Network error')).toBeInTheDocument();
  });
});
