"""Procedural name generator with cultural variety.

Generates first/last names, ship names, and star system names.
"""

from __future__ import annotations

import random

# ── First names — diverse cultural pool ──────────────────────────

FIRST_NAMES: list[str] = [
    # East Asian
    "Akira", "Mei", "Hiroshi", "Yuki", "Wei", "Jing", "Soo-jin", "Min-ho",
    # South Asian
    "Arun", "Priya", "Raj", "Ananya", "Vikram", "Deepa", "Sanjay", "Kavita",
    # African
    "Amara", "Kwame", "Zuri", "Kofi", "Nia", "Emeka", "Fatou", "Jelani",
    # Latin American
    "Sofia", "Carlos", "Isabella", "Diego", "Valentina", "Mateo", "Camila", "Andres",
    # European
    "Elena", "Alexei", "Ingrid", "Marcus", "Lena", "Dmitri", "Astrid", "Henrik",
    # Middle Eastern
    "Layla", "Omar", "Yasmin", "Hassan", "Nadia", "Karim", "Amira", "Tariq",
    # Mixed/Modern
    "Jordan", "Riley", "Morgan", "Sage", "Quinn", "Avery", "Kai", "Nova",
]

# ── Last names — diverse cultural pool ───────────────────────────

LAST_NAMES: list[str] = [
    # East Asian
    "Tanaka", "Chen", "Park", "Nakamura", "Liu", "Kim", "Yamamoto", "Zhang",
    # South Asian
    "Patel", "Singh", "Sharma", "Gupta", "Nair", "Kapoor", "Das", "Reddy",
    # African
    "Okafor", "Mensah", "Diallo", "Osei", "Ndongo", "Abara", "Keita", "Mwangi",
    # Latin American
    "Garcia", "Santos", "Rodriguez", "Morales", "Vasquez", "Herrera", "Reyes", "Ortiz",
    # European
    "Mueller", "Johansson", "Petrov", "O'Brien", "Kowalski", "Fischer", "Dubois", "Russo",
    # Middle Eastern
    "Al-Rashid", "Haddad", "Nazari", "Khalil", "Farouk", "Mansour", "Hashemi", "Khoury",
    # Mixed/Modern
    "Sterling", "Voss", "Crane", "Ashford", "Reeves", "Wren", "Blackwood", "Lark",
]

# ── Ship names ───────────────────────────────────────────────────

SHIP_PREFIXES: list[str] = [
    "CSV", "GS", "ARC", "ESV",  # Colony Ship Vessel, Generation Ship, Ark, Expedition Ship Vessel
]

SHIP_NAMES: list[str] = [
    "Horizon", "Endurance", "Perseverance", "Odyssey", "Venture",
    "New Dawn", "Far Reach", "Vanguard", "Resolute", "Pathfinder",
    "Covenant", "Aspiration", "Meridian", "Constellation", "Exodus",
    "Pioneer", "Harbinger", "Nomad", "Zenith", "Solace",
    "Fortitude", "Wanderer", "Aegis", "Beacon", "Catalyst",
]

# ── Star system names ────────────────────────────────────────────

STAR_PROPER_NAMES: list[str] = [
    "Kepler", "Gliese", "Trappist", "Proxima", "Barnard",
    "Tau Ceti", "Epsilon Eridani", "Lalande", "Ross", "Wolf",
    "Luyten", "Lacaille", "Kapteyn", "Groombridge", "Kruger",
    "Struve", "Van Maanen", "Teegarden", "Scholz", "Wise",
]

STAR_SUFFIXES: list[str] = [
    "Prime", "Major", "Minor", "Alpha", "Beta",
    "Gamma", "Delta", "Proxima", "Novus", "Tertia",
]


def generate_name() -> str:
    """Generate a random full name."""
    return f"{random.choice(FIRST_NAMES)} {random.choice(LAST_NAMES)}"


def generate_ship_name() -> str:
    """Generate a random ship name like 'CSV Horizon'."""
    return f"{random.choice(SHIP_PREFIXES)} {random.choice(SHIP_NAMES)}"


def generate_star_system_name() -> str:
    """Generate a star system name like 'Kepler-427' or 'Tau Ceti Prime'."""
    if random.random() < 0.5:
        # Catalog style: "Kepler-427"
        base = random.choice(STAR_PROPER_NAMES)
        num = random.randint(100, 999)
        return f"{base}-{num}"
    else:
        # Proper name style: "Tau Ceti Prime"
        base = random.choice(STAR_PROPER_NAMES)
        suffix = random.choice(STAR_SUFFIXES)
        return f"{base} {suffix}"
