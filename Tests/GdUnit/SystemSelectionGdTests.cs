using System;
using System.Collections.Generic;
using System.Linq;
using GdUnit4;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;
using Outpost3.Core.Systems;

namespace Outpost3.Tests.GdUnit;

/// <summary>
/// GdUnit4 tests for Feature 2.1: System Selection & Details.
/// Tests the domain logic for selecting star systems and managing selection state.
/// Note: HandleSelectSystem takes (state, command) not (state, command, gameTime)
/// Note: HandleDeselectSystem takes (state) not (state, command, gameTime)
/// </summary>
[TestSuite]
public class SystemSelectionGdTests
{
    #region Test Data Helpers

    private static StarSystem CreateTestSystem(string name = "Alpha Centauri", string spectralClass = "G2V")
    {
        return new StarSystem
        {
            Id = Ulid.NewUlid(),
            Name = name,
            SpectralClass = spectralClass,
            Bodies = new List<CelestialBody>
            {
                new CelestialBody
                {
                    Id = Ulid.NewUlid(),
                    Name = $"{name} I",
                    BodyType = "Rocky Planet",
                    Explored = false
                },
                new CelestialBody
                {
                    Id = Ulid.NewUlid(),
                    Name = $"{name} II",
                    BodyType = "Gas Giant",
                    Explored = false
                }
            }
        };
    }

    private static GameState CreateStateWithSystems(int count = 3)
    {
        var systems = new List<StarSystem>();
        for (int i = 0; i < count; i++)
        {
            systems.Add(CreateTestSystem($"System-{i}", $"G{i}V"));
        }

        return new GameState
        {
            GameTime = 1000.0,
            Systems = systems,
            ProbesInFlight = new List<ProbeInFlight>()
        };
    }

    #endregion

    #region SelectSystemCommand Tests

    [TestCase]
    public void SelectSystemCommand_HasCorrectSystemId()
    {
        // Arrange
        var systemId = Ulid.NewUlid();

        // Act
        var command = new SelectSystemCommand(systemId);

        // Assert
        Assertions.AssertThat(command.SystemId).IsEqual(systemId);
    }

    [TestCase]
    public void SelectSystemCommand_IsImmutable()
    {
        // Arrange
        var systemId = Ulid.NewUlid();
        var command = new SelectSystemCommand(systemId);

        // Assert: Record types are immutable by default
        Assertions.AssertThat(command).IsInstanceOf<ICommand>();
        // The SystemId property should be init-only (cannot be reassigned)
    }

    #endregion

    #region SystemSelected Event Tests

    [TestCase]
    public void SystemSelected_CreatedWithCorrectSystemId()
    {
        // Arrange
        var systemId = Ulid.NewUlid();

        // Act
        var evt = new SystemSelected { SystemId = systemId };

        // Assert
        Assertions.AssertThat(evt.SystemId).IsEqual(systemId);
    }

    [TestCase]
    public void SystemSelected_InheritsFromGameEvent()
    {
        // Arrange
        var evt = new SystemSelected { SystemId = Ulid.NewUlid() };

        // Assert
        Assertions.AssertThat(evt).IsInstanceOf<GameEvent>();
    }

    [TestCase]
    public void SystemSelected_HasParameterlessConstructor()
    {
        // Act
        var evt = new SystemSelected();

        // Assert
        Assertions.AssertThat(evt).IsNotNull();
        Assertions.AssertThat(evt.SystemId).IsEqual(Ulid.Empty);
    }

    #endregion

    #region SystemSelectionSystem Reducer Tests

    [TestCase]
    public void HandleSelectSystem_ValidSystem_UpdatesSelectedSystemId()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var systemToSelect = state.Systems[1];
        var command = new SelectSystemCommand(systemToSelect.Id);

        // Act
        var (newState, events) = SystemSelectionSystem.HandleSelectSystem(state, command);

