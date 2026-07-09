# Social Interaction Rules Reference
> For use by coding agents implementing Milestone 4 social systems.
> Source: XWN core rules + UNE (Universal NPC Emulator) by Zach Best.

## XWN Social Skill Checks

Social interactions use the standard XWN 2d6 skill check mechanic:

```
Roll: 2d6 + skill_level + attribute_modifier
Target: base_difficulty (default 8)
Margin: roll_total - target
```

Attributes: STR(-2 to +2), DEX, CON, INT, WIS, CHA. Modifier scale:
- 3: -2
- 4–7: -1
- 8–13: 0
- 14–17: +1
- 18: +2

Skills relevant to social interactions:
- **Talk** (CHA): General persuasion, deception, intimidation
- **Connect** (CHA): Leveraging contacts and networks
- **Trade** (CHA): Negotiation, commerce, bribing
- **Perform** (CHA): Entertainment, distraction, impressment
- **Lead** (CHA): Commanding, inspiring, rallying

Skills default to -1 if untrained. Maximum skill level is +4.

## Skill Mapping (Default — data-driven, not hardcoded)

| Verb | Skill | Attribute | Difficulty | Opposed | Notes |
|---|---|---|---|---|---|
| convince | Talk | CHA | 8 | Yes | Target resists with WIS mod |
| intimidate | Talk | STR | 10 | Yes | Physical presence. Always costs disposition on success |
| deceive | Talk | CHA | 10 | Yes | Caught (fail 3+) = major trust damage |
| bribe | Trade | CHA | 8 | No | Requires gold/item in hand |
| connect | Connect | CHA | 8 | No | Requires plausible contact |
| ask | Talk | CHA | 6 | No | Free unless topic is sensitive |
| perform | Perform | CHA | 8 | No | Deferred — included for completeness |

**These are defaults from YAML. Per-world overrides live in SQLite `skill_mappings` table. Never hardcode.**

Opposed checks: NPC resists with their WIS modifier. Subtract NPC WIS mod from the player's total.

## Outcome Resolution

```
margin = roll_total - difficulty

margin ≤ -4:     exceptional_failure  (disposition delta: -2)
margin -3 to -1: failure              (disposition delta: -1)
margin 0 to 1:   bare_success         (disposition delta: 0)
margin 2 to 3:   solid_success        (disposition delta: +1)
margin ≥ 4:      exceptional_success  (disposition delta: +2)
```

Special cases (also stored in `disposition_outcomes` table):
- `intimidate_success`: even on success, disposition -1 (intimidation always costs goodwill)
- `deceive_caught`: fail by 3+ (margin ≤ -3), disposition -3 (NPC knows they were lied to)

## Disposition System

Disposition tracks an NPC's attitude toward the player as an integer score:

```
-3: Hostile      — will not talk, may attack
-2: Unsteady     — suspicious, guarded responses
-1: Guarded      — cautious, minimal engagement
 0: Indifferent  — neutral, transactional
+1: Sociable     — friendly, forthcoming
+2: Friendly     — warm, helpful
+3: Helpful      — actively assists, shares freely
```

**Scene entry blocked** when NPC disposition is -3 (Hostile). Attempts to talk are narrated as refusal.

**Auto-exit to Combat** when disposition drops to -3 during conversation.

Disposition is stored per NPC in their entity `data` JSON and persists between sessions.

## UNE — Universal NPC Emulator

UNE generates personality, motivation, and behavior for NPCs on first contact.

### Power Level (d7)

| d7 | Level |
|---|---|
| 1 | Wretched |
| 2 | Feeble |
| 3 | Weak |
| 4 | Average |
| 5 | Capable |
| 6 | Powerful |
| 7 | Superb |

Power level indicates the NPC's relative influence, capability, and resources compared to the player character. Affects narration tone and what the NPC can offer or threaten.

### Character Descriptor (d100)

100-entry adjective table describing the NPC's dominant personality trait (scheming, kind, brutal, mysterious, etc.). Encode verbatim from UNE rulebook into `data/tables/npc/une_descriptors.yaml`.

### Motivation (d100 verb + d100 noun)

Two separate 100-entry tables:
- **Motivation Verb**: what the NPC is actively doing (advance, acquire, avoid, create, destroy, obtain, etc.)
- **Motivation Noun**: what the verb acts on (wealth, power, fame, knowledge, justice, chaos, etc.)

Combined: "advance wealth", "destroy enemies", "obtain knowledge". This is the NPC's primary drive.

### Bearing (8 types × 5 sub-entries)

Bearing describes the NPC's current mood and behavior toward the player. Roll d8 for bearing type, then d5 for specific sub-entry.

| d8 | Bearing Type |
|---|---|
| 1 | Scheming |
| 2 | Insane |
| 3 | Friendly |
| 4 | Hostile |
| 5 | Inquisitive |
| 6 | Knowing |
| 7 | Mysterious |
| 8 | Prejudiced |

Each bearing type has 5 specific sub-entries that describe how it manifests. Plus a **Focus** for each sub-entry (what the bearing is directed at: the PC, an NPC, a topic, an object, etc.).

Bearing is modified by chaos factor:
- High chaos (7–9): bearing shifts one step hostile
- Low chaos (1–3): bearing shifts one step friendly

### Mood (7 levels)

```
1: Hated
2: Anticipating
3: Afraid
4: Neutral
5: Cautious
6: Suspicious  
7: Loved
```

The relationship disposition (loved → hated scale) modified by chaos factor determines the starting bearing type.

### NPC Generation Procedure

1. Roll power level (or infer from context: bandit = weak, merchant = average, noble = powerful)
2. Roll d100 for descriptor
3. Roll d100 twice for motivation (verb + noun)
4. Determine relationship disposition from existing faction/context, modify by chaos factor
5. Roll d8 + d5 for bearing type + sub-entry
6. Store result in entity `une_personality` JSON block

On first `talk` command: if NPC has no `une_personality`, generate one and persist it. This means personality is consistent across sessions for the same NPC.

## Social Scene Flow

```
Player: talk <npc>
  → Check: NPC in same location? NPC disposition ≥ -2?
  → If hostile (-3): narrate refusal, stay in Exploration
  → If accessible: enter Social scene

Social scene loop:
  GM: narrates NPC bearing + current activity + disposition hint
  Player: issues social command
  System: resolve skill check → disposition change → narration
  GM: narrates result
  Check: disposition -3? → transition to Combat
  Check: scene check fires? → possible interrupt/exit
  Player: leave/goodbye → return to Exploration
```

## Narration Guidelines

Narration should reflect BOTH the NPC's bearing AND the skill check outcome. Examples:

**Scheming bearing + exceptional success:**
"The merchant's eyes light up with a calculating gleam. He leans forward. 'Now that,' he says softly, 'is the kind of proposition I can work with. Here's what I can offer you...'"

**Hostile bearing + failure:**
"The guard spits at your feet. 'Smooth words won't work on me. Get moving before I change my mind about letting you walk away.'"

Vary narration text. Minimum 2 variants per bearing type × outcome combination. Store in `data/templates/social_narration.yaml`.
