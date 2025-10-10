//float CalculateTravelTime(float distanceLy)
//{
	///* Simple model: 1 light-year = 100 game hours */
	//return distanceLy * 100.0f;
//}

/// <summary>
/// Procedurally Generate Star System
/// This currently assumes a single star in the center of the system
/// </summary>
enum SpectralClass
{
	O,
	B,
	A,
	F,
	G,
	K,
	M,
}

enum CelestialBodyType
{
	Star,
	Planet,
	Moon,
	AsteroidBelt,
}
