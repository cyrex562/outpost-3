using Godot;
using System;
using System.Threading;

namespace Outpost3.Features;

public partial class PolarPosition : Node
{
    public double Distance { get; set; }
    public double Angle { get; set; }
}

public partial class CartesianPosition : Node
{
    public double X { get; set; }
    public double Y { get; set; }
}

public partial class OrbitalParameters : Node
{
    public double SemiMajorAxisKm { get; set; }
    public double Eccentricity { get; set; }
    public double OrbitalPeriodDays { get; set; }
    public double MeanAnomalyDegrees { get; set; }

    public double MeanAnomalyRadians => MeanAnomalyDegrees * Math.PI / 180.0;
}

public partial class OrbitalState : Node
{
    private const double SignificantThresholdDegreesDefault = 1.0;

    public OrbitalParameters Parameters { get; set; }
    public PolarPosition CurrentPosition { get; set; }
    public DateTime CurrentDate { get; set; }

    public OrbitalState()
    {
        CurrentPosition = new PolarPosition();
        CurrentDate = DateTime.Now;
        Parameters = new OrbitalParameters();
    }

    public static OrbitalState Create(OrbitalParameters parameters, DateTime startDate)
    {
        var state = new OrbitalState
        {
            Parameters = parameters,
            CurrentDate = startDate
        };
        var initialDistance = state.CalculateDistanceFromAngle(parameters.SemiMajorAxisKm, parameters.Eccentricity, parameters.MeanAnomalyRadians);
        state.CurrentPosition = new PolarPosition
        {
            Distance = initialDistance,
            Angle = parameters.MeanAnomalyRadians
        };
        return state;
    }

    public void UpdatePosition(double daysElapsed)
    {
        var meanMotion = 2.0 * Math.PI / Parameters.OrbitalPeriodDays;
        var meanAnomalyChange = meanMotion * daysElapsed;
        var newMeanAnomaly = CurrentPosition.Angle + meanAnomalyChange;
        var trueAnomaly = MeanAnomalyToTrueAnomaly(newMeanAnomaly, Parameters.Eccentricity);
        var newDistance = CalculateDistanceFromAngle(Parameters.SemiMajorAxisKm, Parameters.Eccentricity, trueAnomaly);

        CurrentPosition = new PolarPosition
        {
            Distance = newDistance,
            Angle = trueAnomaly
        };
        CurrentDate = CurrentDate.AddDays(daysElapsed);
    }

    double MeanAnomalyToTrueAnomaly(double meanAnomaly, double eccentricity)
    {
        // Using Newton-Raphson method to solve Kepler's equation: M = E - e*sin(E)
        double E = meanAnomaly; // Initial guess
        for (int i = 0; i < 10; i++)
        {
            double f = E - eccentricity * Math.Sin(E) - meanAnomaly;
            double fPrime = 1 - eccentricity * Math.Cos(E);
            E -= f / fPrime;
        }
        // Convert eccentric anomaly to true anomaly
        double trueAnomaly = 2 * Math.Atan2(Math.Sqrt(1 + eccentricity) * Math.Sin(E / 2), Math.Sqrt(1 - eccentricity) * Math.Cos(E / 2));
        return trueAnomaly;
    }

    double CalculateDistanceFromAngle(double semiMajorAxis, double eccentricity, double angle)
    {
        return (semiMajorAxis * (1 - eccentricity * eccentricity)) / (1 + eccentricity * Math.Cos(angle));
    }

    public CartesianPosition ToCartesian()
    {
        return new CartesianPosition
        {
            X = CurrentPosition.Distance * Math.Cos(CurrentPosition.Angle),
            Y = CurrentPosition.Distance * Math.Sin(CurrentPosition.Angle)
        };
    }

    public bool IsSignificantChange(double previousAngle, double thresholdDegrees = SignificantThresholdDegreesDefault)
    {
        var angleDiff = Math.Abs(CurrentPosition.Angle - previousAngle);
        var angleDiffDegrees = angleDiff * 180.0 / Math.PI;
        return angleDiffDegrees >= thresholdDegrees;
    }

    public double AngleDegrees => CurrentPosition.Angle * 180.0 / Math.PI;
    public string FormattedDate => CurrentDate.ToString("yyyy MMMM dd", System.Globalization.CultureInfo.InvariantCulture);
}