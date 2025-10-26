using System;
using System.Threading;
using Avalonia.Controls;
using Avalonia.Threading;

namespace LetheLauncher;

public partial class MainWindow : Window
{
    private Timer? _updateTimer;
    private double _currentProgress = 0;

    public MainWindow()
    {
        InitializeComponent();
        StartProgressSimulation();
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