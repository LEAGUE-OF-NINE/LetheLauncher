using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Hashing;
using System.Net.Http;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Threading;
using LetheLauncher.Models;

namespace LetheLauncher;

public partial class MainWindow : Window
{
    private readonly string _configFilePath = "lethe-launcher.ini";
    private Dictionary<string, string> _config = new();
    private readonly HttpClient _httpClient = new();
    private const string ManifestUrl = "https://files.lethelc.site/lethe-manifest.json";
    private const string DownloadBaseUrl = "https://files.lethelc.site/download/";
    private FileManifest? _manifest;
    private int _totalFilesToCheck;
    private int _checkedFiles;

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
            await GameStarter.StartGame();

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
            await UpdateStatus("Downloading manifest...");
            await DownloadManifest();

            if (_manifest == null)
            {
                await UpdateStatus("Failed to download manifest");
                return;
            }

            _totalFilesToCheck = _manifest.Files.Count;
            await UpdateStatus($"Checking {_totalFilesToCheck} files...");

            await PerformFileChecks();
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error during file synchronization: {ex.Message}");
            await UpdateStatus("Synchronization failed");
        }
    }

    private async Task UpdateStatus(string status)
    {
        await Dispatcher.UIThread.InvokeAsync(() =>
        {
            StatusText.Text = status;
        });
    }

    private async Task DownloadManifest()
    {
        try
        {
            var response = await _httpClient.GetStringAsync(ManifestUrl);
            _manifest = JsonSerializer.Deserialize<FileManifest>(response);
            Console.WriteLine($"Downloaded manifest: {_manifest?.TotalFiles} files, {_manifest?.TotalSize} bytes");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error downloading manifest: {ex.Message}");
            _manifest = null;
        }
    }

    private async Task PerformFileChecks()
    {
        if (_manifest == null) return;

        var filesToDownload = new List<FileEntry>();

        foreach (var fileEntry in _manifest.Files)
        {
            _checkedFiles++;
            var progress = (double)_checkedFiles / _totalFilesToCheck * 100;

            await Dispatcher.UIThread.InvokeAsync(() =>
            {
                UpdateProgressBar.Value = progress;
                PercentageText.Text = $"{progress:F0}%";
            });

            var filePath = fileEntry.Path;
            await UpdateStatus("Checking " + filePath + "...");

            // First check: file size
            if (!await CheckFileSize(fileEntry))
            {
                Console.WriteLine("File size mismatch or missing: " + filePath);
                filesToDownload.Add(fileEntry);
                continue;
            }

            // Second check: XXHash if size matches
            if (!await CheckFileHash(fileEntry))
            {
                Console.WriteLine("File hash mismatch: " + filePath);
                filesToDownload.Add(fileEntry);
            }
        }

        // Download missing/corrupted files
        if (filesToDownload.Count > 0)
        {
            await UpdateStatus($"Downloading {filesToDownload.Count} files...");
            await DownloadFiles(filesToDownload);
        }
        else
        {
            await UpdateStatus("All files are up to date!");
        }
    }

    private async Task<bool> CheckFileSize(FileEntry fileEntry)
    {
        try
        {
            var fileInfo = new FileInfo(fileEntry.Path);
            return fileInfo.Exists && fileInfo.Length == fileEntry.Size;
        }
        catch
        {
            return false;
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

    private async Task DownloadFiles(List<FileEntry> filesToDownload)
    {
        var downloadedCount = 0;
        var totalFiles = filesToDownload.Count;

        foreach (var fileEntry in filesToDownload)
        {
            try
            {
                downloadedCount++;
                var progress = (double)downloadedCount / totalFiles * 100;

                await Dispatcher.UIThread.InvokeAsync(() =>
                {
                    UpdateProgressBar.Value = progress;
                    PercentageText.Text = $"{progress:F0}%";
                });

                await UpdateStatus("Downloading " + fileEntry.Path + "...");

                var downloadUrl = DownloadBaseUrl + fileEntry.Path;
                var fileData = await _httpClient.GetByteArrayAsync(downloadUrl);

                // Ensure directory exists
                var directory = Path.GetDirectoryName(fileEntry.Path);
                if (!string.IsNullOrEmpty(directory))
                {
                    Directory.CreateDirectory(directory);
                }

                await File.WriteAllBytesAsync(fileEntry.Path, fileData);
                Console.WriteLine("Downloaded: " + fileEntry.Path);
            }
            catch (Exception ex)
            {
                Console.WriteLine("Error downloading " + fileEntry.Path + ": " + ex.Message);
            }
        }

        await UpdateStatus("Download complete!");
    }

    protected override void OnClosed(EventArgs e)
    {
        _httpClient.Dispose();
        base.OnClosed(e);
    }
}