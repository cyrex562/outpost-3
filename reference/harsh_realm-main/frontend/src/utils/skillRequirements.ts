// Pure skill-requirement evaluation, mirroring the backend
// `harsh_realm.gm.scenes.character_build` logic so the form gates skills the
// same way the server validates them. Kept dependency-free for easy testing.

import type { SkillOption, SkillRequirement } from "../types/api";

export interface SkillDraftContext {
  characterClass: string;
  attributes: Record<string, number>;
  currentMilestone: number;
  traits: string[];
}

export interface SkillLockState {
  takeable: boolean;
  reasons: string[];
}

/** Return an unmet-reason string for one requirement, or null if satisfied. */
export function evaluateRequirement(
  req: SkillRequirement,
  ctx: SkillDraftContext,
): string | null {
  switch (req.type) {
    case "class":
      if (req.classes.length && !req.classes.includes(ctx.characterClass)) {
        return req.message ?? `Requires class: ${req.classes.join(", ")}.`;
      }
      return null;
    case "attribute":
      if (req.attribute != null && req.min != null) {
        if ((ctx.attributes[req.attribute] ?? 0) < req.min) {
          return req.message ?? `Requires ${req.attribute.toUpperCase()} ${req.min}+.`;
        }
      }
      return null;
    case "trait":
      if (req.trait_id != null && !ctx.traits.includes(req.trait_id)) {
        return req.message ?? `Requires: ${req.trait_id.replace(/_/g, " ")}.`;
      }
      return null;
    default:
      // Unknown type from a content module the frontend doesn't model: lock it
      // conservatively, surfacing the authored message.
      return req.message ?? `Requires: ${req.type}.`;
  }
}

/** Compute every reason a skill is currently un-takeable (empty = takeable). */
export function evaluateSkillLock(
  skill: SkillOption,
  ctx: SkillDraftContext,
): SkillLockState {
  const reasons: string[] = [];
  if (skill.available_milestone > ctx.currentMilestone) {
    reasons.push("Not yet available — unlocks at a later milestone.");
  }
  for (const req of skill.requirements) {
    const reason = evaluateRequirement(req, ctx);
    if (reason != null) reasons.push(reason);
  }
  return { takeable: reasons.length === 0, reasons };
}
