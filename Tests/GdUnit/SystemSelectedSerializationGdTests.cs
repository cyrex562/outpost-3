using System;
using System.Text.Json;
using GdUnit4;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;

namespace Outpost3.Tests.GdUnit;

/// <summary>
/// GdUnit4 tests for SystemSelected event serialization and deserialization.
/// Ensures events can be persisted and replayed correctly.
/// </summary>
[TestSuite]
public class SystemSelectedSerializationGdTests
{
    private JsonSerializerOptions _jsonOptions = null!;

    [Before]
    public void Setup()
    {
        _jsonOptions = new JsonSerializerOptions
        {
            Converters = { new GameEventJsonConverter() },
            WriteIndented = true
        };
    }

    [TestCase]
    public void SystemSelected_SerializesToJson()
    {
        // Arrange
        var systemId = Ulid.NewUlid();
        var systemSelectedEvent = new SystemSelected
        {
            SystemId = systemId,
            GameTime = 1234.56f
        };

        // Act
        var json = JsonSerializer.Serialize<GameEvent>(systemSelectedEvent, _jsonOptions);

        // Assert
        Assertions.AssertThat(json).Contains("SystemSelected");
        Assertions.AssertThat(json).Contains(systemId.ToString());
        Assertions.AssertThat(json).Contains("1234.56");
    }

    [TestCase]
    public void SystemSelected_DeserializesFromJson()
    {
        // Arrange
        var systemId = Ulid.NewUlid();
        var originalEvent = new SystemSelected
        {
            SystemId = systemId,
            GameTime = 5678.9f
        };
        var json = JsonSerializer.Serialize<GameEvent>(originalEvent, _jsonOptions);

        // Act
        var deserializedEvent = JsonSerializer.Deserialize<GameEvent>(json, _jsonOptions);

        // Assert
        Assertions.AssertThat(deserializedEvent).IsNotNull();
        Assertions.AssertThat(deserializedEvent).IsInstanceOf<SystemSelected>();
        var systemSelected = (SystemSelected)deserializedEvent!;
        Assertions.AssertThat(systemSelected.SystemId).IsEqual(systemId);
        Assertions.AssertThat(systemSelected.GameTime).IsEqual(5678.9f);
    }

    [TestCase]
    public void SystemSelected_RoundTrip_PreservesData()
    {
        // Arrange
        var systemId = Ulid.NewUlid();
        var originalEvent = new SystemSelected
        {
            SystemId = systemId,
            GameTime = 9999.123f
        };

        // Act: Serialize and deserialize
        var json = JsonSerializer.Serialize<GameEvent>(originalEvent, _jsonOptions);
        var roundTrippedEvent = JsonSerializer.Deserialize<GameEvent>(json, _jsonOptions);

        // Assert: All data preserved
        Assertions.AssertThat(roundTrippedEvent).IsInstanceOf<SystemSelected>();
        var systemSelected = (SystemSelected)roundTrippedEvent!;
        Assertions.AssertThat(systemSelected.SystemId).IsEqual(originalEvent.SystemId);
        Assertions.AssertThat(systemSelected.GameTime).IsEqual(originalEvent.GameTime);
    }

    [TestCase]
    public void SystemSelected_InPolymorphicArray_Deserializes()
    {
        // Arrange: Mix of different event types
        var systemId = Ulid.NewUlid();
        var events = new GameEvent[]
        {
            new ProbeArrived { GameTime = 100.0f },
            new SystemSelected { SystemId = systemId, GameTime = 200.0f },
            new ProbeArrived { GameTime = 300.0f }
        };

        // Act: Serialize and deserialize array
        var json = JsonSerializer.Serialize(events, _jsonOptions);
        var deserializedEvents = JsonSerializer.Deserialize<GameEvent[]>(json, _jsonOptions);

        // Assert
        Assertions.AssertThat(deserializedEvents).IsNotNull();
        Assertions.AssertThat(deserializedEvents!.Length).IsEqual(3);

        Assertions.AssertThat(deserializedEvents[1]).IsInstanceOf<SystemSelected>();
        var systemSelected = (SystemSelected)deserializedEvents[1];
        Assertions.AssertThat(systemSelected.SystemId).IsEqual(systemId);
        Assertions.AssertThat(systemSelected.GameTime).IsEqual(200.0f);
    }

