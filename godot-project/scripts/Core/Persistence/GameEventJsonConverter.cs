using System;
using System.Text.Json;
using System.Text.Json.Serialization;
using Outpost3.Core.Events;

namespace Outpost3.Core.Persistence;

/// <summary>
/// Custom JSON converter for polymorphic GameEvent serialization and deserialization.
/// Uses the eventType property to determine which concrete event type to deserialize.
/// </summary>
public class GameEventJsonConverter : JsonConverter<GameEvent>
{
    /// <summary>
    /// Reads and deserializes a GameEvent from JSON.
    /// </summary>
    public override GameEvent Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        // Parse the JSON into a document
        using var jsonDoc = JsonDocument.ParseValue(ref reader);
        var root = jsonDoc.RootElement;

        // Extract the eventType property
        if (!root.TryGetProperty("eventType", out var eventTypeProp))
        {
            throw new JsonException("Missing 'eventType' property in GameEvent JSON.");
        }

        var eventTypeName = eventTypeProp.GetString();
        if (string.IsNullOrEmpty(eventTypeName))
        {
            throw new JsonException("'eventType' property is null or empty.");
        }

        // Map event type name to concrete type
        var concreteType = GetEventType(eventTypeName);

        // Deserialize to the concrete type
        var json = root.GetRawText();
        var evt = JsonSerializer.Deserialize(json, concreteType, options);

        if (evt == null)
        {
            throw new JsonException($"Failed to deserialize event of type {eventTypeName}.");
        }

        return (GameEvent)evt;
    }

    /// <summary>
    /// Writes a GameEvent to JSON, adding the eventType discriminator property.
    /// </summary>
    public override void Write(Utf8JsonWriter writer, GameEvent value, JsonSerializerOptions options)
    {
        writer.WriteStartObject();

        // Write the eventType discriminator first
        writer.WriteString("eventType", value.GetType().Name);

        // Create options without this converter to avoid recursion
        var serializeOptions = new JsonSerializerOptions(options);
        serializeOptions.Converters.Clear();
        foreach (var converter in options.Converters)
        {
            if (converter is not GameEventJsonConverter)
            {
                serializeOptions.Converters.Add(converter);
            }
        }

        // Serialize the object and copy its properties
        var json = JsonSerializer.Serialize(value, value.GetType(), serializeOptions);
        using var doc = JsonDocument.Parse(json);

        foreach (var property in doc.RootElement.EnumerateObject())
        {
            property.WriteTo(writer);
        }

        writer.WriteEndObject();
    }

    /// <summary>
    /// Maps an event type name to its corresponding Type.
    /// </summary>
    /// <param name="eventTypeName">The name of the event type (e.g., "ShipDepartedEvent").</param>
    /// <returns>The Type corresponding to the event type name.</returns>
    /// <exception cref="NotSupportedException">Thrown if the event type is unknown.</exception>
    private static Type GetEventType(string eventTypeName)
    {
        return eventTypeName switch
        {
            // Journey phase events
            "ShipDepartedEvent" => typeof(ShipDepartedEvent),
            "MechanicalFailureEvent" => typeof(MechanicalFailureEvent),
            "SocialConflictEvent" => typeof(SocialConflictEvent),
            "AnomalyDetectedEvent" => typeof(AnomalyDetectedEvent),
            "ShipArrivedEvent" => typeof(ShipArrivedEvent),

            // Exploration events
            "TimeAdvanced" => typeof(TimeAdvanced),
            "ProbeLaunched" => typeof(ProbeLaunched),
            "ProbeArrived" => typeof(ProbeArrived),
            "SystemDiscovered" => typeof(SystemDiscovered),
            "SystemSelected" => typeof(SystemSelected),
            "GalaxyInitialized" => typeof(GalaxyInitialized),
            "SystemScanned" => typeof(SystemScanned),

            // Unknown event type
            _ => throw new NotSupportedException(
                $"Unknown event type: {eventTypeName}. " +
                $"Add mapping in {nameof(GameEventJsonConverter)}.{nameof(GetEventType)}")
        };
    }
}
