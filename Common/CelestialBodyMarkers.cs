namespace HarshRealm.Data;

public sealed class CelestialBodyRenderable
{
}

public sealed class PreviousPosition
{
    public PreviousPosition(double angleRadians)
    {
        AngleRadians = angleRadians;
    }

    public double AngleRadians { get; set; }
}
