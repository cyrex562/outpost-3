# XWN Rules Reference: Skills

> **Purpose:** Reference for coding agents implementing XWN skill mechanics.
> **Status:** Placeholders marked with `[PLACEHOLDER]` need to be filled from source books.

## Skill Check Mechanic

Roll 2d6 + skill level + attribute modifier vs. difficulty target.

| Difficulty | Target Number |
|---|---|
| Routine | 6 |
| Standard | 8 |
| Challenging | 10 |
| Difficult | 12 |
| Very Difficult | 14 |

- **Natural 2:** Always fails, regardless of modifiers.
- **Untrained checks:** If the character has no level in the relevant skill, they roll at skill level -1.

[PLACEHOLDER — Can all skills be attempted untrained, or are some restricted? SWN/WWN may differ here.]

## Skill Levels

| Level | Description |
|---|---|
| -1 | Untrained (default for skills not taken) |
| 0 | Basic competence |
| 1 | Trained professional |
| 2 | Veteran specialist |
| 3 | Master |
| 4 | Legendary |

[PLACEHOLDER — cost in skill points to buy each level. Is it 1 point per level? Increasing cost?]

## Skill List

Skills active in Milestone 1 are marked. Others exist in the data model but aren't mechanically used until later milestones.

| Skill | Common Attributes | Active In | Description |
|---|---|---|---|
| Administer | INT, CHA | M4+ | Manage organizations, bureaucracy, logistics |
| Connect | CHA, INT | M4+ | Social networking, finding contacts, rumors |
| Exert | STR, CON | M5+ | Physical feats of strength, endurance, forced entry |
| Fix | INT, DEX | M5+ | Repair, disable, jury-rig mechanical/electronic devices |
| Heal | INT, WIS | M3+ | Medical treatment, first aid, diagnosis |
| Know | INT, WIS | M2+ | Academic knowledge, history, science, identify objects |
| Lead | CHA, WIS | M4+ | Command, inspire, coordinate groups |
| Notice | WIS, INT | M2+ | Perception, spot hidden things, detect ambush |
| Perform | CHA, DEX | M4+ | Entertainment, distraction, artistic expression |
| Pilot | DEX, INT | Future | Operate vehicles, mounts, eventually starships |
| Program | INT | Future | Computer operation, hacking, AI interaction |
| Punch | STR, DEX | M3+ | Unarmed combat |
| Shoot | DEX | M3+ | Ranged weapon attacks |
| Sneak | DEX, INT | M5+ | Stealth, hiding, moving silently, pickpocketing |
| Stab | STR, DEX | M3+ | Melee weapon attacks |
| Survive | WIS, CON | M1 | Wilderness survival, navigation, foraging, tracking |
| Talk | CHA, WIS | M4+ | Persuasion, negotiation, deception, diplomacy |
| Trade | CHA, INT | M4+ | Buying, selling, appraising, bargaining |
| Work | Varies | M4+ | General labor, crafting, profession-specific tasks |

## Attribute Pairing for Skill Checks

The system (GM) selects which attribute pairs with a skill based on context:

- **Survive + WIS:** Navigation, reading weather signs, knowing which plants are edible.
- **Survive + CON:** Enduring harsh conditions, forced march, resisting exposure.
- **Stab + STR:** Power attacks, forcing through a guard.
- **Stab + DEX:** Precise strikes, quick attacks.
- **Notice + WIS:** General awareness, sensing danger.
- **Notice + INT:** Detailed analysis, spotting technical details in pretech ruins.

For the automated GM, default attribute pairings should be defined per skill in the skill definitions YAML. The system can override for specific situational contexts.

## Starting Skill Points

[PLACEHOLDER — Fill from source books]

**Warrior:**
- [PLACEHOLDER — N skill points. Can choose from which skills? Any non-magic skill?]
- [PLACEHOLDER — Any free skills at level 0 automatically?]

**Expert:**
- [PLACEHOLDER — N skill points. Can choose from all non-magic skills?]
- [PLACEHOLDER — Any free skills at level 0 automatically?]

**Adventurer:**
- [PLACEHOLDER — Skill points depend on partial class combination?]

## Skill Definitions YAML Schema

```yaml
# data/skills.yaml
skills:
  - id: stab
    name: Stab
    description: "Melee weapon combat skill."
    default_attribute: str
    alternate_attributes: [dex]
    can_use_untrained: true    # [PLACEHOLDER — verify]
    combat_skill: true
    available_milestone: 3

  - id: survive
    name: Survive
    description: "Wilderness survival, navigation, foraging, tracking."
    default_attribute: wis
    alternate_attributes: [con]
    can_use_untrained: true
    combat_skill: false
    available_milestone: 1
```

## Notes for the Coding Agent

- All `[PLACEHOLDER]` entries must be filled by the developer from source books.
- In Milestone 1, the only skill that matters mechanically is Survive (and possibly Notice for hex descriptions). Other skills exist in the character data model but aren't checked.
- The skill check resolver is an extension point — implement the default XWN 2d6 + mod vs. target, but structure the code so house rules can override it.
- Store all skills on the character even if they're at -1 (untrained). This makes it easy to display the full skill list and track practice ticks for the advancement house rule.
