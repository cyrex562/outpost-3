namespace OutpostGame.Tests;

using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Core.World;

/// <summary>Phase 5: colony status — destruction, critical, abandonment, thriving, recovery.</summary>
[TestFixture]
public class Phase5Tests
{
    private static ColonySession NewSession() =>
        new(new SiteDefinition(42, BiomeType.Barren, new GridSize(64, 64)));

    /// <summary>Drops all survival resources so population starts dying immediately.</summary>
    private static ColonySession StarvedSession()
    {
        var s = NewSession();
        // Remove the survival buffer so needs can't be met.
        s.State.Resources.TryConsume("nutrients", s.State.Resources.Get("nutrients"));
        s.State.Resources.TryConsume("water",     s.State.Resources.Get("water"));
        s.State.Resources.TryConsume("oxygen",    s.State.Resources.Get("oxygen"));
        return s;
    }

    // ── Destruction ──────────────────────────────────────────────────────────

    [Test]
    public void Status_Destroyed_WhenPopulationReachesZero()
    {
        var session = NewSession();
        session.State.Population.Count  = 1;
        session.State.Population.Health = 0f;  // Forces immediate death

        // No survival resources — one turn kills the last colonist.
        session.State.Resources.TryConsume("nutrients", session.State.Resources.Get("nutrients"));
        session.State.Resources.TryConsume("water",     session.State.Resources.Get("water"));
        session.State.Resources.TryConsume("oxygen",    session.State.Resources.Get("oxygen"));

        session.EndTurn(1);

        Assert.That(session.Status, Is.EqualTo(ColonyStatus.Destroyed));
    }

    [Test]
    public void Status_Destroyed_IsTerminal_NoFurtherChanges()
    {
        var session = NewSession();
        session.State.Population.Count  = 1;
        session.State.Population.Health = 0f;
        session.State.Resources.TryConsume("nutrients", session.State.Resources.Get("nutrients"));
        session.State.Resources.TryConsume("water",     session.State.Resources.Get("water"));
        session.State.Resources.TryConsume("oxygen",    session.State.Resources.Get("oxygen"));
        session.EndTurn(1);

        Assert.That(session.Status, Is.EqualTo(ColonyStatus.Destroyed));

        // Artificially restore population to test that terminal state is frozen.
        session.State.Population.Count = 20;
        session.EndTurn(50);

        Assert.That(session.Status, Is.EqualTo(ColonyStatus.Destroyed),
            "Terminal state must not change once set.");
    }

    [Test]
    public void ColonyEnded_Event_Fires_OnDestruction()
    {
        var session = NewSession();
        session.State.Population.Count  = 1;
        session.State.Population.Health = 0f;
        session.State.Resources.TryConsume("nutrients", session.State.Resources.Get("nutrients"));
        session.State.Resources.TryConsume("water",     session.State.Resources.Get("water"));
        session.State.Resources.TryConsume("oxygen",    session.State.Resources.Get("oxygen"));

        ColonyStatus? captured = null;
        session.ColonyStatusChanged += s => captured = s;

        session.EndTurn(1);

        Assert.That(captured, Is.EqualTo(ColonyStatus.Destroyed));
    }

    // ── Critical ─────────────────────────────────────────────────────────────

    [Test]
    public void Status_Critical_AfterFiveLowMoraleTurns()
    {
        var session = NewSession();
        session.State.Population.Morale = 10f;  // Below threshold
        // Prevent actual deaths so we isolate morale logic.
        session.State.Population.Health = 100f;
        foreach (var r in new[] { "nutrients", "water", "oxygen" })
        {
            session.State.Resources.SetCap(r, 100_000f);
            session.State.Resources.Add(r, 99_000f);
        }

        // Force LowMoraleTurns counter directly — faster than waiting for morale to drop.
        session.State.LowMoraleTurns = 4;  // One more turn will tip it to Critical.
        session.State.Population.Morale = 10f;

        session.EndTurn(1);

        Assert.That(session.Status, Is.EqualTo(ColonyStatus.Critical));
    }

