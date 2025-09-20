#nullable enable
using System;
using Godot;

namespace HarshRealm.Data;

public enum CelestialBodyType
{
    Star,
    Planet,
    Moon,
    Asteroid,
    Comet,
    DwarfPlanet,
}

[GlobalClass]
public partial class CelestialBody : Node
{
    [Export] public string BodyName { get; set; } = string.Empty;
    [Export] public string BodyId { get; set; } = string.Empty;
    [Export] public CelestialBodyType BodyType { get; set; }
    [Export] public double Mass { get; set; } // Mass in kg
    [Export] public double Diameter { get; set; } // Diameter in km
    
    public OrbitalState? OrbitalState { get; set; }

    public CelestialBody()
    {
        BodyId = Guid.NewGuid().ToString();
    }

    public static CelestialBody Create(string name, CelestialBodyType bodyType, double mass, double diameter)
    {
        var body = new CelestialBody
        {
            BodyName = name,
            BodyId = Guid.NewGuid().ToString(),
            BodyType = bodyType,
            Mass = mass,
            Diameter = diameter
        };
        return body;
    }
    
    public CelestialBody WithOrbitalState(OrbitalState orbitalState)
    {
        OrbitalState = orbitalState;
        return this;
    }
    
    public void UpdateOrbitalPosition(double daysElapsed)
    {
        OrbitalState?.UpdatePosition(daysElapsed);
    }
    
    public bool HasSignificantPositionChange(double previousAngle, double thresholdDegrees)
    {
        return OrbitalState?.IsSignificantChange(previousAngle, thresholdDegrees) ?? false;
    }

    public override string ToString() => $"{BodyName} ({BodyType})";
}

/// <summary>
/// Marker component for celestial bodies that should be rendered
/// </summary>
[GlobalClass]
public partial class CelestialBodyRenderable : Node
{
    // Marker component - no additional properties needed
}

/// <summary>
/// Component for tracking previous positions for change detection
/// </summary>
[GlobalClass]
public partial class PreviousPosition : Node
{
    [Export] public double Angle { get; set; }
    
    public PreviousPosition() { }
    
    public PreviousPosition(double angle)
    {
        Angle = angle;
    }
}
