using System;
using System.Globalization;

namespace HarshRealm.Data;

public readonly record struct PolarPosition(double DistanceKm, double AngleRadians);

public readonly record struct CartesianPosition(double X, double Y);

public sealed class OrbitalParameters
{
    public double SemiMajorAxisKm { get; }
    public double Eccentricity { get; }
    public double OrbitalPeriodDays { get; }
    public double MeanAnomalyDegrees { get; }

    public OrbitalParameters(double semiMajorAxisKm, double eccentricity, double orbitalPeriodDays, double meanAnomalyDegrees)
    {
        SemiMajorAxisKm = semiMajorAxisKm;
        Eccentricity = eccentricity;
        OrbitalPeriodDays = orbitalPeriodDays;
        MeanAnomalyDegrees = meanAnomalyDegrees;
    }

    public double MeanAnomalyRadians => MeanAnomalyDegrees * Math.PI / 180.0;
}

public sealed class OrbitalState
{
    private const double SignificantThresholdDegreesDefault = 1.0;

    public OrbitalParameters Parameters { get; }
    public PolarPosition CurrentPosition { get; private set; }
    public DateTime CurrentDate { get; private set; }

    public OrbitalState(OrbitalParameters parameters, DateTime startDate)
    {
        Parameters = parameters;
        var initialAngle = NormalizeAngle(parameters.MeanAnomalyRadians);
        var initialDistance = CalculateDistanceFromAngle(parameters.SemiMajorAxisKm, parameters.Eccentricity, initialAngle);
        CurrentPosition = new PolarPosition(initialDistance, initialAngle);
        CurrentDate = startDate;
    }

    public void UpdatePosition(double daysElapsed)
    {
        var meanMotion = 2.0 * Math.PI / Parameters.OrbitalPeriodDays;
        var meanAnomalyChange = meanMotion * daysElapsed;
        var newMeanAnomaly = CurrentPosition.AngleRadians + meanAnomalyChange;
        var trueAnomaly = MeanAnomalyToTrueAnomaly(newMeanAnomaly, Parameters.Eccentricity);
        var newDistance = CalculateDistanceFromAngle(Parameters.SemiMajorAxisKm, Parameters.Eccentricity, trueAnomaly);

        CurrentPosition = new PolarPosition(newDistance, NormalizeAngle(trueAnomaly));
        CurrentDate = CurrentDate.AddDays(daysElapsed);
    }

    public bool IsSignificantChange(double previousAngleRadians, double thresholdDegrees = SignificantThresholdDegreesDefault)
    {
        var angleDiff = Math.Abs(CurrentPosition.AngleRadians - previousAngleRadians);
        var angleDiffDegrees = angleDiff * 180.0 / Math.PI;
        return angleDiffDegrees >= thresholdDegrees;
    }

    public CartesianPosition ToCartesian()
    {
        return new CartesianPosition(
            CurrentPosition.DistanceKm * Math.Cos(CurrentPosition.AngleRadians),
            CurrentPosition.DistanceKm * Math.Sin(CurrentPosition.AngleRadians));
    }

    public double AngleDegrees => CurrentPosition.AngleRadians * 180.0 / Math.PI;

    public string FormattedDate => CurrentDate.ToString("yyyy MMMM dd", CultureInfo.InvariantCulture);

    private static double NormalizeAngle(double angle)
    {
        var twoPi = Math.PI * 2.0;
        var normalized = angle % twoPi;
        return normalized < 0 ? normalized + twoPi : normalized;
    }

    private static double MeanAnomalyToTrueAnomaly(double meanAnomaly, double eccentricity)
    {
        if (eccentricity < 0.1)
        {
            return meanAnomaly + eccentricity * Math.Sin(meanAnomaly);
        }

        var eccentricAnomaly = meanAnomaly;
        for (var i = 0; i < 5; i++)
        {
            var delta = (eccentricAnomaly - eccentricity * Math.Sin(eccentricAnomaly) - meanAnomaly) /
                        (1.0 - eccentricity * Math.Cos(eccentricAnomaly));
            eccentricAnomaly -= delta;
        }

        var numerator = Math.Sqrt(1.0 + eccentricity) * Math.Sin(eccentricAnomaly / 2.0);
        var denominator = Math.Sqrt(1.0 - eccentricity) * Math.Cos(eccentricAnomaly / 2.0);
        return 2.0 * Math.Atan2(numerator, denominator);
    }

    private static double CalculateDistanceFromAngle(double semiMajorAxis, double eccentricity, double trueAnomaly)
    {
        return semiMajorAxis * (1.0 - eccentricity * eccentricity) / (1.0 + eccentricity * Math.Cos(trueAnomaly));
    }
}
