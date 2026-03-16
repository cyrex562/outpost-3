"""Tests for procedural name generation."""

from outpost3.simulation.generation.names import (
    generate_name, generate_ship_name, generate_star_system_name,
    FIRST_NAMES, LAST_NAMES, SHIP_NAMES, STAR_PROPER_NAMES,
)


class TestNameGeneration:
    def test_generate_name_format(self):
        name = generate_name()
        parts = name.split(" ")
        assert len(parts) >= 2  # first + last (some last names have hyphens)

    def test_generate_name_variety(self):
        names = {generate_name() for _ in range(50)}
        assert len(names) > 20  # should produce variety

    def test_name_pools_have_cultural_variety(self):
        assert len(FIRST_NAMES) >= 40
        assert len(LAST_NAMES) >= 40


class TestShipNameGeneration:
    def test_ship_name_format(self):
        name = generate_ship_name()
        parts = name.split(" ", 1)
        assert len(parts) == 2
        prefix, ship = parts
        assert prefix in ("CSV", "GS", "ARC", "ESV")

    def test_ship_name_variety(self):
        names = {generate_ship_name() for _ in range(30)}
        assert len(names) > 10


class TestStarSystemNameGeneration:
    def test_star_system_name_nonempty(self):
        name = generate_star_system_name()
        assert len(name) > 3

    def test_star_system_name_variety(self):
        names = {generate_star_system_name() for _ in range(30)}
        assert len(names) > 10
