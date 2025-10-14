using System;
using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.Core.Projections;

/// <summary>
/// Helper for calculating orbital positions and motion.
/// Pure static methods for deterministic calculations.
/// </summary>
public static class OrbitalMath
{
    /// <summary>
    /// Calculates the current position of a body in its orbit at a given game time.
    /// Returns position in AU as a 2D vector (x, y) with star at origin.
    /// </summary>
    /// <param name="orbitalParams">The orbital parameters of the body.</param>
    /// <param name="gameTime">The current game time in hours.</param>
    /// <returns>Position vector in AU.</returns>
    public static Vector2 CalculateOrbitalPosition(OrbitalParameters orbitalParams, double gameTime)
    {
        // Convert game time (hours) to days
        var timeDays = gameTime / 24.0;

        // Calculate mean anomaly (angle traveled in orbit)
        // M = (2π * t) / T
        var meanAnomaly = (2.0 * Math.PI * timeDays / orbitalParams.OrbitalPeriodDays)
            + DegreesToRadians(orbitalParams.StartingAngleDegrees);

        // For simplicity, assume circular orbits (eccentricity ignored for now)
        // In a circular orbit, mean anomaly = true anomaly
        var trueAnomaly = meanAnomaly;

        // Calculate position in orbital plane
        var distance = orbitalParams.SemiMajorAxisAU;
        var x = distance * Math.Cos(trueAnomaly);
        var y = distance * Math.Sin(trueAnomaly);

        return new Vector2((float)x, (float)y);
    }

    /// <summary>
    /// Calculates orbital velocity in AU per day at the current position.
    /// Useful for animating orbital motion smoothly.
    /// </summary>
    /// <param name="orbitalParams">The orbital parameters of the body.</param>
    /// <returns>Velocity magnitude in AU per day.</returns>
    public static double CalculateOrbitalVelocity(OrbitalParameters orbitalParams)
    {
        // For circular orbits: v = 2π * r / T
        var circumference = 2.0 * Math.PI * orbitalParams.SemiMajorAxisAU;
        var velocity = circumference / orbitalParams.OrbitalPeriodDays;

        return velocity;
    }

    /// <summary>
    /// Converts AU (Astronomical Units) to pixels for rendering.
    /// </summary>
    /// <param name="distanceAU">Distance in AU.</param>
    /// <param name="pixelsPerAU">Scale factor (pixels per AU).</param>
    /// <returns>Distance in pixels.</returns>
    public static float AUToPixels(double distanceAU, float pixelsPerAU)
    {
        return (float)(distanceAU * pixelsPerAU);
    }

    /// <summary>
    /// Converts pixels to AU for input handling.
    /// </summary>
    /// <param name="pixels">Distance in pixels.</param>
    /// <param name="pixelsPerAU">Scale factor (pixels per AU).</param>
    /// <returns>Distance in AU.</returns>
    public static double PixelsToAU(float pixels, float pixelsPerAU)
    {
        return pixels / pixelsPerAU;
    }

    /// <summary>
    /// Calculates appropriate zoom level to fit a given radius on screen.
    /// </summary>
    /// <param name="radiusAU">Radius to fit in AU.</param>
    /// <param name="viewportSize">Size of the viewport in pixels.</param>
    /// <param name="margin">Margin factor (e.g., 1.2 for 20% margin).</param>
    /// <returns>Recommended pixels per AU scale.</returns>
    public static float CalculateZoomToFit(double radiusAU, Vector2 viewportSize, float margin = 1.2f)
    {
        var minViewportDimension = Math.Min(viewportSize.X, viewportSize.Y);
        var pixelsPerAU = (float)(minViewportDimension / (2.0 * radiusAU * margin));

        return pixelsPerAU;
    }

    /// <summary>
    /// Converts degrees to radians.
    /// </summary>
    private static double DegreesToRadians(double degrees)
    {
        return degrees * Math.PI / 180.0;
    }

    /// <summary>
    /// Converts radians to degrees.
    /// </summary>
    private static double RadiansToDegrees(double radians)
    {
        return radians * 180.0 / Math.PI;
    }
}
