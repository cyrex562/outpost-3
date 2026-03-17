"""Tests for the SurveySystem."""

import random

from outpost3.simulation import GameTime
from outpost3.simulation.ecs import World
from outpost3.simulation.systems import Phase
from outpost3.simulation.components import (
    FactionPhase, Resources, Population, ShipSystems, Notable,
    StarSystem, Planet, TransitState, SearchProgress,
    SurveyProgress, SurveyDetail,
)
from outpost3.simulation.survey import (
    SurveySystem, _generate_survey_detail, _compute_acceptance_threshold,
)


def _make_survey_world(
    num_planets: int = 3,
    hab_scores: list[float] | None = None,
) -> tuple[World, int, int]:
    """Create a world in SURVEY phase with a target system and planets."""
    world = World()
    faction_id = world.create_entity()
    world.add_component(faction_id, FactionPhase(phase=Phase.SURVEY))
    world.add_component(faction_id, Resources(
        fuel=2000, food=1500, water=1000, spare_parts=300, materials=400,
    ))
    world.add_component(faction_id, Population(
        total=900, morale=0.70,
    ))
    world.add_component(faction_id, ShipSystems())

    # Create target star system
    sys_eid = world.create_entity()
    world.add_component(sys_eid, StarSystem(
        name="Kepler-427", distance_ly=12.0,
        habitability_score=0.6, num_planets=num_planets, selected=True,
    ))

    # Create transit state pointing at the system
    world.add_component(faction_id, TransitState(
        destination_id=sys_eid, distance_ly=12.0,
        distance_traveled_ly=12.0, eta_days=5000,
    ))

    # Create planets
    if hab_scores is None:
        hab_scores = [0.3, 0.6, 0.2]
    for i in range(num_planets):
        p_eid = world.create_entity()
        hab = hab_scores[i] if i < len(hab_scores) else 0.2
        world.add_component(p_eid, Planet(
            name=f"Kepler-427-{chr(97 + i)}",
            parent_system_id=sys_eid,
            planet_type="rocky",
            size=1.0,
            atmosphere=0.5 if hab > 0.4 else 0.1,
            water=0.5 if hab > 0.4 else 0.1,
            minerals=0.5,
            habitability=hab,
        ))

    return world, faction_id, sys_eid


class TestSurveyDetailGeneration:
    def test_generates_all_fields(self):
        planet = Planet(
            planet_type="rocky", size=1.0,
            atmosphere=0.7, water=0.6, minerals=0.5, habitability=0.65,
        )
        detail = _generate_survey_detail(planet)
        assert detail.atmosphere_composition != ""
        assert detail.water_quality != ""
        assert detail.mineral_deposits != ""
        assert detail.hazards != ""
        assert detail.seasonal_pattern != ""
        assert 0.0 <= detail.refined_habitability <= 1.0

    def test_high_water_planet_gets_good_water_quality(self):
        planet = Planet(planet_type="rocky", water=0.8, habitability=0.7)
        # Run many times to check distribution
        good_count = 0
        for _ in range(50):
            detail = _generate_survey_detail(planet)
            if "fresh" in detail.water_quality or "clean" in detail.water_quality or "abundant" in detail.water_quality:
                good_count += 1
        assert good_count > 0

    def test_low_hab_planet_gets_more_hazards(self):
        planet = Planet(planet_type="rocky", habitability=0.1)
        hazard_count = 0
        for _ in range(50):
            detail = _generate_survey_detail(planet)
            if detail.hazards != "none detected":
                hazard_count += 1
        assert hazard_count > 10  # should usually have hazards