    [Test]
    public void Status_RecoversToCritical_WhenMoraleRises()
    {
        var session = NewSession();
        // Manually put session into Critical state.
        session.State.Status         = ColonyStatus.Critical;
        session.State.LowMoraleTurns = 6;
        session.State.Population.Morale = 80f;  // Morale recovered
        foreach (var r in new[] { "nutrients", "water", "oxygen" })
        {
            session.State.Resources.SetCap(r, 100_000f);
            session.State.Resources.Add(r, 99_000f);
        }

        session.EndTurn(1);

        Assert.That(session.Status, Is.EqualTo(ColonyStatus.Active),
            "Status must revert to Active when morale rises above critical threshold.");
        Assert.That(session.State.LowMoraleTurns, Is.EqualTo(0));
    }

    // ── Abandonment ──────────────────────────────────────────────────────────

    [Test]
    public void Status_Abandoned_AfterFifteenLowMoraleTurns()
    {
        var session = NewSession();
        session.State.LowMoraleTurns    = 14;
        session.State.Status            = ColonyStatus.Critical;
        session.State.Population.Morale = 10f;
        session.State.Population.Health = 100f;
        foreach (var r in new[] { "nutrients", "water", "oxygen" })
        {
            session.State.Resources.SetCap(r, 100_000f);
            session.State.Resources.Add(r, 99_000f);
        }

        session.EndTurn(1);

        Assert.That(session.Status, Is.EqualTo(ColonyStatus.Abandoned));
    }

    [Test]
    public void Status_Abandoned_IsTerminal()
    {
        var session = NewSession();
        session.State.Status            = ColonyStatus.Abandoned;
        session.State.LowMoraleTurns    = 15;
        session.State.Population.Morale = 95f;  // Even high morale can't undo abandonment.
        foreach (var r in new[] { "nutrients", "water", "oxygen" })
        {
            session.State.Resources.SetCap(r, 100_000f);
            session.State.Resources.Add(r, 99_000f);
        }

        session.EndTurn(5);

        Assert.That(session.Status, Is.EqualTo(ColonyStatus.Abandoned));
    }

    // ── Thriving ─────────────────────────────────────────────────────────────

    [Test]
    public void Status_Thriving_AfterThirtyTurnsOfHighConditions()
    {
        var session = NewSession();
        session.State.Population.Count  = 60;
        session.State.Population.Morale = 90f;
        session.State.Population.Health = 90f;
        session.State.Labor.TotalWorkers = 40;
        foreach (var r in new[] { "nutrients", "water", "oxygen" })
        {
            session.State.Resources.SetCap(r, 100_000f);
            session.State.Resources.Add(r, 99_000f);
        }

        // ThrivingTurns must reach 30 — seed it at 29 so one turn tips it.
        session.State.ThrivingTurns = 29;

        session.EndTurn(1);

        Assert.That(session.Status, Is.EqualTo(ColonyStatus.Thriving));
    }

    [Test]
    public void Status_ThrivingConditions_ResetWhenUnmet()
    {
        var session = NewSession();
        session.State.ThrivingTurns = 20;
        session.State.Population.Count  = 10;  // Below 50 — conditions not met.
        session.State.Population.Morale = 90f;
        session.State.Population.Health = 90f;
        foreach (var r in new[] { "nutrients", "water", "oxygen" })
        {
            session.State.Resources.SetCap(r, 100_000f);
            session.State.Resources.Add(r, 99_000f);
        }

        session.EndTurn(1);

        Assert.That(session.State.ThrivingTurns, Is.EqualTo(0));
        Assert.That(session.Status, Is.EqualTo(ColonyStatus.Active));
    }

    // ── EventLog entries ──────────────────────────────────────────────────────

    [Test]
    public void EventLog_ContainsCriticalEntry_OnDestruction()
    {
        var session = NewSession();
        session.State.Population.Count  = 1;
        session.State.Population.Health = 0f;
        session.State.Resources.TryConsume("nutrients", session.State.Resources.Get("nutrients"));
        session.State.Resources.TryConsume("water",     session.State.Resources.Get("water"));
        session.State.Resources.TryConsume("oxygen",    session.State.Resources.Get("oxygen"));

        session.EndTurn(1);

        bool hasEntry = session.State.EventLog.All.Any(
            e => e.Severity == ColonyEventSeverity.Critical
              && e.Message.Contains("perished"));
        Assert.That(hasEntry, Is.True);
    }
}
