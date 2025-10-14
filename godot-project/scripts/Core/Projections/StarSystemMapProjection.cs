using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.Core.Projections;

/// <summary>
/// Projection for the Star System Map screen.
/// Transforms domain state into UI-friendly view models.
/// </summary>
public static class StarSystemMapProjection
{
    /// <summary>
    /// Projects the current game state into a star system map view model.
    /// Returns null if no system is selected.
    /// </summary>
    public static StarSystemMapViewModel? Project(GameState state)
    {
        if (!state.SelectedSystemId.HasValue)
        {
            return null;
        }

        var systemId = state.SelectedSystemId.Value;
        var system = state.Systems.FirstOrDefault(s => s.Id == systemId);

        if (system == null)
        {
            return null;
        }

        // Get camera state for this system (or default)
        var cameraState = state.CameraStates.TryGetValue(systemId, out var cam)
            ? cam
            : new CameraState(new Vector2(0, 0), 1.0f);

        // Map bodies to view models
        var bodyViewModels = system.Bodies
            .Select(b => new CelestialBodyViewModel(
                Id: b.Id,
                Name: b.Name,
                BodyType: b.BodyType,
                OrbitalParams: b.OrbitalParams,
                MassEarthMasses: b.MassEarthMasses,
                RadiusKm: b.RadiusKm,
                Color: GetBodyColor(b.BodyType, b.Composition),
                IsSelected: state.SelectedBodyId.HasValue && state.SelectedBodyId.Value == b.Id
            ))
            .ToList();

        // Map belts to view models
        var beltViewModels = system.Belts
            .Select(b => new AsteroidBeltViewModel(
                Id: b.Id,
                Name: b.Name,
                InnerRadiusAU: b.InnerRadiusAU,
                OuterRadiusAU: b.OuterRadiusAU,
                Color: new Color(0.6f, 0.5f, 0.4f, 0.3f) // Brownish, semi-transparent
            ))
            .ToList();

        // Oort cloud view model (if present)
        var oortCloudViewModel = system.OortCloud != null
            ? new OortCloudViewModel(
                RadiusAU: system.OortCloud.RadiusAU,
                Color: new Color(0.5f, 0.6f, 0.7f, 0.15f) // Icy blue, very transparent
            )
            : null;

        return new StarSystemMapViewModel(
            SystemId: systemId,
            SystemName: system.Name,
            SpectralClass: system.SpectralClass,
            StarColor: GetSpectralClassColor(system.SpectralClass),
            StarLuminosity: system.Luminosity,
            Bodies: bodyViewModels,
            Belts: beltViewModels,
            OortCloud: oortCloudViewModel,
            CameraState: cameraState,
            OverviewPanelOpen: state.SystemOverviewPanelOpen,
            CurrentSpeed: state.CurrentSpeed,
            IsPaused: state.IsPaused,
            GameTime: state.GameTime
        );
    }

    /// <summary>
    /// Gets the color for a celestial body based on type and composition.
    /// </summary>
    private static Color GetBodyColor(string bodyType, string composition)
    {
        // First check composition for more specific colors
        return composition.ToLowerInvariant() switch
        {
            var c when c.Contains("gas giant") => new Color(0.9f, 0.7f, 0.4f), // Orange-brown
            var c when c.Contains("ice giant") => new Color(0.5f, 0.7f, 0.9f), // Pale blue
            var c when c.Contains("ice") => new Color(0.8f, 0.9f, 1.0f), // White-blue
            var c when c.Contains("rock") => new Color(0.6f, 0.5f, 0.4f), // Brown-grey
            var c when c.Contains("metal") => new Color(0.7f, 0.7f, 0.7f), // Grey
            var c when c.Contains("lava") => new Color(1.0f, 0.3f, 0.1f), // Red-orange
            _ => GetBodyTypeColor(bodyType)
        };
    }

    /// <summary>
    /// Gets default color based on body type.
    /// </summary>
    private static Color GetBodyTypeColor(string bodyType)
    {
        return bodyType.ToLowerInvariant() switch
        {
            var t when t.Contains("planet") => new Color(0.6f, 0.7f, 0.8f), // Blue-grey
            var t when t.Contains("moon") => new Color(0.7f, 0.7f, 0.7f), // Grey
            var t when t.Contains("asteroid") => new Color(0.5f, 0.4f, 0.3f), // Dark brown
            var t when t.Contains("comet") => new Color(0.8f, 0.9f, 1.0f), // Icy white
            _ => new Color(0.8f, 0.8f, 0.8f) // Default grey
        };
    }

    /// <summary>
    /// Gets the color for a star based on spectral class.
    /// </summary>
    public static Color GetSpectralClassColor(string spectralClass)
    {
        return spectralClass.ToUpperInvariant() switch
        {
            "O" => new Color(0.6f, 0.7f, 1.0f),   // Blue
            "B" => new Color(0.7f, 0.8f, 1.0f),   // Blue-white
            "A" => new Color(0.9f, 0.95f, 1.0f),  // White
            "F" => new Color(1.0f, 0.98f, 0.9f),  // Yellow-white
            "G" => new Color(1.0f, 0.95f, 0.7f),  // Yellow (like our Sun)
            "K" => new Color(1.0f, 0.8f, 0.5f),   // Orange
            "M" => new Color(1.0f, 0.5f, 0.3f),   // Red
            _ => new Color(1.0f, 1.0f, 0.8f)      // Default yellowish
        };
    }
}

/// <summary>
/// View model for the star system map screen.
/// Immutable, UI-friendly representation of system state.
/// </summary>
public record StarSystemMapViewModel(
    Ulid SystemId,
    string SystemName,
    string SpectralClass,
    Color StarColor,
    float StarLuminosity,
    List<CelestialBodyViewModel> Bodies,
    List<AsteroidBeltViewModel> Belts,
    OortCloudViewModel? OortCloud,
    CameraState CameraState,
    bool OverviewPanelOpen,
    GameSpeed CurrentSpeed,
    bool IsPaused,
    double GameTime
);

/// <summary>
/// View model for a celestial body.
/// </summary>
public record CelestialBodyViewModel(
    Ulid Id,
    string Name,
    string BodyType,
    OrbitalParameters? OrbitalParams,
    double MassEarthMasses,
    double RadiusKm,
    Color Color,
    bool IsSelected
);

/// <summary>
/// View model for an asteroid belt.
/// </summary>
public record AsteroidBeltViewModel(
    Ulid Id,
    string Name,
    double InnerRadiusAU,
    double OuterRadiusAU,
    Color Color
);

/// <summary>
/// View model for the Oort cloud.
/// </summary>
public record OortCloudViewModel(
    double RadiusAU,
    Color Color
);
