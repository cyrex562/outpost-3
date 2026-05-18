using System.Text.Json;
using System.Text.Json.Serialization;
using OutpostGame.Core.Colony;

namespace OutpostGame.Core.Content;

/// <summary>
/// I/O-free content parser: turns JSON content strings into
/// <see cref="BuildingDefinition"/> / <see cref="ResourceDefinition"/> sets.
/// All file I/O happens in the caller (tests pass strings inline; the Godot
/// runtime reads <c>res://content/*.json</c> via Godot.FileAccess).
/// </summary>
public static class ContentLoader
{
    private static readonly JsonSerializerOptions _jsonOpts = new()
    {
        PropertyNameCaseInsensitive = true,
        Converters = { new JsonStringEnumConverter() },
        ReadCommentHandling = JsonCommentHandling.Skip,
        AllowTrailingCommas = true,
    };

    public sealed class ContentValidationException : Exception
    {
        public ContentValidationException(string message) : base(message) { }
    }

    // ── Public entry points ──────────────────────────────────────────────────

    public static IReadOnlyDictionary<string, BuildingDefinition> LoadBuildings(string json)
    {
        var payload = Deserialize<BuildingsPayload>(json, "buildings");
        if (payload.Buildings == null)
            throw new ContentValidationException("Missing 'buildings' array");

        var defs = new Dictionary<string, BuildingDefinition>();
        foreach (var b in payload.Buildings)
        {
            var def = ValidateAndConvertBuilding(b);
            if (defs.ContainsKey(def.Id))
                throw new ContentValidationException($"Duplicate building id: {def.Id}");
            defs[def.Id] = def;
        }
        return defs;
    }

    public static IReadOnlyDictionary<string, ResourceDefinition> LoadResources(string json)
    {
        var payload = Deserialize<ResourcesPayload>(json, "resources");
        if (payload.Resources == null)
            throw new ContentValidationException("Missing 'resources' array");

        var defs = new Dictionary<string, ResourceDefinition>();
        foreach (var r in payload.Resources)
        {
            var def = ValidateAndConvertResource(r);
            if (defs.ContainsKey(def.Id))
                throw new ContentValidationException($"Duplicate resource id: {def.Id}");
            defs[def.Id] = def;
        }
        return defs;
    }

    public static IReadOnlyDictionary<string, EventDefinition> LoadEvents(string json)
    {
        var payload = Deserialize<EventsPayload>(json, "events");
        if (payload.Events == null)
            throw new ContentValidationException("Missing 'events' array");

        var defs = new Dictionary<string, EventDefinition>();
        foreach (var e in payload.Events)
        {
            var def = ValidateAndConvertEvent(e);
            if (defs.ContainsKey(def.Id))
                throw new ContentValidationException($"Duplicate event id: {def.Id}");
            defs[def.Id] = def;
        }
        return defs;
    }

    public static IReadOnlyDictionary<string, TechDefinition> LoadTech(string json)
    {
        var payload = Deserialize<TechPayload>(json, "tech");
        if (payload.Tech == null)
            throw new ContentValidationException("Missing 'tech' array");

        var defs = new Dictionary<string, TechDefinition>();
        foreach (var t in payload.Tech)
        {
            var def = ValidateAndConvertTech(t);
            if (defs.ContainsKey(def.Id))
                throw new ContentValidationException($"Duplicate tech id: {def.Id}");
            defs[def.Id] = def;
        }

        // Cross-reference pass — verify every prerequisite is a known tech id.
        foreach (var def in defs.Values)
        {
            foreach (var prereq in def.Prerequisites)
            {
                if (!defs.ContainsKey(prereq))
                    throw new ContentValidationException(
                        $"tech[{def.Id}]: unknown prerequisite '{prereq}'");
            }
        }
        return defs;
    }

    // ── Validation + conversion ──────────────────────────────────────────────