class TestAcceptanceThreshold:
    def test_base_threshold(self):
        threshold = _compute_acceptance_threshold(0, 3, None, None, [])
        assert 0.35 <= threshold <= 0.55

    def test_decreases_with_rejections(self):
        t0 = _compute_acceptance_threshold(0, 3, None, None, [])
        t1 = _compute_acceptance_threshold(1, 3, None, None, [])
        t2 = _compute_acceptance_threshold(2, 3, None, None, [])
        assert t1 < t0
        assert t2 < t1

    def test_force_accept_at_max_rejections(self):
        threshold = _compute_acceptance_threshold(3, 3, None, None, [])
        assert threshold == 0.0

    def test_low_resources_lower_threshold(self):
        resources = Resources(fuel=100, food=100)
        t_low = _compute_acceptance_threshold(0, 3, resources, None, [])
        t_normal = _compute_acceptance_threshold(0, 3, None, None, [])
        assert t_low < t_normal

    def test_cautious_notables_raise_threshold(self):
        notables = [
            Notable(name="Test", role="Captain", traits=["cautious", "conservative"], alive=True),
        ]
        t_cautious = _compute_acceptance_threshold(0, 3, None, None, notables)
        t_normal = _compute_acceptance_threshold(0, 3, None, None, [])
        assert t_cautious > t_normal

    def test_bold_notables_lower_threshold(self):
        notables = [
            Notable(name="Test", role="Captain", traits=["bold", "optimistic"], alive=True),
        ]
        t_bold = _compute_acceptance_threshold(0, 3, None, None, notables)
        t_normal = _compute_acceptance_threshold(0, 3, None, None, [])
        assert t_bold < t_normal


class TestSurveySystem:
    def test_starts_survey(self):
        world, faction_id, sys_eid = _make_survey_world()
        system = SurveySystem()

        events = system.tick(world, GameTime(100))
        assert len(events) == 1
        assert events[0].event_type == "survey.begin"
        assert events[0].data["num_planets"] == 3

        progress = world.get_component(faction_id, SurveyProgress)
        assert progress is not None
        assert progress.total_planets == 3
        assert progress.phase == "in_transit"

    def test_transit_to_surveying(self):
        world, faction_id, _ = _make_survey_world()
        system = SurveySystem()

        # Start survey
        system.tick(world, GameTime(100))
        progress = world.get_component(faction_id, SurveyProgress)

        # Advance past transit
        events = system.tick(world, GameTime(progress.in_transit_until + 1))
        assert progress.phase == "surveying"
        survey_begins = [e for e in events if e.event_type == "survey.planet_survey_begin"]
        assert len(survey_begins) == 1

    def test_completes_planet_survey(self):
        world, faction_id, _ = _make_survey_world()
        system = SurveySystem()

        # Start and advance to surveying
        system.tick(world, GameTime(100))
        progress = world.get_component(faction_id, SurveyProgress)
        system.tick(world, GameTime(progress.in_transit_until + 1))

        # Advance past survey duration
        done_day = progress.current_planet_survey_start + progress.current_planet_survey_days + 1
        events = system.tick(world, GameTime(done_day))

        reports = [e for e in events if e.event_type == "survey.planet_report"]
        assert len(reports) == 1
        assert progress.planets_surveyed == 1
        assert "atmosphere" in reports[0].data
        assert "refined_hab" in reports[0].data

    def test_accepts_good_system(self):
        """System with high-hab planet should be accepted."""
        world, faction_id, _ = _make_survey_world(
            num_planets=2, hab_scores=[0.8, 0.2],
        )
        system = SurveySystem()

        # Run through entire survey
        events_all = []
        for day in range(100, 600):
            events = system.tick(world, GameTime(day))
            events_all.extend(events)

        accepted = [e for e in events_all if e.event_type == "survey.system_accepted"]
        assert len(accepted) == 1

        fp = world.get_component(faction_id, FactionPhase)
        assert fp.phase == Phase.FOUNDING

    def test_rejects_poor_system(self):
        """System with only low-hab planets should be rejected."""
        world, faction_id, _ = _make_survey_world(
            num_planets=2, hab_scores=[0.1, 0.05],
        )
        system = SurveySystem()

        events_all = []
        for day in range(100, 600):
            events = system.tick(world, GameTime(day))
            events_all.extend(events)

        rejected = [e for e in events_all if e.event_type == "survey.system_rejected"]
        assert len(rejected) == 1

        fp = world.get_component(faction_id, FactionPhase)
        assert fp.phase == Phase.SEARCH

    def test_rejection_increments_counter(self):
        world, faction_id, _ = _make_survey_world(
            num_planets=1, hab_scores=[0.05],
        )
        system = SurveySystem()

        for day in range(100, 600):
            system.tick(world, GameTime(day))

        progress = world.get_component(faction_id, SurveyProgress)
        assert progress.rejections == 1

    def test_rejection_resets_to_search(self):
        world, faction_id, _ = _make_survey_world(
            num_planets=1, hab_scores=[0.05],
        )
        system = SurveySystem()

        for day in range(100, 600):
            system.tick(world, GameTime(day))

        fp = world.get_component(faction_id, FactionPhase)
        assert fp.phase == Phase.SEARCH

        # SearchProgress should exist for new round
        search = world.get_component(faction_id, SearchProgress)
        assert search is not None
        assert search.systems_discovered == 0

    def test_force_accept_after_max_rejections(self):
        """After max rejections, even poor planets should be accepted."""
        world, faction_id, _ = _make_survey_world(
            num_planets=1, hab_scores=[0.15],
        )
        # Pre-set rejections to max
        system = SurveySystem()

        # Start survey and set high rejection count
        system.tick(world, GameTime(100))
        progress = world.get_component(faction_id, SurveyProgress)
        progress.rejections = 3  # at max

        # Run through survey
        events_all = []
        for day in range(101, 600):
            events = system.tick(world, GameTime(day))
            events_all.extend(events)

        # Should accept because threshold is 0.0
        accepted = [e for e in events_all if e.event_type == "survey.system_accepted"]
        assert len(accepted) == 1

    def test_survey_auto_pauses_on_begin(self):
        world, faction_id, _ = _make_survey_world()
        system = SurveySystem()

        events = system.tick(world, GameTime(100))
        assert events[0].auto_pause is True

    def test_survey_tracks_best_planet(self):
        world, faction_id, _ = _make_survey_world(
            num_planets=3, hab_scores=[0.2, 0.7, 0.3],
        )
        system = SurveySystem()

        # Run through all planets
        for day in range(100, 800):
            system.tick(world, GameTime(day))

        progress = world.get_component(faction_id, SurveyProgress)
        # Best planet should be tracked (though refined hab may differ)
        assert progress.best_planet_name != ""

    def test_active_phases(self):
        system = SurveySystem()
        assert system.should_run(Phase.SURVEY)
        assert not system.should_run(Phase.TRANSIT)
        assert not system.should_run(Phase.SEARCH)


