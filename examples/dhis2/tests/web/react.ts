/**
 * A stand-in for React's hook dispatcher, enough to run the web's pages as React would, several at
 * once: each mounted component keeps its own state, refs and memos, effects run after each render
 * whose dependencies changed, in the order they were declared, each one's cleanup first, and an
 * external store's snapshot is read on every render. A context answers its default value, as a
 * page outside any provider reads it. Carried from the dataviz example's (`examples/dataviz/tests/web/react.ts`), itself carried from the podcast example's
 * (`examples/podcast/tests/web/react.ts`), with mounts of their own, `useSyncExternalStore`,
 * `useLayoutEffect`, `useId` and `useContext`.
 */

type Effect = { deps: readonly unknown[] | undefined; cleanup?: (() => void) | void }
type Memo = { deps: readonly unknown[] | undefined; value: unknown }
type Slot = { value: unknown } | Effect | Memo

interface Instance {
    slots: Slot[]
    dirty: boolean
    queued: { slot: number; run: () => (() => void) | void }[]
    stores: (() => void)[]
}

let current: Instance | undefined
let at = 0
let ids = 0
const mounted = new Set<{ instance: Instance; render: () => void }>()

const now = (): Instance => {
    if (current === undefined) throw new Error('a hook outside a mounted component')
    return current
}

const changed = (before: readonly unknown[] | undefined, after: readonly unknown[] | undefined) =>
    before === undefined || after === undefined || before.length !== after.length || before.some((one, index) => !Object.is(one, after[index]))

export function useState<T>(initial: T | (() => T)): [T, (next: T | ((current: T) => T)) => void] {
    const instance = now()
    const index = at
    at += 1
    if (instance.slots[index] === undefined) {
        const slot: { value: T; set?: (next: T | ((current: T) => T)) => void } = {
            value: typeof initial === 'function' ? (initial as () => T)() : initial,
        }
        slot.set = (next) => {
            const value = typeof next === 'function' ? (next as (current: T) => T)(slot.value) : next
            if (!Object.is(value, slot.value)) {
                slot.value = value
                instance.dirty = true
            }
        }
        instance.slots[index] = slot
    }
    const slot = instance.slots[index] as { value: T; set: (next: T | ((current: T) => T)) => void }
    return [slot.value, slot.set]
}

export function useRef<T>(initial: T): { current: T } {
    const instance = now()
    const index = at
    at += 1
    if (instance.slots[index] === undefined) instance.slots[index] = { value: { current: initial } }
    return (instance.slots[index] as { value: { current: T } }).value
}

export function useMemo<T>(make: () => T, deps: readonly unknown[]): T {
    const instance = now()
    const index = at
    at += 1
    const slot = instance.slots[index] as Memo | undefined
    if (slot === undefined || changed(slot.deps, deps)) instance.slots[index] = { deps, value: make() }
    return (instance.slots[index] as Memo).value as T
}

export function useCallback<T>(fn: T, deps: readonly unknown[]): T {
    return useMemo(() => fn, deps)
}

export function useEffect(run: () => (() => void) | void, deps?: readonly unknown[]): void {
    const instance = now()
    const index = at
    at += 1
    const slot = instance.slots[index] as Effect | undefined
    if (slot === undefined) {
        instance.slots[index] = { deps }
        instance.queued.push({ slot: index, run })
    } else if (changed(slot.deps, deps)) {
        slot.deps = deps
        instance.queued.push({ slot: index, run })
    }
}

export const useLayoutEffect = useEffect

export function useId(): string {
    const instance = now()
    const index = at
    at += 1
    if (instance.slots[index] === undefined) {
        ids += 1
        instance.slots[index] = { value: `:r${String(ids)}:` }
    }
    return (instance.slots[index] as { value: string }).value
}

/** A store's snapshot, read on each render; a change it tells of renders the component again. */
export function useSyncExternalStore<T>(subscribe: (listener: () => void) => () => void, get: () => T): T {
    const instance = now()
    const index = at
    at += 1
    let slot = instance.slots[index] as { value: T; stop?: () => void } | undefined
    if (slot === undefined) {
        const made: { value: T; stop?: () => void } = { value: get() }
        made.stop = subscribe(() => {
            if (!Object.is(get(), made.value)) instance.dirty = true
        })
        instance.slots[index] = made
        slot = made
    }
    slot.value = get()
    return slot.value
}

export function useContext<T>(context: { _currentValue: T }): T {
    return context._currentValue
}

/** Runs a component's hooks as one component: `render` renders it again, until no state changes are left. */
export function mount<T>(hook: () => T): { current: () => T; render: () => void } {
    const instance: Instance = { slots: [], dirty: false, queued: [], stores: [] }
    let result: T
    const render = () => {
        do {
            const before = current
            current = instance
            instance.dirty = false
            at = 0
            instance.queued = []
            try {
                result = hook()
            } finally {
                current = before
            }
            for (const effect of instance.queued) {
                const slot = instance.slots[effect.slot] as Effect
                if (typeof slot.cleanup === 'function') slot.cleanup()
                slot.cleanup = effect.run()
            }
        } while (instance.dirty)
    }
    render()
    mounted.add({ instance, render })
    return { current: () => result, render }
}

/** Renders again every mounted component whose state was set since its last render; whether any was. */
export function renderStale(): boolean {
    let any = false
    for (const one of mounted) {
        if (one.instance.dirty) {
            any = true
            one.render()
        }
    }
    return any
}

/** What React's hooks call through while a hook runs here. */
export const dispatcher = {
    useState,
    useRef,
    useMemo,
    useCallback,
    useEffect,
    useLayoutEffect,
    useInsertionEffect: useEffect,
    useId,
    useSyncExternalStore,
    useContext,
}
