using Godot;
using Outpost3.Core.Commands;
using Outpost3.Core;
using System;

namespace Outpost3.UI;

public partial class GameHUD : CanvasLayer
{
	private Godot.Label _timeLabel;
	private Godot.Button _advanceButton;
	private Godot.Button _launchProbeButton;

	private CheckBox _autoCheckBox;
	private ItemList _probeList;
	private ItemList _systemList;
	private StateStore _stateStore;

	private bool _autoAdvance = false;
	private double _autoAdvanceTimer = 0;

	public override void _Ready()
	{
		GD.Print("GameHUD _Ready() called");
		_timeLabel = GetNode<Godot.Label>("PanelContainer/VBoxContainer/TimeLabel");
		_advanceButton = GetNode<Godot.Button>("PanelContainer/VBoxContainer/ButtonBar1/AdvanceButton");
		_launchProbeButton = GetNode<Godot.Button>("PanelContainer/VBoxContainer/ButtonBar1/LaunchProbeButton");
		_probeList = GetNode<ItemList>("PanelContainer/VBoxContainer/ProbeList");
		_systemList = GetNode<ItemList>("PanelContainer/VBoxContainer/SystemList");
		_autoCheckBox = GetNode<CheckBox>("PanelContainer/VBoxContainer/ButtonBar1/AutoCheckBox");

		// get statestore
		_stateStore = GetNode<StateStore>("/root/Main/StateStore");
		// connect signals
		_advanceButton.Pressed += OnAdvancePressed;
		_launchProbeButton.Pressed += OnLaunchProbePressed;
		_stateStore.StateChanged += OnStateChanged;
		_autoCheckBox.Toggled += OnAutoToggled;

		UpdateUI();
	}

	private void OnAutoToggled(bool pressed)
	{
		_autoAdvance = pressed;
		GD.Print($"Auto advance toggled: {_autoAdvance}");
	}

	public override void _Process(double delta)
	{
		if (_autoAdvance)
		{
			_autoAdvanceTimer += delta;
			if (_autoAdvanceTimer >= 1.0) // advance every 1 second
			{
				_autoAdvanceTimer = 0;
				_stateStore.ApplyCommand(new AdvanceTime(1.0));
			}
		}
	}

	private void OnAdvancePressed()
	{
		GD.Print("Advance button pressed");
		var command = new AdvanceTime(10.0);
		_stateStore.ApplyCommand(command);
	}

	private void OnLaunchProbePressed()
	{
		GD.Print("Launch Probe button pressed");
		var newId = Ulid.NewUlid();
		var command = new LaunchProbe(newId);
		_stateStore.ApplyCommand(command);
	}

	private void OnStateChanged()
	{
		GD.Print("State changed signal received in GameHUD");
		UpdateUI();
	}

	private void UpdateUI()
	{
		GD.Print("Updating UI in GameHUD");
		// update time
		var state = _stateStore.State;
		var timeText = $"Game Time: {state.GameTime:F1}h";
		_timeLabel.Text = timeText;
		// update probe list
		_probeList.Clear();
		foreach (var probe in state.ProbesInFlight)
		{
			var remaining = probe.TimeRemaining(state.GameTime);
			var text = $"Probe {probe.Id} to {probe.TargetSystemId}, ETA {remaining:F1}h";
			_probeList.AddItem(text);
		}

		if (state.ProbesInFlight.Count == 0)
		{
			_probeList.AddItem("No probes in flight");
		}

		// update system list
		_systemList.Clear();
		foreach (var system in state.Systems)
		{
			var text = $"System {system.Name} Class-{system.SpectralClass} {system.Bodies.Count} bodies";
			_systemList.AddItem(text);
		}

		if (state.Systems.Count == 0)
		{
			_systemList.AddItem("No systems discovered");
		}
	}
}
