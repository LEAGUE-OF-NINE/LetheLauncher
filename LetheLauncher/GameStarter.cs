using System;
using System.Runtime.InteropServices;

namespace LetheLauncher;

public static class GameStarter
{
    /// <summary>
    /// Imports the LoadLibrary function from kernel32.dll
    /// </summary>
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr LoadLibrary(string lpFileName);

    /// <summary>
    /// Imports the UnityMain function from UnityMain.dll
    /// </summary>
    [DllImport("UnityMain.dll", CallingConvention = CallingConvention.StdCall)]
    private static extern int UnityMain(IntPtr hInstance, IntPtr hPrevInstance, [MarshalAs(UnmanagedType.LPWStr)] string lpCmdLine, int nShowCmd);

    /// <summary>
    /// Starts the game by calling UnityMain from UnityMain.dll
    /// </summary>
    public static void StartGame()
    {
        try
        {
            Console.WriteLine("Starting game...");

            // Load doorstop.dll to trigger its DllEntry
            Console.WriteLine("Loading doorstop.dll...");
            IntPtr doorstopHandle = LoadLibrary("BepInEx/doorstop/doorstop.dll");

            if (doorstopHandle == IntPtr.Zero)
                throw new Exception("Failed to load doorstop.dll");

            // Call UnityMain with the specified parameters
            var result = UnityMain(IntPtr.Zero, IntPtr.Zero, "", 0x1);
            Console.WriteLine($"UnityMain returned: {result}");
        }
        catch (Exception ex)
        {
            Console.WriteLine("Error starting game: " + ex.Message);
        }
    }
}