using System;
using Outpost3.Core;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;
using Outpost3.Core.Projections;

namespace Outpost3.UI.Common;

/// <summary>
/// Time scale context for different game screens.
/// </summary>
public enum TimeScaleContext
{
    /// <summary>
    /// Galaxy map - years should pass quickly for probe travel.
    /// </summary>
    Galaxy,

    /// <summary>
    /// Star system map - days/months should be manageable.
    /// </summary>
    System
}

/// <summary>
/// Reusable time advancement system for game presenters.
/// Handles automatic time progression based on game speed and pause state.
/// Includes context-aware scaling for galaxy vs system operations.
/// </summary>
public class TimeManager
{
    private StateStore _stateStore;
    private double _autoAdvanceTimer = 0.0;
    private TimeScaleContext _context;

    public TimeManager(StateStore stateStore, TimeScaleContext context = TimeScaleContext.System)
    {
        _stateStore = stateStore ?? throw new ArgumentNullException(nameof(stateStore));
        _context = context;
    }

    /// <summary>
    /// Update time advancement. Call this from _Process() in presenters.
    /// </summary>
    /// <param name="delta">Frame delta time in seconds</param>
    /// <param name="gameSpeed">Current game speed setting</param>
    /// <param name="isPaused">Whether the game is paused</param>
    public void Update(double delta, GameSpeed gameSpeed, bool isPaused)
    {
        if (isPaused || _stateStore == null)
        {
            return;
        }

        var timeScale = GetContextualSpeedMultiplier(gameSpeed, _context);
        _autoAdvanceTimer += delta * timeScale;

        if (_autoAdvanceTimer >= 1.0) // Advance every 1 second of real time
        {
            var hours = _autoAdvanceTimer;
            _autoAdvanceTimer = 0;
            var command = new AdvanceTime(hours);
            _stateStore.ApplyCommand(command);
        }
    }

    /// <summary>
    /// Get speed multiplier based on game speed and context.
    /// Galaxy context uses much higher multipliers for interstellar travel.
    /// System context uses moderate multipliers for planetary operations.
    /// </summary>
    private static double GetContextualSpeedMultiplier(GameSpeed speed, TimeScaleContext context)
    {
        return context switch
        {
            TimeScaleContext.Galaxy => speed switch
            {
                GameSpeed.Paused => 0.0,
                GameSpeed.Normal => 24.0 * 30,     // 1 month per second (720 hours)
                GameSpeed.Fast => 24.0 * 90,       // 3 months per second (2,160 hours)
                GameSpeed.Faster => 24.0 * 365,    // 1 year per second (8,760 hours)
                GameSpeed.Fastest => 24.0 * 365 * 5, // 5 years per second (43,800 hours)
                _ => 24.0 * 30
            },
            TimeScaleContext.System => speed switch
            {
                GameSpeed.Paused => 0.0,
                GameSpeed.Normal => 4.0,           // 4 hours per second (1 day in 6 seconds)
                GameSpeed.Fast => 12.0,            // 12 hours per second (1 day in 2 seconds)
                GameSpeed.Faster => 24.0 * 7,     // 1 week per second (168 hours)
                GameSpeed.Fastest => 24.0 * 30,   // 1 month per second (720 hours)
                _ => 4.0
            },
            _ => DisplayFormatter.GetSpeedMultiplier(speed)
        };
    }

    /// <summary>
    /// Change the time scale context (useful when switching between galaxy and system views).
    /// </summary>
    public void SetContext(TimeScaleContext context)
    {
        _context = context;
        Reset(); // Reset timer to avoid sudden time jumps
    }

    /// <summary>
    /// Reset the internal timer (useful when switching scenes or pausing/unpausing).
    /// </summary>
    public void Reset()
    {
        _autoAdvanceTimer = 0.0;
    }
}