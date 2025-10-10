using System.Collections.Generic;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;
using System;

namespace Outpost3.Core.Systems;

public static class TimeSystem
{
    public static (GameState, List<IGameEvent>) Reduce(GameState state, Commands.ICommand command)
    {
        return command switch
        {
            AdvanceTime cmd => ReduceAdvanceTime(state, cmd),
            LaunchProbe cmd => ReduceLaunchProbe(state, cmd),
            _ => (state, new List<IGameEvent>())
        };
    }

    private static (GameState, List<IGameEvent>) ReduceAdvanceTime(
        GameState state,
        AdvanceTime command)
    {
        var newTime = state.GameTime + command.Dt;
        var events = new List<IGameEvent>
        {
            new TimeAdvanced(newTime, command.Dt)
        };

        var newState = state.WithAdvanceTime(command.Dt);

        var arrivedProbeIds = new List<Ulid>();
        foreach (var probe in state.ProbesInFlight)
        {
            if (probe.ArrivalTime <= newTime)
            {
                arrivedProbeIds.Add(probe.Id);
                events.Add(new ProbeArrived(newTime, probe.Id, probe.TargetSystemId));

                // generate discovered system
                var system = GenerateSystem(probe.TargetSystemId);
                newState = newState.WithSystemDiscovered(system);
                events.Add(new SystemDiscovered(newTime, system.Id, system.Name));
            }
        }

        // remove arrived probes
        if (arrivedProbeIds.Count > 0)
        {
            newState = newState.WithProbesRemoved(arrivedProbeIds);
        }

        return (newState, events);
    }

    private static (GameState, List<IGameEvent>) ReduceLaunchProbe(
        GameState state,
        LaunchProbe command
    )
    {
        var travelTime = 100.0;
        var arrivalTime = state.GameTime + travelTime;

        var newState = state.WithProbeLaunched(
            command.TargetSystemId,
            arrivalTime,
            out var probeId
        );

        var events = new List<IGameEvent>
        {
            new ProbeLaunched(
                state.GameTime,
                probeId,
                command.TargetSystemId,
                arrivalTime
            )
        };

        return (newState, events);
    }

    private static StarSystem GenerateSystem(Ulid id)
    {
        Random rand = new Random();

        var spectralClasses = new[] { "O", "B", "A", "F", "G", "K", "M" };
        var spectralClass = spectralClasses[rand.Next(spectralClasses.Length)];
        var bodyTypes = new[] { "Planet", "Asteroid Belt" };

        var numBodies = rand.Next(1, 9);

        var bodies = new List<CelestialBody>();

        bodies.Add(new CelestialBody
        {
            Id = Ulid.NewUlid(),
            Name = "Star",
            BodyType = "Star",
            Explored = true
        });
        for (int i = 1; i < numBodies; i++)
        {
            bodies.Add(new CelestialBody
            {
                Id = Ulid.NewUlid(),
                Name = $"Body {i + 1}",
                BodyType = bodyTypes[rand.Next(bodyTypes.Length)],
                Explored = false
            });
        }

        return new StarSystem
        {
            Id = id,
            Name = $"System {id.ToString().Substring(0, 6)}",
            SpectralClass = spectralClass,
            Bodies = bodies
        };
    }
}