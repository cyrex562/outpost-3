using Godot;

public partial class DebugSettings: Node
{
    private const bool DEBUG_EVENT_PERSISTENCE = false;

    public bool DebugEventPersistence => DEBUG_EVENT_PERSISTENCE;

    // Debug flags
    private const bool DEBUG_PANNING = false;  // Set to true to see panning logs

    public bool DebugPanning => DEBUG_PANNING;

    private const bool DEBUG_SELECTION = false;  // Star selection logs (was flooding console with mouse motion events)

    public bool DebugSelection => DEBUG_SELECTION;

    private const bool DEBUG_ACTIONS = true;    // Probe launch, view details, etc.

    public bool DebugActions => DEBUG_ACTIONS;

    private const bool DEBUG_TIME_CONTROLS = true; // Time control logs

    public bool DebugTimeControls => DEBUG_TIME_CONTROLS;

    private const bool DEBUG_RENDERING = false; // RenderGalaxy/RenderProbes logs

    public bool DebugRendering => DEBUG_RENDERING;

    private const bool DEBUG_STAR_CREATION = false; // CreateStarNode detailed logs 

    public bool DebugStarCreation => DEBUG_STAR_CREATION;
}