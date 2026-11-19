/**
 * What the web's podcast modules give for the inputs in `inputs.json`, as JSON on stdout: run by
 * `tests/web.rs` with bun from the repository's root, the zone UTC, against `.parity/sokkel-ui` at
 * the commit `parity/podcast.json` names. The clock and the timers are held here, `performance.now`
 * reads the same clock, and `Math.random` is a seeded run while the steps play. The provider in
 * `state.tsx` runs as one component on a stand-in for React's hooks (`react.ts`), rendered again
 * after each change as React would. Its audio element is a stand-in: a source set stops what played
 * without a word and reads its metadata a moment later, `play` and `pause` say so as a browser
 * does, a time set says so, and playing on to a moment or to the end is a step of its own.
 */

const ROOT = new URL('../../../../.parity/sokkel-ui/examples/podcast/web/src/', import.meta.url).pathname
const MODULES = `${ROOT}../node_modules/`
const FAKE_REACT = new URL('react.ts', import.meta.url).pathname
const inputs = JSON.parse(await Bun.file(new URL('inputs.json', import.meta.url).pathname).text())

// The clock and the timers, moved by hand.
let now: number = inputs.now
interface Timer {
    at: number
    seq: number
    fn: () => void
    every?: number
}
let timers: Timer[] = []
let seq = 0
const setTimer = (fn: () => void, ms?: number) => {
    seq += 1
    timers.push({ at: now + (ms ?? 0), seq, fn })
    return seq
}
const setEvery = (fn: () => void, ms: number) => {
    seq += 1
    timers.push({ at: now + ms, seq, fn, every: ms })
    return seq
}
const clearTimer = (id: number | undefined) => {
    timers = timers.filter((timer) => timer.seq !== id)
}
Date.now = () => now
const kept = new Map<string, string>()

/** A stand-in for an audio element: says what a browser says, and moves only when told. */
class FakeAudio {
    preload = ''
    volume = 1
    muted = false
    playbackRate = 1
    defaultPlaybackRate = 1
    paused = true
    #src = ''
    #time = 0
    #listeners = new Map<string, { fn: () => void; once: boolean }[]>()
    get src() {
        return this.#src
    }
    /** A new source stops what played without a word, and its metadata is read a moment later. */
    set src(value: string) {
        this.#src = value
        this.paused = true
        this.#time = 0
        queueMicrotask(() => this.#emit('loadedmetadata'))
    }
    get currentTime() {
        return this.#time
    }
    set currentTime(value: number) {
        this.#time = value
        this.#emit('timeupdate')
    }
    addEventListener(name: string, fn: () => void, options?: { once?: boolean }) {
        const list = this.#listeners.get(name) ?? []
        list.push({ fn, once: options?.once === true })
        this.#listeners.set(name, list)
    }
    removeEventListener(name: string, fn: () => void) {
        this.#listeners.set(
            name,
            (this.#listeners.get(name) ?? []).filter((one) => one.fn !== fn),
        )
    }
    #emit(name: string) {
        const list = this.#listeners.get(name) ?? []
        this.#listeners.set(
            name,
            list.filter((one) => !one.once),
        )
        for (const one of list) one.fn()
    }
    play() {
        if (this.paused) {
            this.paused = false
            this.#emit('play')
        }
        return Promise.resolve()
    }
    pause() {
        if (this.paused) return
        this.paused = true
        this.#emit('pause')
    }
    /** Played on to the end: the time there, then a pause and the end, as a browser says them. */
    end(duration: number) {
        this.currentTime = duration
        if (!this.paused) {
            this.paused = true
            this.#emit('pause')
        }
        this.#emit('ended')
    }
}
let audio: FakeAudio | undefined
let urls = 0
const away = new Set<() => void>()
const document = {
    visibilityState: 'visible',
    addEventListener: (name: string, fn: () => void) => {
        if (name === 'visibilitychange') away.add(fn)
    },
    removeEventListener: (name: string, fn: () => void) => {
        if (name === 'visibilitychange') away.delete(fn)
    },
}

Object.assign(globalThis, {
    setTimeout: setTimer,
    clearTimeout: clearTimer,
    setInterval: setEvery,
    clearInterval: clearTimer,
    requestAnimationFrame: (fn: (at: number) => void) => setTimer(() => fn(now), 16),
    cancelAnimationFrame: clearTimer,
    Audio: class extends FakeAudio {
        constructor() {
            super()
            audio = this
        }
    },
    document,
    window: {
        setTimeout: setTimer,
        clearTimeout: clearTimer,
        setInterval: setEvery,
        clearInterval: clearTimer,
        addEventListener: () => {},
        removeEventListener: () => {},
    },
    localStorage: {
        getItem: (key: string) => kept.get(key) ?? null,
        setItem: (key: string, value: string) => void kept.set(key, value),
        removeItem: (key: string) => void kept.delete(key),
        key: (index: number) => [...kept.keys()][index] ?? null,
        get length() {
            return kept.size
        },
    },
})
performance.now = () => now
URL.createObjectURL = () => {
    urls += 1
    return `blob:episode-${String(urls)}`
}
URL.revokeObjectURL = () => {}

