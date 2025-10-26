# Lethe Launcher

A custom launcher for Limbus Company that provides intelligent file management, Steam integration, and modding support via BepInEx.

## Features

### 🚀 Smart File Management
- **Manifest-based updates**: Downloads only changed or missing files
- **XXHash verification**: Ensures file integrity with fast checksum validation
- **Local copy optimization**: Automatically copies files from existing Steam installation to avoid re-downloading
- **Steam path detection**: Uses Windows registry to automatically locate Steam installation
- **Progress tracking**: Real-time download progress with file count and byte progress

### 🎮 Game Integration
- **Direct game launch**: Calls UnityMain from UnityPlayer.dll for seamless startup
- **BepInEx support**: Loads doorstop.dll to enable mod framework integration
- **Steam compatibility**: Works with existing Steam installations of Limbus Company

### 📊 Logging & Diagnostics
- **Dual logging**: Outputs to both console and `lethe-launcher.log` file
- **Detailed diagnostics**: Comprehensive error reporting with file paths and Windows error codes
- **Session tracking**: Timestamped logs with clear session boundaries
- **Configuration logging**: Tracks all configuration changes and settings

### ⚙️ Configuration
- **Configuration file**: `lethe-launcher.ini` for customizable behavior
- **Silent mode**: `DisableAutoUpdate=true` hides UI and launches game directly
- **Flexible deployment**: Single executable with minimal dependencies

## Building

### Prerequisites
- .NET 8.0 SDK
- Windows (for final deployment)

### Build Command
```bash
dotnet publish --configuration Release
```

### Output
The build creates a distribution-ready package at:
```
bin/Release/net8.0/win-x64/distribution/
├── LimbusCompany.exe          (74MB - Main launcher)
├── av_libglesv2.dll          (4.2MB - OpenGL graphics)
├── libHarfBuzzSharp.dll      (1.5MB - Text rendering)
└── libSkiaSharp.dll          (9.0MB - Skia graphics)
```

**Total package size:** ~89MB

## Deployment

### Required Files Structure
Place the launcher in the game directory with this structure:
```
GameDirectory/
├── LimbusCompany.exe          (Launcher - renamed for stealth)
├── UnityPlayer.dll            (Unity engine)
├── UnityMain.dll              (Unity entry point)
├── BepInEx/
│   └── doorstop/
│       └── doorstop.dll       (BepInEx loader)
├── av_libglesv2.dll          (OpenGL support)
├── libHarfBuzzSharp.dll      (Text rendering)
└── libSkiaSharp.dll          (Graphics library)
```

### Configuration File
Create `lethe-launcher.ini` in the same directory:
```ini
DisableAutoUpdate=false
```

**Options:**
- `DisableAutoUpdate=false` - Show UI and perform file synchronization
- `DisableAutoUpdate=true` - Silent mode, skip UI, launch game directly

## How It Works

### Startup Sequence
1. **Configuration Loading**: Reads `lethe-launcher.ini` settings
2. **Mode Detection**: Checks if auto-update is disabled
3. **File Synchronization** (if enabled):
   - Downloads manifest from remote server
   - Checks local files against manifest (size + XXHash)
   - Attempts to copy missing files from Steam installation
   - Downloads remaining files from remote server
4. **Game Launch**:
   - Loads `UnityPlayer.dll` (required by doorstop)
   - Loads `BepInEx/doorstop/doorstop.dll` (enables mods)
   - Calls `UnityMain(0, 0, "", 0x1)` to start game

### File Verification Process
1. **Size Check**: Quick file size comparison
2. **Hash Verification**: XXHash checksum validation for integrity
3. **Local Copy Attempt**: Check Steam installation for existing files
4. **Remote Download**: Download only if local copy unavailable or invalid

### Steam Integration
- Reads Steam installation path from Windows registry: `HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Valve\Steam`
- Constructs game path: `{SteamPath}/steamapps/common/Limbus Company`
- Falls back to default path if registry read fails

## Technical Details

### Dependencies
- **Avalonia UI**: Cross-platform UI framework
- **System.Text.Json**: JSON manifest parsing
- **System.IO.Hashing**: XXHash implementation for file verification
- **Microsoft.Win32**: Windows registry access for Steam path detection

### File Formats
- **Manifest**: JSON format containing file entries with paths, sizes, and XXHash checksums
- **Configuration**: INI-style key=value pairs
- **Logs**: Timestamped plain text with session markers

### Error Handling
- Comprehensive error logging with specific error codes
- Graceful fallbacks for registry access failures
- File existence verification before DLL loading
- Detailed path resolution for debugging

### Performance Optimizations
- XXHash for fast file verification (faster than MD5/SHA)
- Local file copying to minimize network usage
- Streaming downloads with progress callbacks
- Parallel file existence checks

## Logging

All operations are logged to `lethe-launcher.log` with timestamps:

```
=== Lethe Launcher Started at 2025-10-26 12:43:48 ===
[12:43:48] Configuration loaded from lethe-launcher.ini:
[12:43:48]   DisableAutoUpdate = false
[12:43:49] Downloaded manifest: 150 files, 2048576 bytes
[12:43:49] Starting file synchronization...
[12:43:50] Copied from local: Data/file.dat (1.2 MB)
[12:43:51] Downloaded: Assets/texture.png (512 KB)
[12:43:52] Update complete. Starting game...
[12:43:52] ✓ Found UnityPlayer.dll at: C:\Game\UnityPlayer.dll
[12:43:52] ✓ UnityPlayer.dll loaded successfully
[12:43:52] ✓ doorstop.dll loaded successfully
[12:43:52] UnityMain returned: 0
```