    [TestCase]
    public void SystemSelected_WithUlidEmpty_SerializesCorrectly()
    {
        // Arrange
        var systemSelectedEvent = new SystemSelected
        {
            SystemId = Ulid.Empty,
            GameTime = 0.0f
        };

        // Act
        var json = JsonSerializer.Serialize<GameEvent>(systemSelectedEvent, _jsonOptions);
        var deserialized = JsonSerializer.Deserialize<GameEvent>(json, _jsonOptions);

        // Assert
        Assertions.AssertThat(deserialized).IsInstanceOf<SystemSelected>();
        var systemSelected = (SystemSelected)deserialized!;
        Assertions.AssertThat(systemSelected.SystemId).IsEqual(Ulid.Empty);
    }

    [TestCase]
    public void SystemSelected_MinimalJson_Deserializes()
    {
        // Arrange: Minimal valid JSON with correct "eventType" discriminator
        var systemId = Ulid.NewUlid();
        var minimalJson = $$"""
        {
          "eventType": "SystemSelected",
          "SystemId": "{{systemId}}",
          "GameTime": 42.0
        }
        """;

        // Act
        var deserialized = JsonSerializer.Deserialize<GameEvent>(minimalJson, _jsonOptions);

        // Assert
        Assertions.AssertThat(deserialized).IsInstanceOf<SystemSelected>();
        var systemSelected = (SystemSelected)deserialized!;
        Assertions.AssertThat(systemSelected.SystemId).IsEqual(systemId);
        Assertions.AssertThat(Math.Abs(systemSelected.GameTime - 42.0f)).IsLess(0.001f);
    }

    [TestCase]
    public void GameEventJsonConverter_IncludesSystemSelectedType()
    {
        // Arrange
        var converter = new GameEventJsonConverter();
        var systemSelectedEvent = new SystemSelected
        {
            SystemId = Ulid.NewUlid(),
            GameTime = 100.0f
        };

        // Act: Serialize with converter
        var json = JsonSerializer.Serialize<GameEvent>(systemSelectedEvent, _jsonOptions);

        // Assert: Contains "eventType" discriminator (not "$type")
        Assertions.AssertThat(json).Contains("\"eventType\"");
        Assertions.AssertThat(json).Contains("\"SystemSelected\"");
    }

    [TestCase]
    public void SystemSelected_LargeGameTime_SerializesCorrectly()
    {
        // Arrange: Large game time value
        var systemSelectedEvent = new SystemSelected
        {
            SystemId = Ulid.NewUlid(),
            GameTime = 999999999.999999f
        };

        // Act
        var json = JsonSerializer.Serialize<GameEvent>(systemSelectedEvent, _jsonOptions);
        var deserialized = JsonSerializer.Deserialize<GameEvent>(json, _jsonOptions);

        // Assert
        Assertions.AssertThat(deserialized).IsInstanceOf<SystemSelected>();
        var systemSelected = (SystemSelected)deserialized!;
        // Float precision: check within 1.0 tolerance
        Assertions.AssertThat(Math.Abs(systemSelected.GameTime - 999999999.999999f)).IsLess(1.0f);
    }

    [TestCase]
    public void SystemSelected_ParameterlessConstructor_DeserializesWithDefaults()
    {
        // Arrange: JSON with eventType discriminator but no explicit values
        var json = """
        {
          "eventType": "SystemSelected"
        }
        """;

        // Act
        var deserialized = JsonSerializer.Deserialize<GameEvent>(json, _jsonOptions);

        // Assert: Should use default values
        Assertions.AssertThat(deserialized).IsInstanceOf<SystemSelected>();
        var systemSelected = (SystemSelected)deserialized!;
        Assertions.AssertThat(systemSelected.GameTime).IsEqual(0.0f);
        // SystemId will be default Ulid (all zeros)
    }
}
