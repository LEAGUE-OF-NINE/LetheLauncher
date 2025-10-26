using System;
using System.IO;

namespace LetheLauncher;

public static class Logger
{
    private static readonly string LogFilePath = "lethe-launcher.log";
    private static readonly object LogLock = new object();

    static Logger()
    {
        // Initialize log file with timestamp
        try
        {
            lock (LogLock)
            {
                File.AppendAllText(LogFilePath, $"\n=== Lethe Launcher Started at {DateTime.Now:yyyy-MM-dd HH:mm:ss} ===\n");
            }
        }
        catch
        {
            // Silently fail if we can't write to log file
        }
    }

    public static void WriteLine(string message)
    {
        var timestampedMessage = $"[{DateTime.Now:HH:mm:ss}] {message}";

        // Write to console
        Console.WriteLine(timestampedMessage);

        // Write to log file
        try
        {
            lock (LogLock)
            {
                File.AppendAllText(LogFilePath, timestampedMessage + Environment.NewLine);
            }
        }
        catch
        {
            // Silently fail if we can't write to log file
        }
    }

    public static void WriteLine(string format, params object[] args)
    {
        WriteLine(string.Format(format, args));
    }
}