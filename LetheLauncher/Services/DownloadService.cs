using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using LetheLauncher.Models;

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

    public async Task<FileManifest?> DownloadManifestAsync()
    {
        try
        {
            StatusChanged?.Invoke("Downloading manifest...");
            var response = await _httpClient.GetStringAsync(ManifestUrl);
            var manifest = JsonSerializer.Deserialize<FileManifest>(response);
            Console.WriteLine($"Downloaded manifest: {manifest?.TotalFiles} files, {manifest?.TotalSize} bytes");
            return manifest;
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error downloading manifest: {ex.Message}");
            return null;
        }
    }

    public async Task DownloadFilesAsync(List<FileEntry> filesToDownload, long totalBytesProcessed, long totalBytes)
    {
        var downloadedCount = 0;
        var totalFiles = filesToDownload.Count;
        var processedBytes = totalBytesProcessed;

        foreach (var fileEntry in filesToDownload)
        {
            try
            {
                downloadedCount++;
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
                Console.WriteLine($"Downloaded: {fileEntry.Path} ({FormatBytes(fileEntry.Size)})");

                // Add downloaded bytes to processed bytes
                processedBytes += fileEntry.Size;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error downloading {fileEntry.Path}: {ex.Message}");
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

                Console.WriteLine($"Downloaded additional DLL: {file.FileName} ({FormatBytes(fileData.Length)})");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error downloading {file.FileName}: {ex.Message}");
            }
        }
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