const { configureDemo } = await import(`${MODULES}tidevann-shared/backend/index.ts`)
configureDemo({ app: 'podcast' })
const { mount, dispatcher, stale } = await import(FAKE_REACT)
// React's own hooks ask the dispatcher React keeps; the stand-in answers them.
const React = await import(`${MODULES}react/index.js`)
;(React.default ?? React).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE.H = dispatcher
const catalog = await import(`${ROOT}catalog.ts`)
const backend = await import(`${ROOT}backend.ts`)
const script = await import(`${ROOT}script.ts`)
const voice = await import(`${ROOT}voice.ts`)
const { CARRIED } = await import(`${ROOT}themes.ts`)
const { PodcastProvider } = await import(`${ROOT}state.tsx`)
const { ago, spoken, megabytes } = await import(`${ROOT}parts.tsx`)
const { find, matches } = await import(`${ROOT}pages/Discover.tsx`)
const { Stats } = await import(`${ROOT}pages/Stats.tsx`)
const { RATES } = await import(`${ROOT}Player.tsx`)
const chapters = await import(`${MODULES}@winterop-com/sokkel-ui-media/dist/chapters.js`)
const { chapterStarts } = await import(`${MODULES}@winterop-com/sokkel-ui-media/dist/ChapterList.js`)
const { transcriptLineAt } = await import(`${MODULES}@winterop-com/sokkel-ui-media/dist/Transcript.js`)
const { playedOn, chapterTimerStep } = await import(`${MODULES}@winterop-com/sokkel-ui-media/dist/SleepTimer.js`)
const { toast } = await import(`${MODULES}@winterop-com/sokkel-ui/dist/components/ui/toast.js`)
let toasts: unknown[] = []
toast.add = (one: any) => {
    toasts.push({ title: one.title, description: one.description ?? null })
    return ''
}

/** FNV-1a over 32-bit words. */
function hash32(words: Uint32Array | Uint8Array): number {
    let hash = 0x811c9dc5
    for (let index = 0; index < words.length; index += 1) {
        hash ^= words[index]
        hash = Math.imul(hash, 0x01000193) >>> 0
    }
    return hash >>> 0
}

/** A script's syllables in brief: how many, a hash of their numbers' bits and one of their words. */
function syllableDigest(syllables: any[]) {
    const numbers = new Float64Array(syllables.length * 5)
    syllables.forEach((one, index) => numbers.set([one.at, one.length, one.pitch, one.pitchEnd, one.gain], index * 5))
    const words = syllables.map((one) => `${one.vowel}${one.onset} ${one.last ? 1 : 0}${one.voice}`).join(',')
    return {
        count: syllables.length,
        numbers: hash32(new Uint32Array(numbers.buffer)),
        words: hash32(new TextEncoder().encode(words)),
    }
}

// --- The catalog, the backend's seeds and the parts' words --------------------------------------------
const library = {
    hosts: catalog.HOSTS,
    categories: catalog.CATEGORIES,
    shows: catalog.SHOWS,
    episodes: catalog.EPISODES,
    episodesOf: catalog.SHOWS.map((show: any) => catalog.episodesOf(show.id).map((one: any) => one.id)),
    categoryLabels: [...catalog.CATEGORIES.map((one: any) => one.id), 'elsewhere'].map(catalog.categoryLabel),
    hostsOf: catalog.SHOWS.map(catalog.hostsOf),
    themes: CARRIED,
    rates: RATES,
}
const seeds = {
    prefs: backend.DEFAULT_PREFS,
    days: inputs.dayOffsets.map((offset: number) => backend.day(offset)),
    daysFrom: inputs.dayFroms.map(([offset, from]: [number, number]) => backend.day(offset, from)),
    sizes: [...catalog.EPISODES.map((one: any) => one.id), 'nowhere-1'].map(backend.sizeOf),
}
const words = {
    ago: inputs.ago.map((days: number) => ago(now - days, now)),
    spoken: inputs.spoken.map(spoken),
    megabytes: inputs.megabytes.map(megabytes),
}