class TestSearchSystemRejectFilter:
    def test_search_skips_rejected_systems(self):
        """SearchSystem should not select rejected systems."""
        from outpost3.simulation.generation.search import SearchSystem

        world = World()
        faction_id = world.create_entity()
        world.add_component(faction_id, FactionPhase(phase=Phase.SEARCH))
        world.add_component(faction_id, Resources(fuel=2000))
        world.add_component(faction_id, SearchProgress(
            systems_discovered=3, target_discoveries=3,
        ))

        # Create one rejected and one good system
        rej_eid = world.create_entity()
        world.add_component(rej_eid, StarSystem(
            name="Rejected", distance_ly=10, habitability_score=0.9,
            rejected=True, surveyed=True,
        ))

        good_eid = world.create_entity()
        world.add_component(good_eid, StarSystem(
            name="GoodSystem", distance_ly=15, habitability_score=0.5,
        ))

        system = SearchSystem()
        events = system.tick(world, GameTime(100))

        dest_events = [e for e in events if e.event_type == "search.destination_selected"]
        assert len(dest_events) == 1
        assert dest_events[0].data["system_name"] == "GoodSystem"


class TestEcsRemoveComponent:
    def test_remove_component(self):
        world = World()
        eid = world.create_entity()
        world.add_component(eid, Resources(fuel=100))
        assert world.get_component(eid, Resources) is not None

        world.remove_component(eid, Resources)
        assert world.get_component(eid, Resources) is None

    def test_remove_nonexistent_component(self):
        world = World()
        eid = world.create_entity()
        # Should not raise
        world.remove_component(eid, Resources)
