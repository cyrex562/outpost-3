"""Tests for ECS-Lite World container."""

from dataclasses import dataclass

from outpost3.simulation.ecs import World


@dataclass
class Position:
    x: float
    y: float


@dataclass
class Velocity:
    dx: float
    dy: float


@dataclass
class Health:
    hp: int


class TestWorldEntityLifecycle:
    def test_create_entity(self):
        world = World()
        e1 = world.create_entity()
        e2 = world.create_entity()
        assert e1 == 0
        assert e2 == 1

    def test_entity_exists(self):
        world = World()
        e = world.create_entity()
        assert world.entity_exists(e)

    def test_remove_entity(self):
        world = World()
        e = world.create_entity()
        world.add_component(e, Position(1, 2))
        world.remove_entity(e)
        assert not world.entity_exists(e)
        assert world.get_component(e, Position) is None

    def test_remove_nonexistent_entity(self):
        world = World()
        world.remove_entity(999)  # should not raise


class TestWorldComponents:
    def test_add_and_get_component(self):
        world = World()
        e = world.create_entity()
        world.add_component(e, Position(10, 20))
        pos = world.get_component(e, Position)
        assert pos is not None
        assert pos.x == 10
        assert pos.y == 20

    def test_get_missing_component(self):
        world = World()
        e = world.create_entity()
        assert world.get_component(e, Position) is None

    def test_get_components_multiple(self):
        world = World()
        e = world.create_entity()
        world.add_component(e, Position(1, 2))
        world.add_component(e, Velocity(3, 4))
        result = world.get_components(e, Position, Velocity)
        assert result is not None
        pos, vel = result
        assert pos.x == 1
        assert vel.dx == 3

    def test_get_components_missing_one(self):
        world = World()
        e = world.create_entity()
        world.add_component(e, Position(1, 2))
        assert world.get_components(e, Position, Velocity) is None

    def test_has_component(self):
        world = World()
        e = world.create_entity()
        world.add_component(e, Position(0, 0))
        assert world.has_component(e, Position)
        assert not world.has_component(e, Velocity)

    def test_add_component_to_nonexistent_entity(self):
        world = World()
        try:
            world.add_component(999, Position(0, 0))
            assert False, "Should have raised KeyError"
        except KeyError:
            pass

    def test_overwrite_component(self):
        world = World()
        e = world.create_entity()
        world.add_component(e, Position(1, 2))
        world.add_component(e, Position(10, 20))
        pos = world.get_component(e, Position)
        assert pos.x == 10


class TestWorldQueries:
    def test_entities_with_single_type(self):
        world = World()
        e1 = world.create_entity()
        e2 = world.create_entity()
        e3 = world.create_entity()
        world.add_component(e1, Position(0, 0))
        world.add_component(e2, Position(1, 1))
        world.add_component(e3, Velocity(1, 1))
        result = world.entities_with(Position)
        assert sorted(result) == [e1, e2]

    def test_entities_with_multiple_types(self):
        world = World()
        e1 = world.create_entity()
        e2 = world.create_entity()
        world.add_component(e1, Position(0, 0))
        world.add_component(e1, Velocity(1, 1))
        world.add_component(e2, Position(2, 2))
        result = world.entities_with(Position, Velocity)
        assert result == [e1]

    def test_entities_with_no_match(self):
        world = World()
        world.create_entity()
        assert world.entities_with(Position) == []

    def test_entities_with_no_types(self):
        world = World()
        e1 = world.create_entity()
        e2 = world.create_entity()
        result = world.entities_with()
        assert sorted(result) == [e1, e2]

    def test_entities_with_respects_removal(self):
        world = World()
        e1 = world.create_entity()
        e2 = world.create_entity()
        world.add_component(e1, Position(0, 0))
        world.add_component(e2, Position(1, 1))
        world.remove_entity(e1)
        result = world.entities_with(Position)
        assert result == [e2]

    def test_all_components_of_type(self):
        world = World()
        e1 = world.create_entity()
        e2 = world.create_entity()
        world.add_component(e1, Health(100))
        world.add_component(e2, Health(50))
        result = world.all_components_of_type(Health)
        assert len(result) == 2
        assert result[e1].hp == 100
        assert result[e2].hp == 50


class TestWorldSerialization:
    def test_to_dict_basic(self):
        world = World()
        e = world.create_entity()
        world.add_component(e, Position(1, 2))
        d = world.to_dict()
        assert d["next_id"] == 1
        assert 0 in d["alive"]
        assert len(d["components"]) == 1

    def test_roundtrip(self):
        world = World()
        e1 = world.create_entity()
        e2 = world.create_entity()
        world.add_component(e1, Position(10, 20))
        world.add_component(e2, Position(30, 40))
        world.add_component(e1, Health(100))

        d = world.to_dict()

        # Build type registry from the module paths
        registry = {}
        for type_key in d["components"]:
            if "Position" in type_key:
                registry[type_key] = Position
            elif "Health" in type_key:
                registry[type_key] = Health

        world2 = World.from_dict(d, type_registry=registry)
        assert world2.entity_exists(0)
        assert world2.entity_exists(1)
        pos = world2.get_component(0, Position)
        assert pos is not None
        assert pos.x == 10
        assert pos.y == 20
        hp = world2.get_component(0, Health)
        assert hp is not None
        assert hp.hp == 100

    def test_repr(self):
        world = World()
        world.create_entity()
        s = repr(world)
        assert "entities=1" in s
