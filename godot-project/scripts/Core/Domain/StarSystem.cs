using System.Collections.Generic;
using System;

namespace Outpost3.Core.Domain;

public record StarSystem
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";
    public string SpectralClass { get; init; } = "";
    public List<CelestialBody> Bodies { get; init; } = new();
}

public record CelestialBody
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";
    public string BodyType { get; init; } = ""; // e.g. Planet, Moon, Asteroid
    public bool Explored { get; init; } = false;
}