    private static BuildingDefinition ValidateAndConvertBuilding(BuildingDto dto)
    {
        Require(dto.Id, "building.id");
        Require(dto.DisplayName, $"building[{dto.Id}].displayName");
        if (dto.Size is null)
            throw new ContentValidationException($"building[{dto.Id}]: size is required");
        if (dto.Size.Width <= 0 || dto.Size.Height <= 0)
            throw new ContentValidationException($"building[{dto.Id}]: size dimensions must be positive");
        if (dto.ConstructionTurns < 0)
            throw new ContentValidationException($"building[{dto.Id}]: constructionTurns cannot be negative");
        if (dto.LaborRequired < 0)
            throw new ContentValidationException($"building[{dto.Id}]: laborRequired cannot be negative");

        var cost = dto.ConstructionCost ?? new Dictionary<string, int>();
        var maintenance = dto.MaintenanceCost; // null is meaningful — keep it
        var recipe = ConvertRecipe(dto.Recipe, dto.Id!);

        if (dto.HousingCapacity < 0)
            throw new ContentValidationException(
                $"building[{dto.Id}]: housingCapacity cannot be negative");
        if (dto.StorageCapacity < 0)
            throw new ContentValidationException(
                $"building[{dto.Id}]: storageCapacity cannot be negative");

        // Default fleet / operator requirements from footprint area when the
        // JSON omits them. 1 cell → 1 slot, up to 4×4 = 3 slots. int.MinValue
        // is the "not specified" sentinel from the DTO default.
        int defaultSlots = ScaleFromFootprint(dto.Size.Width, dto.Size.Height);
        int fleetSlots = dto.RequiredFleetSlots == int.MinValue ? defaultSlots : dto.RequiredFleetSlots;
        int operators  = dto.RequiredOperators  == int.MinValue ? defaultSlots : dto.RequiredOperators;

        if (fleetSlots < 0)
            throw new ContentValidationException(
                $"building[{dto.Id}]: requiredFleetSlots cannot be negative");
        if (operators < 0)
            throw new ContentValidationException(
                $"building[{dto.Id}]: requiredOperators cannot be negative");

        return new BuildingDefinition(
            Id: dto.Id!,
            DisplayName: dto.DisplayName!,
            Size: new GridSize(dto.Size.Width, dto.Size.Height),
            ConstructionCost: cost,
            ConstructionTurns: dto.ConstructionTurns,
            PowerConsumption: dto.PowerConsumption,
            PowerProduction: dto.PowerProduction,
            LaborRequired: dto.LaborRequired,
            Recipe: recipe,
            MaintenanceCost: maintenance,
            IsEssential: dto.IsEssential,
            IsSolar: dto.IsSolar,
            PrimarySkill: dto.PrimarySkill,
            Description: dto.Description ?? "",
            Category: dto.Category,
            HousingCapacity: dto.HousingCapacity,
            StorageCapacity: dto.StorageCapacity,
            UpgradesTo: dto.UpgradesTo,
            RequiredFleetSlots: fleetSlots,
            RequiredOperators: operators
        );
    }

    private static int ScaleFromFootprint(int w, int h)
    {
        int area = Math.Max(1, w) * Math.Max(1, h);
        if (area >= 16) return 3;  // 4×4
        if (area >= 9)  return 2;  // 3×3
        return 1;                  // 2×2, 2×3, smaller
    }

    private static ProductionRecipe? ConvertRecipe(RecipeDto? dto, string buildingId)
    {
        if (dto == null) return null;
        if (dto.TurnsPerCycle <= 0)
            throw new ContentValidationException(
                $"building[{buildingId}].recipe.turnsPerCycle must be > 0");
        var inputs  = dto.Inputs  ?? new Dictionary<string, float>();
        var outputs = dto.Outputs ?? new Dictionary<string, float>();
        return new ProductionRecipe(inputs, outputs, dto.TurnsPerCycle);
    }

    private static ResourceDefinition ValidateAndConvertResource(ResourceDto dto)
    {
        Require(dto.Id, "resource.id");
        Require(dto.DisplayName, $"resource[{dto.Id}].displayName");
        if (dto.BaseWeight < 0)
            throw new ContentValidationException(
                $"resource[{dto.Id}]: baseWeight cannot be negative");

        return new ResourceDefinition(
            Id: dto.Id!,
            DisplayName: dto.DisplayName!,
            Tier: dto.Tier,
            Category: dto.Category,
            BaseWeight: dto.BaseWeight,
            Description: dto.Description ?? ""
        );
    }

    private static readonly HashSet<string> _knownOutcomeKinds = new()
    {
        "resource_change", "log", "damage_random",
        "dust_storm", "population_add", "spawn_decision",
    };

