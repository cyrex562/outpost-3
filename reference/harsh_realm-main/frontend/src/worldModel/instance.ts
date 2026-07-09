/**
 * Singleton WorldModel instance (HR-795, 104c).
 *
 * Importing this module:
 *  1. creates the single `worldModel` shared by the entire app, and
 *  2. side-effect-imports `reducers.ts` so the reducer registry is populated
 *     before the first `apply()` call.
 *
 * The projection (`projection.ts`) and the WS handler
 * (`_websocketHandlers.ts`) both import from here.
 */
import { createWorldModel } from "./model";
import "./reducers"; // side-effect: populate reducerRegistry

export const worldModel = createWorldModel();
