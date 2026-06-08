# Lethe Launcher

A custom launcher for Limbus Company that provides intelligent file management, Steam integration, and modding support via BepInEx.

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
