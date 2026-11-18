/**
 * A stand-in for React's hook dispatcher, enough to run one hook as a component would: state, refs,
 * callbacks and effects, the effects run after each render whose dependencies changed. `dump.ts`
 * sets it as React's current dispatcher, so the web's `history.ts` and `editor/use-playback.ts` run
 * their own code on React's own exports.
 */

type Slot = { value: unknown } | { deps: readonly unknown[] | undefined; cleanup?: (() => void) | void }

let slots: Slot[] = []
let at = 0
let dirty = false
let queued: { slot: number; run: () => (() => void) | void }[] = []

const changed = (before: readonly unknown[] | undefined, after: readonly unknown[] | undefined) =>
    before === undefined || after === undefined || before.length !== after.length || before.some((one, index) => !Object.is(one, after[index]))

export function useState<T>(initial: T | (() => T)): [T, (next: T | ((current: T) => T)) => void] {
    const index = at
    at += 1
    if (slots[index] === undefined) slots[index] = { value: typeof initial === 'function' ? (initial as () => T)() : initial }
    const slot = slots[index] as { value: T }
    const set = (next: T | ((current: T) => T)) => {
        const value = typeof next === 'function' ? (next as (current: T) => T)(slot.value) : next
        if (!Object.is(value, slot.value)) {
            slot.value = value
            dirty = true
        }
    }
    return [slot.value, set]
}

export function useRef<T>(initial: T): { current: T } {
    const index = at
    at += 1
    if (slots[index] === undefined) slots[index] = { value: { current: initial } }
    return (slots[index] as { value: { current: T } }).value
}

export function useCallback<T>(fn: T, _deps: readonly unknown[]): T {
    at += 1
    return fn
}

export function useEffect(run: () => (() => void) | void, deps?: readonly unknown[]): void {
    const index = at
    at += 1
    const slot = slots[index] as { deps: readonly unknown[] | undefined; cleanup?: (() => void) | void } | undefined
    if (slot === undefined) {
        slots[index] = { deps }
        queued.push({ slot: index, run })
    } else if (changed(slot.deps, deps)) {
        slot.deps = deps
        queued.push({ slot: index, run })
    }
}

/** Runs a hook as one component: `render` renders it again, until no state changes are left. */
export function mount<T>(hook: () => T): { current: () => T; render: () => void } {
    slots = []
    let result: T
    const render = () => {
        do {
            dirty = false
            at = 0
            queued = []
            result = hook()
            for (const effect of queued) {
                const slot = slots[effect.slot] as { cleanup?: (() => void) | void }
                if (typeof slot.cleanup === 'function') slot.cleanup()
                slot.cleanup = effect.run()
            }
        } while (dirty)
    }
    render()
    return { current: () => result, render }
}

/** What React's hooks call through while a hook runs here. */
export const dispatcher = { useState, useRef, useCallback, useEffect }
