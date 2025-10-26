using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Hashing;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Threading.Tasks;
using LetheLauncher.Models;
using Microsoft.Win32;

namespace LetheLauncher.Services;

public class DownloadService : IDisposable
{
    private readonly HttpClient _httpClient;
    private const string ManifestUrl = "https://files.lethelc.site/lethe-manifest.json";
    private const string DownloadBaseUrl = "https://files.lethelc.site/download/";

    public event Action<long, long>? ProgressChanged; // bytesDownloaded, totalBytes
    public event Action<string>? StatusChanged; // status message

    public DownloadService()
    {
        _httpClient = new HttpClient();
    }

    public static string GetLocalGameFolderPath()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            try
            {
                // Read Steam installation path from Windows registry
                using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\WOW6432Node\Valve\Steam"))
                {
                    if (key?.GetValue("InstallPath") is string steamPath)
                    {
                        return Path.Combine(steamPath, "steamapps", "common", "Limbus Company");
                    }
                }
            }
            catch (Exception ex)
            {
                Logger.WriteLine($"Error reading Steam path from registry: {ex.Message}");
            }

            // Fallback for Windows if registry read fails
            return Path.Combine("C:", "Program Files (x86)", "Steam", "steamapps", "common", "Limbus Company");
        }
        else
        {
            // macOS/CrossOver path
            var homeDirectory = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            return Path.Combine(homeDirectory, "Library", "Application Support", "CrossOver", "Bottles", "Steam", "drive_c", "Program Files (x86)", "Steam", "steamapps", "common", "Limbus Company");
        }
    }

    public async Task<FileManifest?> DownloadManifestAsync()
    {
        try
        {
            StatusChanged?.Invoke("Downloading manifest...");
            var response = await _httpClient.GetStringAsync(ManifestUrl);
            var manifest = JsonSerializer.Deserialize<FileManifest>(response);
            Logger.WriteLine($"Downloaded manifest: {manifest?.TotalFiles} files, {manifest?.TotalSize} bytes");
            return manifest;
        }
        catch (Exception ex)
        {
            Logger.WriteLine($"Error downloading manifest: {ex.Message}");
            return null;
        }
    }

    public async Task DownloadFilesAsync(List<FileEntry> filesToDownload, long totalBytesProcessed, long totalBytes)
    {
        var downloadedCount = 0;
        var totalFiles = filesToDownload.Count;
        var processedBytes = totalBytesProcessed;
        var localGameFolderPath = GetLocalGameFolderPath();

        foreach (var fileEntry in filesToDownload)
        {
            try
            {
                downloadedCount++;

                // First try to get the file from local folder
                StatusChanged?.Invoke($"Checking local for {fileEntry.Path}...");
                bool copiedFromLocal = await TryGetFileFromLocalAsync(fileEntry, localGameFolderPath);

                if (!copiedFromLocal)
                {
                    // Fall back to downloading
                    StatusChanged?.Invoke($"Downloading {fileEntry.Path}...");

                    var downloadUrl = DownloadBaseUrl + fileEntry.Path;
                    var fileData = await DownloadWithProgressAsync(downloadUrl, fileEntry.Size, processedBytes, totalBytes);

                    // Ensure directory exists
                    var directory = Path.GetDirectoryName(fileEntry.Path);
                    if (!string.IsNullOrEmpty(directory))
                    {
                        Directory.CreateDirectory(directory);
                    }

                    await File.WriteAllBytesAsync(fileEntry.Path, fileData);
                    Logger.WriteLine($"Downloaded: {fileEntry.Path} ({FormatBytes(fileEntry.Size)})");
                }

                // Add processed bytes regardless of source (local copy or download)
                processedBytes += fileEntry.Size;
            }
            catch (Exception ex)
            {
                Logger.WriteLine($"Error processing {fileEntry.Path}: {ex.Message}");
            }
        }

        StatusChanged?.Invoke("Download complete!");
    }

    public async Task DownloadAdditionalDllsAsync()
    {
        var additionalFiles = new[]
        {
            new { Url = "https://api.lethelc.site/Lethe.dll", FileName = "Lethe.dll" },
            new { Url = "https://api.lethelc.site/ModularSkillScripts.dll", FileName = "ModularSkillScripts.dll" }
        };

        foreach (var file in additionalFiles)
        {
            try
            {
                StatusChanged?.Invoke($"Downloading {file.FileName}...");

                // Ensure BepInEx/plugins directory exists
                var pluginsDir = Path.Combine("BepInEx", "plugins");
                Directory.CreateDirectory(pluginsDir);

                var filePath = Path.Combine(pluginsDir, file.FileName);

                var fileData = await DownloadFileDirectAsync(file.Url);
                await File.WriteAllBytesAsync(filePath, fileData);

                Logger.WriteLine($"Downloaded additional DLL: {file.FileName} ({FormatBytes(fileData.Length)})");
            }
            catch (Exception ex)
            {
                Logger.WriteLine($"Error downloading {file.FileName}: {ex.Message}");
            }
        }
    }

    public async Task<bool> TryGetFileFromLocalAsync(FileEntry fileEntry, string localGameFolderPath)
    {
        try
        {
            var localFilePath = Path.Combine(localGameFolderPath, fileEntry.Path);

            // Check if file exists locally
            if (!File.Exists(localFilePath))
            {
                return false;
            }

            // Check file size first (quick check)
            var fileInfo = new FileInfo(localFilePath);
            if (fileInfo.Length != fileEntry.Size)
            {
                Logger.WriteLine($"Size mismatch for local copy {fileEntry.Path}: expected {fileEntry.Size}, got {fileInfo.Length}");
                return false;
            }

            // Check XXHash to ensure file integrity
            var localFileHash = await ComputeXxHashAsync(localFilePath);
            if (localFileHash != fileEntry.XxHash)
            {
                Logger.WriteLine($"Hash mismatch for local copy {fileEntry.Path}: expected {fileEntry.XxHash}, got {localFileHash}");
                return false;
            }

            // File matches! Copy it to the target location
            var targetPath = fileEntry.Path;
            var targetDirectory = Path.GetDirectoryName(targetPath);
            if (!string.IsNullOrEmpty(targetDirectory))
            {
                Directory.CreateDirectory(targetDirectory);
            }

            using (var sourceStream = new FileStream(localFilePath, FileMode.Open, FileAccess.Read))
            using (var targetStream = new FileStream(targetPath, FileMode.Create, FileAccess.Write))
            {
                await sourceStream.CopyToAsync(targetStream);
            }

            Logger.WriteLine($"Copied from local: {fileEntry.Path} ({FormatBytes(fileEntry.Size)})");
            return true;
        }
        catch (Exception ex)
        {
            Logger.WriteLine($"Error checking/copying local file {fileEntry.Path}: {ex.Message}");
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

    private async Task<byte[]> DownloadFileDirectAsync(string url)
    {
        using (var response = await _httpClient.GetAsync(url))
        {
            response.EnsureSuccessStatusCode();
            return await response.Content.ReadAsByteArrayAsync();
        }
    }

    private async Task<byte[]> DownloadWithProgressAsync(string url, long expectedSize, long baseProcessedBytes, long totalBytes)
    {
        using (var response = await _httpClient.GetAsync(url, HttpCompletionOption.ResponseHeadersRead))
        {
            response.EnsureSuccessStatusCode();

            var buffer = new byte[8192];
            var totalRead = 0L;
            var content = new MemoryStream();

            using (var stream = await response.Content.ReadAsStreamAsync())
            {
                int read;
                while ((read = await stream.ReadAsync(buffer, 0, buffer.Length)) > 0)
                {
                    await content.WriteAsync(buffer, 0, read);
                    totalRead += read;

                    // Report progress with partial download progress
                    var currentTotalBytes = baseProcessedBytes + totalRead;
                    ProgressChanged?.Invoke(currentTotalBytes, totalBytes);
                }
            }

            return content.ToArray();
        }
    }

    public static string FormatBytes(long bytes)
    {
        if (bytes < 1024) return $"{bytes} B";
        if (bytes < 1024 * 1024) return $"{bytes / 1024.0:F1} KB";
        if (bytes < 1024 * 1024 * 1024) return $"{bytes / (1024.0 * 1024.0):F1} MB";
        return $"{bytes / (1024.0 * 1024.0 * 1024.0):F1} GB";
    }

    public void Dispose()
    {
        _httpClient?.Dispose();
    }
}