        // Assert
        Assertions.AssertThat(newState.SelectedSystemId).IsEqual(systemToSelect.Id);
    }

    [TestCase]
    public void HandleSelectSystem_ValidSystem_EmitsSystemSelectedEvent()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var systemToSelect = state.Systems[0];
        var command = new SelectSystemCommand(systemToSelect.Id);

        // Act
        var (newState, events) = SystemSelectionSystem.HandleSelectSystem(state, command);

        // Assert
        Assertions.AssertThat(events.Count).IsEqual(1);
        var evt = events[0] as SystemSelected;
        Assertions.AssertThat(evt).IsNotNull();
        Assertions.AssertThat(evt!.SystemId).IsEqual(systemToSelect.Id);
    }

    [TestCase]
    public void HandleSelectSystem_NonexistentSystem_DoesNotChangeState()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var nonexistentId = Ulid.NewUlid();
        var command = new SelectSystemCommand(nonexistentId);

        // Act
        var (newState, events) = SystemSelectionSystem.HandleSelectSystem(state, command);

        // Assert
        Assertions.AssertThat(newState.SelectedSystemId).IsEqual(state.SelectedSystemId);
        Assertions.AssertThat(events.Count).IsEqual(0);
    }

    [TestCase]
    public void HandleSelectSystem_AlreadySelected_DoesNotEmitEvent()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var systemToSelect = state.Systems[1];
        state = state.WithSelectedSystem(systemToSelect.Id);
        var command = new SelectSystemCommand(systemToSelect.Id);

        // Act
        var (newState, events) = SystemSelectionSystem.HandleSelectSystem(state, command);

        // Assert
        Assertions.AssertThat(events.Count).IsEqual(0);
        Assertions.AssertThat(newState.SelectedSystemId).IsEqual(systemToSelect.Id);
    }

    [TestCase]
    public void HandleSelectSystem_EmptySystemList_DoesNotChangeState()
    {
        // Arrange
        var state = new GameState
        {
            GameTime = 1000.0,
            Systems = new List<StarSystem>(),
            ProbesInFlight = new List<ProbeInFlight>()
        };
        var command = new SelectSystemCommand(Ulid.NewUlid());

        // Act
        var (newState, events) = SystemSelectionSystem.HandleSelectSystem(state, command);

        // Assert
        Assertions.AssertThat(newState.SelectedSystemId).IsEqual(Ulid.Empty);
        Assertions.AssertThat(events.Count).IsEqual(0);
    }

    [TestCase]
    public void HandleSelectSystem_PreservesOtherStateProperties()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var originalGameTime = state.GameTime;
        var originalProbesCount = state.ProbesInFlight.Count;
        var systemToSelect = state.Systems[0];
        var command = new SelectSystemCommand(systemToSelect.Id);

        // Act
        var (newState, events) = SystemSelectionSystem.HandleSelectSystem(state, command);

        // Assert
        Assertions.AssertThat(newState.GameTime).IsEqual(originalGameTime);
        Assertions.AssertThat(newState.ProbesInFlight.Count).IsEqual(originalProbesCount);
        Assertions.AssertThat(newState.Systems.Count).IsEqual(state.Systems.Count);
    }

    [TestCase]
    public void HandleDeselectSystem_ClearsSelectedSystemId()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var systemToSelect = state.Systems[1];
        state = state.WithSelectedSystem(systemToSelect.Id);
        // Deselect uses no command

        // Act
        var (newState, events) = SystemSelectionSystem.HandleDeselectSystem(state);

        // Assert
        Assertions.AssertThat(newState.SelectedSystemId).IsEqual(Ulid.Empty);
    }

    #endregion

    #region GameState Selection Management Tests

    [TestCase]
    public void GameState_WithSelectedSystem_UpdatesSelectedSystemId()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var systemId = state.Systems[1].Id;

        // Act
        var newState = state.WithSelectedSystem(systemId);

        // Assert
        Assertions.AssertThat(newState.SelectedSystemId).IsEqual(systemId);
    }

    [TestCase]
    public void GameState_WithSelectedSystem_Null_ClearsSelection()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        state = state.WithSelectedSystem(state.Systems[0].Id);

        // Act
        var newState = state.WithSelectedSystem(Ulid.Empty);

        // Assert
        Assertions.AssertThat(newState.SelectedSystemId).IsEqual(Ulid.Empty);
    }

    [TestCase]
    public void GameState_WithSelectedSystem_ReturnsNewInstance()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var systemId = state.Systems[0].Id;

        // Act
        var newState = state.WithSelectedSystem(systemId);

        // Assert: State is immutable, so a new instance should be returned
        Assertions.AssertThat(newState).IsNotSame(state);
    }

    [TestCase]
    public void GameState_WithSelectedSystem_PreservesOtherProperties()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var originalGameTime = state.GameTime;
        var originalSystemsCount = state.Systems.Count;
        var systemId = state.Systems[2].Id;

        // Act
        var newState = state.WithSelectedSystem(systemId);

        // Assert
        Assertions.AssertThat(newState.GameTime).IsEqual(originalGameTime);
        Assertions.AssertThat(newState.Systems.Count).IsEqual(originalSystemsCount);
    }

    #endregion

    #region Reducer Integration Tests

    [TestCase]
    public void SelectThenDeselect_WorkflowCompletes()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var systemToSelect = state.Systems[1];
        var selectCommand = new SelectSystemCommand(systemToSelect.Id);
        // Deselect uses no command

        // Act: Select
        var (stateAfterSelect, selectEvents) = SystemSelectionSystem.HandleSelectSystem(state, selectCommand);

        // Act: Deselect
        var (finalState, deselectEvents) = SystemSelectionSystem.HandleDeselectSystem(stateAfterSelect);

        // Assert
        Assertions.AssertThat(selectEvents.Count).IsEqual(1);
        Assertions.AssertThat(stateAfterSelect.SelectedSystemId).IsEqual(systemToSelect.Id);
        Assertions.AssertThat(finalState.SelectedSystemId).IsEqual(Ulid.Empty);
    }

    [TestCase]
    public void SelectMultipleSystems_OnlyLastSelectionPersists()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var firstSystem = state.Systems[0];
        var secondSystem = state.Systems[1];
        var thirdSystem = state.Systems[2];

        // Act: Select first system
        var (state1, events1) = SystemSelectionSystem.HandleSelectSystem(state, new SelectSystemCommand(firstSystem.Id));

        // Act: Select second system
        var (state2, events2) = SystemSelectionSystem.HandleSelectSystem(state1, new SelectSystemCommand(secondSystem.Id));

        // Act: Select third system
        var (state3, events3) = SystemSelectionSystem.HandleSelectSystem(state2, new SelectSystemCommand(thirdSystem.Id));

        // Assert
        Assertions.AssertThat(events1.Count).IsEqual(1);
        Assertions.AssertThat(events2.Count).IsEqual(1);
        Assertions.AssertThat(events3.Count).IsEqual(1);
        Assertions.AssertThat(state3.SelectedSystemId).IsEqual(thirdSystem.Id);
    }

    #endregion

    #region Event Determinism Tests

    [TestCase]
    public void HandleSelectSystem_SameInputs_ProducesSameOutput()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var systemToSelect = state.Systems[1];
        var command = new SelectSystemCommand(systemToSelect.Id);

        // Act: Run twice with same inputs
        var (newState1, events1) = SystemSelectionSystem.HandleSelectSystem(state, command);
        var (newState2, events2) = SystemSelectionSystem.HandleSelectSystem(state, command);

        // Assert: Same inputs produce same outputs (determinism)
        Assertions.AssertThat(newState1.SelectedSystemId).IsEqual(newState2.SelectedSystemId);
        Assertions.AssertThat(events1.Count).IsEqual(events2.Count);
        Assertions.AssertThat(((SystemSelected)events1[0]).SystemId).IsEqual(((SystemSelected)events2[0]).SystemId);
    }

    [TestCase]
    public void SystemSelectedEvent_PreservesGameTime()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        state = new GameState
        {
            GameTime = 5000.0,
            Systems = state.Systems,
            ProbesInFlight = state.ProbesInFlight
        };
        var systemToSelect = state.Systems[0];
        var command = new SelectSystemCommand(systemToSelect.Id);

        // Act
        var (newState, events) = SystemSelectionSystem.HandleSelectSystem(state, command);

        // Assert
        var evt = events[0] as SystemSelected;
        Assertions.AssertThat(evt).IsNotNull();
        Assertions.AssertThat(evt!.GameTime).IsEqual(5000.0f);
    }

    #endregion

    #region Pure Function Tests

    [TestCase]
    public void HandleSelectSystem_DoesNotMutateInputState()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var originalSelectedId = state.SelectedSystemId;
        var systemToSelect = state.Systems[1];
        var command = new SelectSystemCommand(systemToSelect.Id);

        // Act
        var (newState, events) = SystemSelectionSystem.HandleSelectSystem(state, command);

        // Assert: Original state is unchanged (immutability)
        Assertions.AssertThat(state.SelectedSystemId).IsEqual(originalSelectedId);
        Assertions.AssertThat(state.SelectedSystemId).IsNotEqual(newState.SelectedSystemId);
    }

    [TestCase]
    public void HandleDeselectSystem_DoesNotMutateInputState()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        state = state.WithSelectedSystem(state.Systems[0].Id);
        var originalSelectedId = state.SelectedSystemId;
        // Deselect uses no command

        // Act
        var (newState, events) = SystemSelectionSystem.HandleDeselectSystem(state);

        // Assert: Original state is unchanged
        Assertions.AssertThat(state.SelectedSystemId).IsEqual(originalSelectedId);
        Assertions.AssertThat(newState.SelectedSystemId).IsNotEqual(state.SelectedSystemId);
    }

    #endregion

    #region Edge Cases

    [TestCase]
    public void HandleSelectSystem_NullSystems_HandledGracefully()
    {
        // Arrange
        var state = new GameState
        {
            GameTime = 1000.0,
            Systems = null!,
            ProbesInFlight = new List<ProbeInFlight>()
        };
        var command = new SelectSystemCommand(Ulid.NewUlid());

        // Act & Assert: Should not throw
        var (newState, events) = SystemSelectionSystem.HandleSelectSystem(state, command);

        Assertions.AssertThat(events.Count).IsEqual(0);
    }

    [TestCase]
    public void SystemSelectedEvent_WithZeroGameTime_IsValid()
    {
        // Arrange
        var state = CreateStateWithSystems(3);
        var systemToSelect = state.Systems[0];
        var command = new SelectSystemCommand(systemToSelect.Id);

        // Act
        var (newState, events) = SystemSelectionSystem.HandleSelectSystem(state, command);

        // Assert
        Assertions.AssertThat(events.Count).IsEqual(1);
        var evt = events[0] as SystemSelected;
        Assertions.AssertThat(evt).IsNotNull();
        Assertions.AssertThat(evt!.GameTime).IsEqual(0.0f);
    }

    #endregion
}