// --- Scripts and their sound ------------------------------------------------------------------------------
const scripts = {
    random: inputs.randomSeeds.map((seed: string) => {
        const next = script.seeded(seed)
        return Array.from({ length: 6 }, () => next())
    }),
    voices: script.VOICES,
    episodes: catalog.EPISODES.map((episode: any) => {
        const made = script.scriptOf(episode.id)
        return {
            id: episode.id,
            duration: made.duration,
            length: script.lengthOf(episode.id),
            chapters: made.chapters,
            lines: made.lines,
            cues: made.cues,
            syllables: syllableDigest(made.syllables),
        }
    }),
    syllables: inputs.fullSyllables.map((id: string) => script.scriptOf(id).syllables.slice(0, 60)),
    spoken: inputs.spokenScripts.map(([text, speaker, seed]: [string, string, string]) => {
        const made = script.spokenScript(text, speaker, seed)
        return { duration: made.duration, lines: made.lines, syllables: made.syllables }
    }),
}
function sound(made: any) {
    const samples: Float32Array = voice.render(made)
    return samples
}
const renders = await Promise.all(
    inputs.renders.map(async ([kind, ...rest]: [string, ...string[]]) => {
        const made = kind === 'episode' ? script.scriptOf(rest[0]) : script.spokenScript(rest[0], rest[1], rest[2])
        const samples = sound(made)
        const wav = new Uint8Array(await voice.wavOf(samples).arrayBuffer())
        const at = [0, 1000, 40000, 100000, samples.length - 1]
        return {
            length: samples.length,
            hash: hash32(new Uint32Array(samples.buffer)),
            samples: at.map((index) => samples[index] ?? null),
            levels: voice.levelsOf(samples),
            coarse: voice.levelsOf(samples, 7),
            wavHead: Array.from(wav.slice(0, 44)),
            wavHash: hash32(wav),
        }
    }),
)

// --- The media add-on's pure parts the app leans on ------------------------------------------------------
const media = inputs.chapterCases.map(([id, positions]: [string, number[]]) => {
    const made = script.scriptOf(id)
    const starts = chapterStarts(made.chapters)
    return {
        starts,
        index: positions.map((at: number) => chapters.chapterIndex(starts, at)),
        previous: positions.map((at: number) => chapters.previousChapterStart(starts, at)),
        next: positions.map((at: number) => chapters.nextChapterStart(starts, at) ?? null),
        spans: starts.map((_: number, index: number) => chapters.chapterSpan(starts, index, made.duration)),
        line: positions.map((at: number) => transcriptLineAt(made.lines, at)),
        seek: positions.map((at: number) => chapters.seekPoint(at, made.duration) ?? null),
    }
})
const timerSteps = inputs.timerSteps.map(([anchor, from, to, seconds, playing]: [number, number | null, number, number, boolean]) =>
    chapterTimerStep([0, 10, 20, 30], anchor, { from: from ?? undefined, to, seconds, playing }),
)
const played = inputs.playedOn.map(([from, to, seconds]: [number, number, number]) => playedOn(from, to, seconds))

// --- The provider, step by step -------------------------------------------------------------------------
const startedRandom = Math.random
Math.random = script.seeded(inputs.random)
const view = mount(() => PodcastProvider({ children: null }))
const state = () => (view.current() as any).props.value

/** Lets promises settle and renders again while a state is set, as React would. */
async function flush() {
    for (let round = 0; round < 100; round += 1) {
        for (let i = 0; i < 20; i += 1) await Promise.resolve()
        if (!stale()) return
        view.render()
    }
    throw new Error('the provider never settled')
}

/** Fires every timer due by `to`, each at its time, in the order they were set, rendering after each. */
async function advance(to: number) {
    for (;;) {
        await flush()
        timers.sort((a, b) => a.at - b.at || a.seq - b.seq)
        const next = timers[0]
        if (next === undefined || next.at > to) break
        now = next.at
        if (next.every === undefined) timers.shift()
        else next.at += next.every
        next.fn()
    }
    now = to
    await flush()
}

// The Stats page, run on the provider's state, read from what it draws.
;(dispatcher as any).useContext = () => state()
function stats() {
    const found: any = { tiles: [], bars: [], nothing: false }
    const walk = (node: any) => {
        if (node === null || node === undefined || typeof node !== 'object') return
        if (Array.isArray(node)) {
            node.forEach(walk)
            return
        }
        const props = node.props ?? {}
        if (props.variant === 'tile') found.tiles.push({ label: props.label, value: props.value, note: props.note })
        if (Array.isArray(props.items) && typeof props.format === 'function')
            found.bars.push({
                label: props.label,
                items: props.items,
                written: props.items.map((item: any) => props.format(item.value)),
            })
        if (props.title === 'Nothing listened yet') found.nothing = true
        walk(props.children)
    }
    walk(Stats())
    return found
}

