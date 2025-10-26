using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Hashing;
using System.Linq;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Threading;
using LetheLauncher.Models;
using LetheLauncher.Services;

namespace LetheLauncher;

public partial class MainWindow : Window
{
    private readonly string _configFilePath = "lethe-launcher.ini";
    private Dictionary<string, string> _config = new();
    private DownloadService? _downloadService;
    private FileManifest? _manifest;
    private int _totalFilesToCheck;
    private long _totalBytes;
    private long _processedBytes;

    public MainWindow()
    {
        InitializeComponent();
        InitializeConfiguration();

        // Check if auto-update is disabled
        if (GetConfigValue("DisableAutoUpdate", "false").ToLowerInvariant() == "true")
        {
            // Hide the window completely and launch game directly
            HideUIAndStartGame();
        }
        else
        {
            // Normal flow with file synchronization
            StartFileSynchronization();
        }
    }

    private void InitializeConfiguration()
    {
        try
        {
            if (!File.Exists(_configFilePath))
            {
                CreateDefaultConfigFile();
            }

            ReadConfigFile();

            // Log the configuration (for debugging)
            Console.WriteLine($"Configuration loaded from {_configFilePath}:");
            foreach (var kvp in _config)
            {
                Console.WriteLine($"  {kvp.Key} = {kvp.Value}");
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error initializing configuration: {ex.Message}");
        }
    }

    private void CreateDefaultConfigFile()
    {
        try
        {
            var defaultConfig = "DisableAutoUpdate=false\n";
            File.WriteAllText(_configFilePath, defaultConfig);
            Console.WriteLine($"Created default configuration file: {_configFilePath}");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error creating config file: {ex.Message}");
        }
    }

    private void ReadConfigFile()
    {
        try
        {
            _config.Clear();
            var lines = File.ReadAllLines(_configFilePath);

            foreach (var line in lines)
            {
                var trimmedLine = line.Trim();
                if (string.IsNullOrEmpty(trimmedLine) || trimmedLine.StartsWith("#") || trimmedLine.StartsWith(";"))
                {
                    continue; // Skip empty lines and comments
                }

                var separatorIndex = trimmedLine.IndexOf('=');
                if (separatorIndex > 0)
                {
                    var key = trimmedLine.Substring(0, separatorIndex).Trim();
                    var value = trimmedLine.Substring(separatorIndex + 1).Trim();
                    _config[key] = value;
                }
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error reading config file: {ex.Message}");
        }
    }

    private string GetConfigValue(string key, string defaultValue = "")
    {
        return _config.TryGetValue(key, out var value) ? value : defaultValue;
    }

    private async void HideUIAndStartGame()
    {
        try
        {
            // Completely hide the window
            await Dispatcher.UIThread.InvokeAsync(() =>
            {
                WindowState = WindowState.Minimized;
                ShowInTaskbar = false;
                IsVisible = false;
            });

            Console.WriteLine("DisableAutoUpdate=true detected. UI hidden, skipping file synchronization.");

            // Launch the game directly
            GameStarter.StartGame();

            // Close the launcher after game starts
            await Dispatcher.UIThread.InvokeAsync(() =>
            {
                Close();
            });
        }
        catch (Exception ex)
        {
            Console.WriteLine("Error in direct game start: " + ex.Message);
            // Don't show UI on error when in hidden mode, just exit
            await Dispatcher.UIThread.InvokeAsync(() =>
            {
                Close();
            });
        }
    }

    private async void StartFileSynchronization()
    {
        try
        {
            _downloadService = new DownloadService();
            _downloadService.StatusChanged += async (status) => await UpdateFilePath(status);
            _downloadService.ProgressChanged += OnDownloadProgress;

            _manifest = await _downloadService.DownloadManifestAsync();

            if (_manifest == null)
            {
                await UpdateFilePath("Failed to download manifest");
                return;
            }

            _totalFilesToCheck = _manifest.Files.Count;
            _totalBytes = _manifest.Files.Sum(f => f.Size);
            _processedBytes = 0;
            await UpdateFilePath("Checking " + _totalFilesToCheck + " files...");

            await PerformFileChecks();
        }
        catch (Exception ex)
        {
            Console.WriteLine("Error during file synchronization: " + ex.Message);
            await UpdateFilePath("Synchronization failed");
        }
    }

    private async Task UpdateStatus(string status)
    {
        await Dispatcher.UIThread.InvokeAsync(() =>
        {
            // Keep the main status as "Updating Lethe..." always
            StatusText.Text = "Updating Lethe...";
        });
    }

    private async Task UpdateFilePath(string filePath)
    {
        await Dispatcher.UIThread.InvokeAsync(() =>
        {
            FilePathText.Text = filePath;
        });
    }

    private async void OnDownloadProgress(long bytesProcessed, long totalBytes)
    {
        var progress = totalBytes > 0 ? (double)bytesProcessed / totalBytes * 100 : 0;
        await Dispatcher.UIThread.InvokeAsync(() =>
        {
            UpdateProgressBar.Value = progress;
            PercentageText.Text = progress.ToString("F1") + "%";
            DownloadProgressText.Text = DownloadService.FormatBytes(bytesProcessed) + " / " + DownloadService.FormatBytes(totalBytes);
        });
    }

    private async Task StartGameAfterUpdate()
    {
        try
        {
            // Small delay to let user see completion message
            await Task.Delay(1500);

            // Update UI to show game starting
            await UpdateFilePath("Starting game...");
            await Dispatcher.UIThread.InvokeAsync(() =>
            {
                UpdateProgressBar.Value = 100;
                PercentageText.Text = "100%";
            });

            // Small delay before hiding UI
            await Task.Delay(500);

            // Hide the UI
            await Dispatcher.UIThread.InvokeAsync(() =>
            {
                WindowState = WindowState.Minimized;
                ShowInTaskbar = false;
                IsVisible = false;
            });

            Console.WriteLine("Update complete. Starting game...");

            // Launch the game
            GameStarter.StartGame();

            // Close the launcher after game starts
            await Dispatcher.UIThread.InvokeAsync(() =>
            {
                Close();
            });
        }
        catch (Exception ex)
        {
            Console.WriteLine("Error starting game after update: " + ex.Message);
            await Dispatcher.UIThread.InvokeAsync(() =>
            {
                Close();
            });
        }
    }


    private async Task PerformFileChecks()
    {
        if (_manifest == null) return;

        var filesToDownload = new List<FileEntry>();

        foreach (var fileEntry in _manifest.Files)
        {
            var filePath = fileEntry.Path;
            await UpdateFilePath("Checking " + filePath + "...");

            // First check: file size
            if (!await CheckFileSize(fileEntry))
            {
                Console.WriteLine("File size mismatch or missing: " + filePath);
                filesToDownload.Add(fileEntry);
            }
            else
            {
                // Second check: XXHash if size matches
                if (!await CheckFileHash(fileEntry))
                {
                    Console.WriteLine("File hash mismatch: " + filePath);
                    filesToDownload.Add(fileEntry);
                }
                else
                {
                    // File is valid, count its bytes as processed
                    _processedBytes += fileEntry.Size;
                }
            }

            // Update progress based on bytes processed vs total bytes
            var progress = _totalBytes > 0 ? (double)_processedBytes / _totalBytes * 100 : 0;
            await Dispatcher.UIThread.InvokeAsync(() =>
            {
                UpdateProgressBar.Value = progress;
                PercentageText.Text = progress.ToString("F1") + "%";
                DownloadProgressText.Text = DownloadService.FormatBytes(_processedBytes) + " / " + DownloadService.FormatBytes(_totalBytes);
            });
        }

        // Download missing/corrupted files
        if (filesToDownload.Count > 0)
        {
            await UpdateFilePath("Downloading " + filesToDownload.Count + " files...");
            if (_downloadService != null)
            {
                await _downloadService.DownloadFilesAsync(filesToDownload, _processedBytes, _totalBytes);
            }
            await UpdateFilePath("Download complete!");
        }
        else
        {
            await UpdateFilePath("All files are up to date!");
        }

        // Download additional DLLs not in manifest (always download these)
        if (_downloadService != null)
        {
            await _downloadService.DownloadAdditionalDllsAsync();
        }

        // Start game after all downloads are complete
        await StartGameAfterUpdate();
    }

    private Task<bool> CheckFileSize(FileEntry fileEntry)
    {
        try
        {
            var fileInfo = new FileInfo(fileEntry.Path);
            return Task.FromResult(fileInfo.Exists && fileInfo.Length == fileEntry.Size);
        }
        catch
        {
            return Task.FromResult(false);
        }
    }

    private async Task<bool> CheckFileHash(FileEntry fileEntry)
    {
        try
        {
            if (!File.Exists(fileEntry.Path)) return false;

            var computedHash = await ComputeXxHashAsync(fileEntry.Path);
            return computedHash == fileEntry.XxHash;
        }
        catch (Exception ex)
        {
            Console.WriteLine("Error checking hash for " + fileEntry.Path + ": " + ex.Message);
            return false;
        }
    }

    private async Task<string> ComputeXxHashAsync(string filePath)
    {
        const int bufferSize = 64 * 1024; // 64KB chunks for efficient reading
        var buffer = new byte[bufferSize];
        var xxHash = new XxHash64();

        using (var fileStream = new FileStream(filePath, FileMode.Open, FileAccess.Read, FileShare.Read, bufferSize, useAsync: true))
        {
            int bytesRead;
            while ((bytesRead = await fileStream.ReadAsync(buffer, 0, buffer.Length)) > 0)
            {
                // Append the chunk to the hash computation
                xxHash.Append(buffer.AsSpan(0, bytesRead));
            }
        }

        // Get the final hash as a UInt64 and convert to hex string
        var hashValue = xxHash.GetCurrentHashAsUInt64();
        return hashValue.ToString("x16"); // 16-character lowercase hex string
    }

    protected override void OnClosed(EventArgs e)
    {
        _downloadService?.Dispose();
        base.OnClosed(e);
    }
}