    private static EventDefinition ValidateAndConvertEvent(EventDto dto)
    {
        Require(dto.Id, "event.id");
        Require(dto.Title, $"event[{dto.Id}].title");
        if (dto.Trigger is null)
            throw new ContentValidationException($"event[{dto.Id}]: trigger is required");
        if (dto.Trigger.Probability < 0f || dto.Trigger.Probability > 1f)
            throw new ContentValidationException(
                $"event[{dto.Id}]: trigger.probability must be in [0, 1]");
        if (dto.Trigger.MinSol < 0)
            throw new ContentValidationException($"event[{dto.Id}]: minSol must be >= 0");

        var trigger = new EventTrigger(
            Type: dto.Trigger.Type,
            Probability: dto.Trigger.Probability,
            MinSol: dto.Trigger.MinSol,
            RequiredResources: dto.Trigger.RequiredResources ?? new Dictionary<string, float>()
        );

        IReadOnlyList<EventOutcome> outcomes = (dto.Outcomes ?? new List<EventOutcomeDto>())
            .Select(o => ConvertOutcome(o, dto.Id!))
            .ToList();

        IReadOnlyList<EventChoiceDefinition> choices = (dto.Choices ?? new List<EventChoiceDto>())
            .Select(c => new EventChoiceDefinition(
                Label: c.Label ?? "(unlabeled)",
                ConsequenceSummary: c.ConsequenceSummary ?? "",
                Outcomes: (c.Outcomes ?? new List<EventOutcomeDto>())
                    .Select(o => ConvertOutcome(o, dto.Id!))
                    .ToList()))
            .ToList();

        return new EventDefinition(
            Id: dto.Id!,
            Title: dto.Title!,
            Severity: dto.Severity,
            Trigger: trigger,
            Outcomes: outcomes,
            Choices: choices,
            Description: dto.Description ?? ""
        );
    }

    private static EventOutcome ConvertOutcome(EventOutcomeDto dto, string eventId)
    {
        if (string.IsNullOrWhiteSpace(dto.Kind))
            throw new ContentValidationException($"event[{eventId}]: outcome.kind is required");
        if (!_knownOutcomeKinds.Contains(dto.Kind))
            throw new ContentValidationException(
                $"event[{eventId}]: unknown outcome.kind '{dto.Kind}'");

        return new EventOutcome(
            Kind: dto.Kind,
            Resource: dto.Resource,
            Amount: dto.Amount,
            Message: dto.Message,
            Duration: dto.Duration,
            DecisionId: dto.DecisionId
        );
    }

    private static TechDefinition ValidateAndConvertTech(TechDto dto)
    {
        Require(dto.Id, "tech.id");
        Require(dto.DisplayName, $"tech[{dto.Id}].displayName");
        if (dto.Tier < 1)
            throw new ContentValidationException(
                $"tech[{dto.Id}]: tier must be >= 1");
        if (dto.ResearchCost < 0)
            throw new ContentValidationException(
                $"tech[{dto.Id}]: researchCost cannot be negative");
        if (dto.ResearchTurns < 0)
            throw new ContentValidationException(
                $"tech[{dto.Id}]: researchTurns cannot be negative");

        IReadOnlyList<string> AsList(List<string>? src) =>
            (IReadOnlyList<string>?)src ?? Array.Empty<string>();

        var unlocks = new TechUnlocks(
            Buildings: AsList(dto.Unlocks?.Buildings),
            Resources: AsList(dto.Unlocks?.Resources),
            Bonuses:   AsList(dto.Unlocks?.Bonuses)
        );

        return new TechDefinition(
            Id: dto.Id!,
            DisplayName: dto.DisplayName!,
            Category: dto.Category,
            Tier: dto.Tier,
            ResearchCost: dto.ResearchCost,
            ResearchTurns: dto.ResearchTurns,
            Prerequisites: AsList(dto.Prerequisites),
            Unlocks: unlocks,
            Description: dto.Description ?? ""
        );
    }

    private static void Require(string? value, string field)
    {
        if (string.IsNullOrWhiteSpace(value))
            throw new ContentValidationException($"Missing required field: {field}");
    }