/** A collection's rows as the backend keeps them, or its seed while nothing is written yet. */
const rows = (name: string, seed: () => unknown[]) => JSON.parse(kept.get(`tidevann-demo:podcast:${name}`) ?? 'null') ?? seed()
const answers: unknown[] = []
function look(): Record<string, unknown> {
    const now_ = state()
    const out = {
        shows: rows('shows', () => [...now_.shows.values()]),
        episodes: rows('episodes', () => [...now_.episodes.values()]),
        player: rows('player', () => [now_.player]),
        prefs: rows('prefs', () => [now_.prefs]),
        listens: rows('listens', () => now_.listens),
        current: now_.current,
        playing: now_.playing,
        position: now_.position,
        media: now_.media !== null,
        waveform: now_.waveform === undefined ? null : now_.waveform.length,
        sleep: { timer: now_.sleep.timer, remaining: now_.sleep.remaining ?? null },
        element: audio === undefined ? null : { paused: audio.paused, time: audio.currentTime, rate: audio.playbackRate, volume: audio.volume, muted: audio.muted },
        stats: stats(),
        toasts,
        answers: [...answers],
    }
    toasts = []
    answers.length = 0
    return out
}

/** Each step's look holds only what changed since the step before; the first holds everything. */
let last: Record<string, string> = {}
function changed(seen: Record<string, unknown>): Record<string, unknown> {
    const out: Record<string, unknown> = {}
    const written: Record<string, string> = {}
    for (const [key, value] of Object.entries(seen)) {
        written[key] = JSON.stringify(value)
        if (written[key] !== last[key]) out[key] = value
    }
    last = written
    return out
}

await advance(inputs.now + 900)
const found = inputs.queries.map((query: string) => {
    const result = find(query, (id: string) => state().episodes.get(id)?.out === true)
    return { shows: result.shows.map((one: any) => one.id), episodes: result.episodes.map((one: any) => one.id) }
})
const matched = inputs.matches.map(([query, texts]: [string, string[]]) => matches(query, texts))
const steps: unknown[] = [changed(look())]
for (const step of inputs.steps) {
    await advance(inputs.now + step.at)
    const on = state()
    switch (step.op) {
        case 'play':
            on.play(step.id, step.from ?? undefined)
            break
        case 'setPlaying':
            on.setPlaying(step.on)
            break
        case 'seek':
            on.seek(step.to)
            break
        case 'skip':
            on.skip(step.by)
            break
        case 'rate':
            on.setRate(step.rate)
            break
        case 'volume':
            on.setVolume(step.volume)
            break
        case 'muted':
            on.setMuted(step.muted)
            break
        case 'playNext':
            answers.push(on.playNext())
            break
        case 'queueAdd':
            on.queueAdd(step.id, step.where)
            break
        case 'queueRemove':
            on.queueRemove(step.id)
            break
        case 'queueMove':
            on.queueMove(step.from, step.to)
            break
        case 'queueClear':
            on.queueClear()
            break
        case 'markPlayed':
            on.markPlayed(step.id, step.played)
            break
        case 'star':
            on.star(step.id, step.starred)
            break
        case 'archive':
            on.archive(step.id)
            break
        case 'archiveAll':
            on.archiveAll()
            break
        case 'download':
            on.download(step.id)
            break
        case 'removeDownload':
            on.removeDownload(step.id)
            break
        case 'removeAllDownloads':
            on.removeAllDownloads()
            break
        case 'checkForNew':
            void on.checkForNew().then((count: number) => answers.push(count))
            break
        case 'subscribe':
            on.subscribe(step.id)
            break
        case 'unsubscribe':
            on.unsubscribe(step.id)
            break
        case 'prefs':
            on.setPrefs(step.patch)
            break
        case 'clearStats':
            void on.clearStats()
            break
        case 'sleep':
            on.sleep.set(step.timer)
            break
        case 'played':
            if (audio !== undefined) audio.currentTime = step.to
            break
        case 'ended':
            audio?.end(script.lengthOf(on.current))
            break
        case 'away':
            document.visibilityState = 'hidden'
            for (const fn of away) fn()
            document.visibilityState = 'visible'
            break
        default:
            throw new Error(`no step ${String(step.op)}`)
    }
    await advance(inputs.now + step.at + (step.wait ?? 900))
    steps.push(changed(look()))
}
Math.random = startedRandom

process.stdout.write(
    JSON.stringify({
        library,
        seeds,
        words,
        scripts,
        renders,
        media,
        timerSteps,
        played,
        found,
        matched,
        steps,
    }),
)
