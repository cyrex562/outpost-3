
using Godot;
namespace Outpost3;

public partial class App : Node
{
	public override void _Ready()
	{
		GD.Print("Outpost3 - started");
		GD.Print("Godot version: " + Engine.GetVersionInfo()["string"]);
	}

	public override void _Process(double delta)
	{
	}
}
