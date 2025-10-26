using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace LetheLauncher.Models;

public class FileManifest
{
    [JsonPropertyName("scanned_folder")]
    public string ScannedFolder { get; set; } = string.Empty;

    [JsonPropertyName("total_files")]
    public int TotalFiles { get; set; }

    [JsonPropertyName("total_size")]
    public long TotalSize { get; set; }

    [JsonPropertyName("files")]
    public List<FileEntry> Files { get; set; } = new();
}

public class FileEntry
{
    [JsonPropertyName("path")]
    public string Path { get; set; } = string.Empty;

    [JsonPropertyName("size")]
    public long Size { get; set; }

    [JsonPropertyName("xxhash")]
    public string XxHash { get; set; } = string.Empty;
}