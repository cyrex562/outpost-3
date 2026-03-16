"""Tests for the world snapshot builder."""

from outpost3.simulation import GameTime
from outpost3.simulation.ecs import World
from outpost3.simulation.systems import Phase
from outpost3.simulation.components import (
    FactionPhase, Resources, Population, ShipSystems,
    Notable, Loadout, StarSystem, TransitState, SearchProgress,
)


def _build_snapshot_from_world(world: World, day: int = 100) -> dict:
    """Build a snapshot dict like main.build_snapshot but without engine dependency."""
    gt = GameTime(day)

    snapshot = {
        "type": "snapshot",
        "day_offset": day,
        "year": gt.year,
        "month": gt.month,
        "phase": None,
        "resources": None,
        "population": None,
        "ship_systems": None,
        "notables": [],
        "star_systems": [],
        "transit": None,
        "search": None,
        "ship_name": None,
    }

    faction_entities = world.entities_with(FactionPhase)
    if faction_entities:
        fid = faction_entities[0]
        fp = world.get_component(fid, FactionPhase)
        if fp:
            snapshot["phase"] = fp.phase.value

        res = world.get_component(fid, Resources)
        if res:
            snapshot["resources"] = res.to_summary()

        pop = world.get_component(fid, Population)
        if pop:
            snapshot["population"] = {
                "total": pop.total,
                "scientists": pop.scientists,
                "engineers": pop.engineers,
                "medical": pop.medical,
                "military": pop.military,
                "civilians": pop.civilians,
                "morale": round(pop.morale, 2),
            }

        ss = world.get_component(fid, ShipSystems)
        if ss:
            snapshot["ship_systems"] = ss.to_summary()

        transit = world.get_component(fid, TransitState)
        if transit:
            dest_system = world.get_component(transit.destination_id, StarSystem)
            snapshot["transit"] = {
                "destination": dest_system.name if dest_system else "Unknown",
                "distance_ly": transit.distance_ly,
                "distance_traveled_ly": round(transit.distance_traveled_ly, 2),
                "transit_phase": transit.transit_phase,
                "eta_days": transit.eta_days,
                "departure_day": transit.departure_day,
                "pct_complete": round(
                    transit.distance_traveled_ly / max(transit.distance_ly, 0.01) * 100, 1
                ),
            }

        search = world.get_component(fid, SearchProgress)
        if search:
            snapshot["search"] = {
                "systems_discovered": search.systems_discovered,
                "target_discoveries": search.target_discoveries,
            }

        loadout = world.get_component(fid, Loadout)
        if loadout:
            snapshot["ship_name"] = loadout.ship_name

    for eid, notable in world.all_components_of_type(Notable).items():
        snapshot["notables"].append({
            "name": notable.name,
            "role": notable.role,
            "age": int(notable.age),
            "traits": notable.traits,
            "alive": notable.alive,
        })

    for eid, star in world.all_components_of_type(StarSystem).items():
        snapshot["star_systems"].append({
            "name": star.name,
            "distance_ly": star.distance_ly,
            "habitability_score": star.habitability_score,
            "num_planets": star.num_planets,
            "selected": star.selected,
        })

    return snapshot


class TestSnapshotBuilder:
    def test_basic_snapshot(self):
        world = World()
        fid = world.create_entity()
        world.add_component(fid, FactionPhase(phase=Phase.TRANSIT))
        world.add_component(fid, Resources(fuel=1000, food=500))
        world.add_component(fid, Population(total=800, morale=0.65))
        world.add_component(fid, ShipSystems())

        snap = _build_snapshot_from_world(world)
        assert snap["type"] == "snapshot"
        assert snap["phase"] == "transit"
        assert snap["resources"]["fuel"] == 1000.0
        assert snap["population"]["total"] == 800
        assert snap["population"]["morale"] == 0.65
        assert snap["ship_systems"]["hull_integrity"] == 1.0

    def test_snapshot_with_transit(self):
        world = World()
        fid = world.create_entity()
        world.add_component(fid, FactionPhase(phase=Phase.TRANSIT))
        world.add_component(fid, Resources())

        dest_eid = world.create_entity()
        world.add_component(dest_eid, StarSystem(name="Alpha", distance_ly=15.0))

        world.add_component(fid, TransitState(
            destination_id=dest_eid,
            distance_ly=15.0,
            distance_traveled_ly=7.5,
            eta_days=5000,
        ))

        snap = _build_snapshot_from_world(world)
        assert snap["transit"]["destination"] == "Alpha"
        assert snap["transit"]["pct_complete"] == 50.0

    def test_snapshot_with_notables(self):
        world = World()
        fid = world.create_entity()
        world.add_component(fid, FactionPhase(phase=Phase.TRANSIT))

        n_eid = world.create_entity()
        world.add_component(n_eid, Notable(
            name="Test Captain", role="Captain", age=45,
            traits=["bold", "pragmatic"], alive=True,
        ))

        snap = _build_snapshot_from_world(world)
        assert len(snap["notables"]) == 1
        assert snap["notables"][0]["name"] == "Test Captain"
        assert snap["notables"][0]["alive"] is True

    def test_snapshot_with_star_systems(self):
        world = World()
        fid = world.create_entity()
        world.add_component(fid, FactionPhase(phase=Phase.SEARCH))

        for i in range(3):
            s_eid = world.create_entity()
            world.add_component(s_eid, StarSystem(
                name=f"System-{i}",
                distance_ly=10 + i * 5,
                habitability_score=0.3 + i * 0.2,
                selected=(i == 2),
            ))

        snap = _build_snapshot_from_world(world)
        assert len(snap["star_systems"]) == 3
        selected = [s for s in snap["star_systems"] if s["selected"]]
        assert len(selected) == 1

    def test_snapshot_with_search_progress(self):
        world = World()
        fid = world.create_entity()
        world.add_component(fid, FactionPhase(phase=Phase.SEARCH))
        world.add_component(fid, SearchProgress(
            systems_discovered=2,
            target_discoveries=4,
        ))

        snap = _build_snapshot_from_world(world)
        assert snap["search"]["systems_discovered"] == 2
        assert snap["search"]["target_discoveries"] == 4

    def test_snapshot_with_ship_name(self):
        world = World()
        fid = world.create_entity()
        world.add_component(fid, FactionPhase(phase=Phase.TRANSIT))
        world.add_component(fid, Loadout(ship_name="Endeavour"))

        snap = _build_snapshot_from_world(world)
        assert snap["ship_name"] == "Endeavour"

    def test_empty_world_snapshot(self):
        world = World()
        snap = _build_snapshot_from_world(world)
        assert snap["phase"] is None
        assert snap["resources"] is None
        assert snap["notables"] == []
        assert snap["star_systems"] == []
