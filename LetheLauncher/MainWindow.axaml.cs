using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using Avalonia.Controls;
using Avalonia.Threading;

namespace LetheLauncher;

public partial class MainWindow : Window
{
    private Timer? _updateTimer;
    private double _currentProgress = 0;
    private readonly string _configFilePath = "lethe-launcher.ini";
    private Dictionary<string, string> _config = new();

    public MainWindow()
    {
        InitializeComponent();
        InitializeConfiguration();
        StartProgressSimulation();
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

    private void StartProgressSimulation()
    {
        // Update progress every second
        _updateTimer = new Timer(UpdateProgress, null, TimeSpan.Zero, TimeSpan.FromSeconds(1));
    }

    private void UpdateProgress(object? state)
    {
        // Increment progress by a random amount between 1-5% each second
        var random = new Random();
        var increment = random.NextDouble() * 4 + 1; // 1-5%

        _currentProgress += increment;

        // Cap at 100%
        if (_currentProgress > 100)
        {
            _currentProgress = 100;
            _updateTimer?.Dispose(); // Stop the timer when complete
        }

        // Update UI on the main thread
        Dispatcher.UIThread.Post(() =>
        {
            UpdateProgressBar.Value = _currentProgress;
            PercentageText.Text = $"{_currentProgress:F0}%";

            // Change status text when complete
            if (_currentProgress >= 100)
            {
                StatusText.Text = "Update Complete!";
            }
        });
    }

    protected override void OnClosed(EventArgs e)
    {
        _updateTimer?.Dispose();
        base.OnClosed(e);
    }
}