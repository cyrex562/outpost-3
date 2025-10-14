using System;

namespace Outpost3.Core.Domain;

/// <summary>
/// Strongly-typed ID for celestial bodies.
/// </summary>
public record BodyId(Ulid Value)
{
    public static BodyId New() => new(Ulid.NewUlid());
    public override string ToString() => Value.ToString();
}

/// <summary>
/// Strongly-typed ID for asteroid belts.
/// </summary>
public record BeltId(Ulid Value)
{
    public static BeltId New() => new(Ulid.NewUlid());
    public override string ToString() => Value.ToString();
}

/// <summary>
/// Strongly-typed ID for star systems.
/// </summary>
public record SystemId(Ulid Value)
{
    public static SystemId New() => new(Ulid.NewUlid());
    public override string ToString() => Value.ToString();
}
