/**
 * Bridge to the native Rust core exposed by the Tauri shell.
 *
 * When the app runs inside Tauri, these helpers invoke `#[tauri::command]`
 * functions backed by the `harsh-core` Rust crate. When it does not (a normal
 * browser, or the PyWebView build), `available` is `false` and callers should
 * fall back to the HTTP backend. This is the seam that lets engine logic migrate
 * from Python to Rust without the UI needing to know which is in play.
 */

import type { DiceResult } from "../types/api"

/** True when running inside the Tauri desktop shell. */
export function isTauri(): boolean {
  return typeof window !== "undefined" && window.__TAURI__ !== undefined
}

function invokeNative<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const tauri = window.__TAURI__
  if (tauri === undefined) {
    return Promise.reject(
      new Error(`Native command "${cmd}" is unavailable outside the Tauri shell.`),
    )
  }
  return args === undefined ? tauri.core.invoke<T>(cmd) : tauri.core.invoke<T>(cmd, args)
}

export interface NativeCore {
  /** Whether native commands can be invoked in the current runtime. */
  readonly available: boolean
  /** Roll `numDice` d`sides` plus `modifier` natively. */
  rollDice(numDice: number, sides: number, modifier?: number): Promise<DiceResult>
  /** Roll a 6-attribute set via 4d6-drop-lowest natively. */
  rollAttributeSet(): Promise<number[]>
  /** Return the XWN attribute modifier for a score natively. */
  attributeModifier(score: number): Promise<number>
}

export function useNativeCore(): NativeCore {
  return {
    available: isTauri(),
    rollDice(numDice: number, sides: number, modifier = 0): Promise<DiceResult> {
      return invokeNative<DiceResult>("roll_dice", { numDice, sides, modifier })
    },
    rollAttributeSet(): Promise<number[]> {
      return invokeNative<number[]>("roll_attribute_set")
    },
    attributeModifier(score: number): Promise<number> {
      return invokeNative<number>("attribute_modifier", { score })
    },
  }
}
