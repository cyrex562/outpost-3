using System;
using Godot;

namespace Outpost3.UI.Common;

/// <summary>
/// Utility functions for rendering celestial objects with appropriate visual scaling.
/// </summary>
public static class CelestialRenderingUtils
{
    /// <summary>
    /// Calculate appropriate visual radius for celestial bodies.
    /// Uses realistic but visually meaningful scaling.
    /// </summary>
    public static float CalculateVisualRadius(string bodyType, double radiusKm)
    {
        // Base sizes for different body types (in pixels)
        var baseRadius = bodyType.ToLower() switch
        {
            var x when x.Contains("star") => 12.0f,        // Stars should be visible but not overwhelming
            var x when x.Contains("gas giant") => 8.0f,   // Gas giants are large
            var x when x.Contains("planet") => 6.0f,      // Regular planets 
            var x when x.Contains("moon") => 3.0f,        // Moons are smaller
            var x when x.Contains("asteroid") => 2.0f,    // Asteroids are tiny
            _ => 4.0f                                      // Default size
        };

        // Apply scaling based on actual radius (logarithmic for reasonable visual scaling)
        // Earth radius ~6371 km, Jupiter ~69911 km, Sun ~696340 km
        var scaleFactor = radiusKm > 1000
            ? (float)Math.Pow(radiusKm / 6371.0, 0.3)  // 0.3 power for gentle scaling
            : 0.5f; // Very small objects get minimum scaling

        var finalRadius = baseRadius * Math.Max(0.5f, Math.Min(scaleFactor, 3.0f)); // Clamp between 0.5x and 3x base size

        return Math.Max(2.0f, finalRadius); // Ensure minimum 2 pixel radius for visibility
    }

    /// <summary>
    /// Calculate star visual radius based on luminosity.
    /// </summary>
    public static float CalculateStarRadius(float luminosity)
    {
        // Scale star size based on luminosity (Sol = 1.0 luminosity)
        return Math.Max(6.0f, Math.Min(16.0f, 8.0f * (float)Math.Pow(luminosity, 0.5)));
    }

    /// <summary>
    /// Create a filled circle polygon for celestial bodies.
    /// </summary>
    public static Vector2[] CreateCirclePolygon(float radius, int segments = 16)
    {
        var points = new Vector2[segments];
        for (int i = 0; i < segments; i++)
        {
            var angle = i * Mathf.Pi * 2.0f / segments;
            points[i] = new Vector2(Mathf.Cos(angle), Mathf.Sin(angle)) * radius;
        }
        return points;
    }

    /// <summary>
    /// Create a Line2D circle for orbits and boundaries.
    /// </summary>
    public static Line2D CreateCircleLine(float radius, Color color, float width = 1.5f, int segments = 64)
    {
        var circle = new Line2D();
        circle.DefaultColor = color;
        circle.Width = width;

        for (int i = 0; i <= segments; i++)
        {
            var angle = i * Mathf.Pi * 2.0f / segments;
            var point = new Vector2(Mathf.Cos(angle), Mathf.Sin(angle)) * radius;
            circle.AddPoint(point);
        }

        return circle;
    }

    /// <summary>
    /// Create a celestial body node with proper visual representation.
    /// </summary>
    public static Node2D CreateCelestialBodyNode(string name, string bodyType, double radiusKm, Color color)
    {
        var bodyNode = new Node2D { Name = name };

        var circle = new Polygon2D();
        circle.Color = color;

        var visualRadius = CalculateVisualRadius(bodyType, radiusKm);
        circle.Polygon = CreateCirclePolygon(visualRadius);

        bodyNode.AddChild(circle);
        return bodyNode;
    }

    /// <summary>
    /// Create a star node with luminosity-based scaling.
    /// </summary>
    public static Node2D CreateStarNode(string name, float luminosity, Color color)
    {
        var starNode = new Node2D { Name = name };

        var circle = new Polygon2D();
        circle.Color = Colors.White; // Will be modulated by parent

        var starRadius = CalculateStarRadius(luminosity);
        circle.Polygon = CreateCirclePolygon(starRadius, 24);

        starNode.AddChild(circle);
        starNode.Modulate = color;

        return starNode;
    }
}