    private static T Deserialize<T>(string json, string label)
    {
        try
        {
            var result = JsonSerializer.Deserialize<T>(json, _jsonOpts);
            if (result is null)
                throw new ContentValidationException($"Content payload for '{label}' is empty");
            return result;
        }
        catch (JsonException ex)
        {
            throw new ContentValidationException(
                $"Failed to parse content '{label}': {ex.Message}");
        }
    }

    // ── DTOs (internal — JSON shape) ─────────────────────────────────────────

    private sealed class BuildingsPayload
    {
        public List<BuildingDto>? Buildings { get; set; }
    }

    private sealed class BuildingDto
    {
        public string? Id { get; set; }
        public string? DisplayName { get; set; }
        public string? Description { get; set; }
        public BuildingCategory Category { get; set; } = BuildingCategory.Production;
        public SizeDto? Size { get; set; }
        public Dictionary<string, int>? ConstructionCost { get; set; }
        public int ConstructionTurns { get; set; }
        public float PowerConsumption { get; set; }
        public float PowerProduction { get; set; }
        public int LaborRequired { get; set; }
        public RecipeDto? Recipe { get; set; }
        public Dictionary<string, float>? MaintenanceCost { get; set; }
        public bool IsEssential { get; set; }
        public bool IsSolar { get; set; }
        public SkillType PrimarySkill { get; set; } = SkillType.Laborer;
        public int HousingCapacity { get; set; }
        public float StorageCapacity { get; set; }
        public string? UpgradesTo { get; set; }
        // Use int.MinValue as the "not specified — derive from footprint" sentinel
        // so 0 (valid for Lander / multi_habitat) stays distinguishable.
        public int RequiredFleetSlots { get; set; } = int.MinValue;
        public int RequiredOperators  { get; set; } = int.MinValue;
    }

    private sealed class SizeDto
    {
        public int Width { get; set; }
        public int Height { get; set; }
    }

    private sealed class RecipeDto
    {
        public Dictionary<string, float>? Inputs { get; set; }
        public Dictionary<string, float>? Outputs { get; set; }
        public int TurnsPerCycle { get; set; }
    }

    private sealed class ResourcesPayload
    {
        public List<ResourceDto>? Resources { get; set; }
    }

    private sealed class ResourceDto
    {
        public string? Id { get; set; }
        public string? DisplayName { get; set; }
        public ResourceTier Tier { get; set; }
        public ResourceCategory Category { get; set; }
        public float BaseWeight { get; set; }
        public string? Description { get; set; }
    }

    private sealed class TechPayload
    {
        public List<TechDto>? Tech { get; set; }
    }

    private sealed class TechDto
    {
        public string? Id { get; set; }
        public string? DisplayName { get; set; }
        public TechCategory Category { get; set; }
        public int Tier { get; set; }
        public float ResearchCost { get; set; }
        public int ResearchTurns { get; set; }
        public List<string>? Prerequisites { get; set; }
        public TechUnlocksDto? Unlocks { get; set; }
        public string? Description { get; set; }
    }

    private sealed class TechUnlocksDto
    {
        public List<string>? Buildings { get; set; }
        public List<string>? Resources { get; set; }
        public List<string>? Bonuses { get; set; }
    }

    private sealed class EventsPayload
    {
        public List<EventDto>? Events { get; set; }
    }

    private sealed class EventDto
    {
        public string? Id { get; set; }
        public string? Title { get; set; }
        public string? Description { get; set; }
        public ColonyEventSeverity Severity { get; set; }
        public EventTriggerDto? Trigger { get; set; }
        public List<EventOutcomeDto>? Outcomes { get; set; }
        public List<EventChoiceDto>? Choices { get; set; }
    }

    private sealed class EventTriggerDto
    {
        public EventTriggerType Type { get; set; }
        public float Probability { get; set; }
        public int MinSol { get; set; }
        public Dictionary<string, float>? RequiredResources { get; set; }
    }

    private sealed class EventOutcomeDto
    {
        public string? Kind { get; set; }
        public string? Resource { get; set; }
        public float Amount { get; set; }
        public string? Message { get; set; }
        public int Duration { get; set; }
        public string? DecisionId { get; set; }
    }

    private sealed class EventChoiceDto
    {
        public string? Label { get; set; }
        public string? ConsequenceSummary { get; set; }
        public List<EventOutcomeDto>? Outcomes { get; set; }
    }
}
