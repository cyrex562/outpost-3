"""Tests for component dataclasses."""

from outpost3.simulation.components import (
    FactionPhase, Resources, Population, ShipSystems,
    Notable, Loadout, ROLES, ALL_TRAITS, TRAIT_AXES,
)
from outpost3.simulation.systems import Phase


class TestFactionPhase:
    def test_default_phase(self):
        fp = FactionPhase()
        assert fp.phase == Phase.LOADOUT

    def test_set_phase(self):
        fp = FactionPhase(phase=Phase.TRANSIT)
        assert fp.phase == Phase.TRANSIT


class TestResources:
    def test_defaults_zero(self):
        r = Resources()
        assert r.food == 0.0
        assert r.fuel == 0.0

    def test_to_summary(self):
        r = Resources(materials=100.123, fuel=200.456)
        s = r.to_summary()
        assert s["materials"] == 100.1
        assert s["fuel"] == 200.5
        assert "food" in s
        assert "water" in s
        assert "spare_parts" in s


class TestPopulation:
    def test_defaults(self):
        p = Population()
        assert p.total == 0
        assert p.morale == 0.75

    def test_custom_values(self):
        p = Population(total=1000, scientists=200, morale=0.5)
        assert p.total == 1000
        assert p.scientists == 200


class TestShipSystems:
    def test_defaults_full_health(self):
        ss = ShipSystems()
        assert ss.hull_integrity == 1.0
        assert ss.engines == 1.0

    def test_to_summary(self):
        ss = ShipSystems(hull_integrity=0.8567)
        s = ss.to_summary()
        assert s["hull_integrity"] == 0.857


class TestNotable:
    def test_creation(self):
        n = Notable(name="Test Person", role="Captain", age=40, traits=["bold", "optimistic"])
        assert n.name == "Test Person"
        assert n.alive is True
        assert len(n.traits) == 2


class TestLoadout:
    def test_immutable_record(self):
        lo = Loadout(crew_size=1000, ship_name="CSV Test")
        assert lo.crew_size == 1000
        assert lo.ship_name == "CSV Test"


class TestTraits:
    def test_all_traits_from_axes(self):
        assert len(ALL_TRAITS) == 8
        for a, b in TRAIT_AXES:
            assert a in ALL_TRAITS
            assert b in ALL_TRAITS

    def test_roles_defined(self):
        assert len(ROLES) == 8
        assert "Captain" in ROLES
