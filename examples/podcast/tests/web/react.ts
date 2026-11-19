/**
 * A stand-in for React's hook dispatcher, enough to run one component's hooks as React would:
 * state, refs, memos and callbacks kept while their dependencies hold, and effects run after each
 * render whose dependencies changed, in the order they were declared, each one's cleanup first.
 * `dump.ts` sets it as React's current dispatcher, so the web's `state.tsx` runs its own provider,
 * `useCollection` and `useSleepTimer` on React's own exports. Carried from the cut example's
 * (`examples/cut/tests/web/react.ts`), with `useMemo` and callbacks that keep their dependencies.
 */

type Effect = { deps: readonly unknown[] | undefined; cleanup?: (() => void) | void }
type Memo = { deps: readonly unknown[] | undefined; value: unknown }
type Slot = { value: unknown } | Effect | Memo

let slots: Slot[] = []
let at = 0
let dirty = false
let queued: { slot: number; run: () => (() => void) | void }[] = []

const changed = (before: readonly unknown[] | undefined, after: readonly unknown[] | undefined) =>
    before === undefined || after === undefined || before.length !== after.length || before.some((one, index) => !Object.is(one, after[index]))

export function useState<T>(initial: T | (() => T)): [T, (next: T | ((current: T) => T)) => void] {
    const index = at
    at += 1
    if (slots[index] === undefined) {
        const slot: { value: T; set?: (next: T | ((current: T) => T)) => void } = {
            value: typeof initial === 'function' ? (initial as () => T)() : initial,
        }
        slot.set = (next) => {
            const value = typeof next === 'function' ? (next as (current: T) => T)(slot.value) : next
            if (!Object.is(value, slot.value)) {
                slot.value = value
                dirty = true
            }
        }
        slots[index] = slot
    }
    const slot = slots[index] as { value: T; set: (next: T | ((current: T) => T)) => void }
    return [slot.value, slot.set]
}

export function useRef<T>(initial: T): { current: T } {
    const index = at
    at += 1
    if (slots[index] === undefined) slots[index] = { value: { current: initial } }
    return (slots[index] as { value: { current: T } }).value
}

export function useMemo<T>(make: () => T, deps: readonly unknown[]): T {
    const index = at
    at += 1
    const slot = slots[index] as Memo | undefined
    if (slot === undefined || changed(slot.deps, deps)) {
        slots[index] = { deps, value: make() }
    }
    return (slots[index] as Memo).value as T
}

export function useCallback<T>(fn: T, deps: readonly unknown[]): T {
    return useMemo(() => fn, deps)
}

export function useEffect(run: () => (() => void) | void, deps?: readonly unknown[]): void {
    const index = at
    at += 1
    const slot = slots[index] as Effect | undefined
    if (slot === undefined) {
        slots[index] = { deps }
        queued.push({ slot: index, run })
    } else if (changed(slot.deps, deps)) {
        slot.deps = deps
        queued.push({ slot: index, run })
    }
}

export function useContext(): undefined {
    return undefined
}

/** Runs a component's hooks as one component: `render` renders it again, until no state changes are left. */
export function mount<T>(hook: () => T): { current: () => T; render: () => boolean } {
    slots = []
    let result: T
    const render = () => {
        let rendered = false
        do {
            dirty = false
            at = 0
            queued = []
            result = hook()
            rendered = true
            for (const effect of queued) {
                const slot = slots[effect.slot] as Effect
                if (typeof slot.cleanup === 'function') slot.cleanup()
                slot.cleanup = effect.run()
            }
        } while (dirty)
        return rendered
    }
    render()
    return { current: () => result, render }
}

/** Whether a state was set since the last render. */
export const stale = (): boolean => dirty

/** What React's hooks call through while a hook runs here. */
export const dispatcher = { useState, useRef, useMemo, useCallback, useEffect, useContext }
