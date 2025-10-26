using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;

namespace LetheLauncher;

public static class GameStarter
{
    /// <summary>
    /// Imports the LoadLibrary function from kernel32.dll
    /// </summary>
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr LoadLibrary(string lpFileName);

    /// <summary>
    /// Imports GetLastError function from kernel32.dll
    /// </summary>
    [DllImport("kernel32.dll")]
    private static extern uint GetLastError();

    /// <summary>
    /// Imports the UnityMain function from UnityMain.dll
    /// </summary>
    [DllImport("UnityPlayer.dll", CallingConvention = CallingConvention.StdCall)]
    private static extern int UnityMain(IntPtr hInstance, IntPtr hPrevInstance, [MarshalAs(UnmanagedType.LPWStr)] string lpCmdLine, int nShowCmd);

    /// <summary>
    /// Starts the game by calling UnityMain from UnityMain.dll
    /// </summary>
    public static void StartGame()
    {
        try
        {
            Logger.WriteLine("Starting game...");
            Logger.WriteLine($"Current working directory: {Directory.GetCurrentDirectory()}");

            Environment.SetEnvironmentVariable("DOORSTOP_DISABLE_PROXY", "1");

            // Check if required files exist
            const string unityPlayerPath = "UnityPlayer.dll";
            const string doorstopPath = "BepInEx/doorstop/doorstop.dll";

            Logger.WriteLine("Checking for required files...");

            if (!File.Exists(unityPlayerPath))
            {
                throw new Exception($"UnityPlayer.dll not found at: {Path.GetFullPath(unityPlayerPath)}");
            }
            Logger.WriteLine($"✓ Found UnityPlayer.dll at: {Path.GetFullPath(unityPlayerPath)}");

            if (!File.Exists(doorstopPath))
            {
                throw new Exception($"doorstop.dll not found at: {Path.GetFullPath(doorstopPath)}");
            }
            Logger.WriteLine($"✓ Found doorstop.dll at: {Path.GetFullPath(doorstopPath)}");


            // Load UnityPlayer.dll first (required by doorstop)
            Logger.WriteLine("Loading UnityPlayer.dll...");
            IntPtr unityPlayerHandle = LoadLibrary(unityPlayerPath);

            if (unityPlayerHandle == IntPtr.Zero)
            {
                uint error = GetLastError();
                throw new Exception($"Failed to load UnityPlayer.dll (Windows Error: {error})");
            }
            Logger.WriteLine("✓ UnityPlayer.dll loaded successfully");

            // Load doorstop.dll to trigger its DllEntry
            Logger.WriteLine("Loading doorstop.dll...");
            IntPtr doorstopHandle = LoadLibrary(doorstopPath);

            if (doorstopHandle == IntPtr.Zero)
            {
                uint error = GetLastError();
                throw new Exception($"Failed to load doorstop.dll (Windows Error: {error})");
            }
            Logger.WriteLine("✓ doorstop.dll loaded successfully");

            // Call UnityMain with the specified parameters
            Logger.WriteLine("Calling UnityMain...");
            var result = UnityMain(IntPtr.Zero, IntPtr.Zero, "", 0x1);
            Logger.WriteLine($"UnityMain returned: {result}");
        }
        catch (Exception ex)
        {
            Logger.WriteLine("Error starting game: " + ex.Message);
        }
    }
}