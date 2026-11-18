/**
 * What the web's cut modules give for the inputs in `inputs.json`, as JSON on stdout: run by
 * `tests/web.rs` with bun from the repository's root, the zone UTC, against `.parity/sokkel-ui` at
 * the commit `parity/cut.json` names. The clock, the frame clock and the timers are held here; the
 * hooks (`history.ts`, `editor/use-playback.ts`) run on React's own exports answered by the stand-in
 * dispatcher in `react.ts`; Web Audio is a stand-in that writes down what is scheduled, and the
 * toasts and routes gone to are written down too. There is no video encoder, so the export paints
 * and counts every frame and makes no file, on a canvas that draws nothing.
 */

const ROOT = new URL('../../../../.parity/sokkel-ui/examples/cut/web/src/', import.meta.url).pathname
const FAKE_REACT = new URL('react.ts', import.meta.url).pathname
const inputs = JSON.parse(await Bun.file(new URL('inputs.json', import.meta.url).pathname).text())

// Toasts and the routes gone to are written down.
const toasts: unknown[] = []
const went: string[] = []
let hash = ''
const location = {
    get hash() {
        return hash
    },
    set hash(next: string) {
        hash = next
        went.push(next)
    },
}

// The clock, the frame clock and the timers, moved by hand.
let now: number = inputs.now
let perf = 0
interface Timer {
    at: number
    seq: number
    fn: () => void
}
let timers: Timer[] = []
let seq = 0
const setTimer = (fn: () => void, ms?: number) => {
    seq += 1
    timers.push({ at: now + (ms ?? 0), seq, fn })
    return seq
}
const clearTimer = (id: number | undefined) => {
    timers = timers.filter((timer) => timer.seq !== id)
}
let frames: { id: number; fn: (at: number) => void }[] = []
Date.now = () => now
performance.now = () => perf
const kept = new Map<string, string>()

// Web Audio: every buffer started is written down with where it was asked to play.
const sounds: unknown[] = []
class FakeAudio {
    destination = {}
    get currentTime() {
        return perf / 1000
    }
    createGain() {
        return { gain: { value: 1 }, connect() {}, disconnect() {} }
    }
    createBuffer() {
        return { copyToChannel() {} }
    }
    createBufferSource() {
        const node: any = {
            buffer: undefined,
            gain: undefined,
            connect(to: any) {
                node.gain = to
            },
            disconnect() {},
            stop() {},
            start: (when: number, offset: number, duration: number) => {
                sounds[sounds.length - 1]!.starts.push({ delay: when - perf / 1000, offset, duration, gain: node.gain.gain.value })
            },
        }
        return node
    }
    resume() {
        sounds.push({ at: perf, starts: [] as unknown[] })
        return Promise.resolve()
    }
}

Object.assign(globalThis, {
    setTimeout: setTimer,
    clearTimeout: clearTimer,
    requestAnimationFrame: (fn: (at: number) => void) => {
        seq += 1
        frames.push({ id: seq, fn })
        return seq
    },
    cancelAnimationFrame: (id: number) => {
        frames = frames.filter((frame) => frame.id !== id)
    },
    AudioContext: FakeAudio,
    window: { setTimeout: setTimer, clearTimeout: clearTimer, addEventListener: () => {}, location },
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

async function settle() {
    for (let i = 0; i < 20; i += 1) await Promise.resolve()
}

/** Fires every timer due by `to`, each at its time, in the order they were set. */
async function advance(to: number) {
    for (;;) {
        await settle()
        timers.sort((a, b) => a.at - b.at || a.seq - b.seq)
        const next = timers[0]
        if (next === undefined || next.at > to) break
        timers.shift()
        now = next.at
        next.fn()
    }
    now = to
    await settle()
}

const { configureDemo } = await import(`${ROOT}../node_modules/tidevann-shared/backend/index.ts`)
configureDemo({ app: 'cut' })
const model = await import(`${ROOT}model.ts`)
const { seedProjects } = await import(`${ROOT}seed.ts`)
const media = await import(`${ROOT}media.ts`)
const { mixdown } = await import(`${ROOT}sound.ts`)
const render = await import(`${ROOT}render.ts`)
const { useEditHistory } = await import(`${ROOT}history.ts`)
const { usePlayback } = await import(`${ROOT}editor/use-playback.ts`)
const { mount, dispatcher } = await import(FAKE_REACT)
// React's own hooks ask the dispatcher React keeps; the stand-in answers them.
const React = await import(`${ROOT}../node_modules/react/index.js`)
;(React.default ?? React).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE.H = dispatcher
const { synthesize } = await import(`${ROOT}../node_modules/tidevann-shared/clips/sound.ts`)
const { seeded } = await import(`${ROOT}../node_modules/tidevann-shared/clips/scenes.ts`)
const { fileName } = await import(`${ROOT}../node_modules/@winterop-com/sokkel-ui/dist/lib/download.js`)
const { parseTime } = await import(`${ROOT}editor/ClipInspector.tsx`)
const { mediaDrag } = await import(`${ROOT}editor/Bin.tsx`)
const { toast } = await import(`${ROOT}../node_modules/@winterop-com/sokkel-ui/dist/components/ui/toast.js`)
toast.add = (one: unknown) => {
    toasts.push(one)
    return ''
}

const sourceOf = (id: string) => media.SOURCES.find((one: any) => one.id === id)

// --- The seed, the bin and the model's words ------------------------------------------------------
const projects = seedProjects(inputs.now)
const words = {
    timecodes: inputs.times.map(model.timecode),
    lengths: inputs.times.map(model.lengthText),
    onFrame: inputs.times.map(model.onFrame),
    parsed: inputs.typed.map(parseTime),
    fileNames: inputs.titles.map((title: string) => fileName(title, 'webm', 'edit')),
    names: projects.flatMap((project: any) => project.clips.map(model.nameOf)),
    lengthOf: projects.map(model.lengthOf),
    placed: media.SOURCES.map(model.placedLength),
    drags: media.SOURCES.map(mediaDrag),
    specs: media.SOURCES.map(media.specOf),
    random: [1, 11, 348, 4294967295].map((seed) => {
        const next = seeded(seed)
        return Array.from({ length: 6 }, () => next())
    }),
    clipAt: projects.map((project: any) =>
        project.tracks.map((track: any) => inputs.moments.map((t: number) => model.clipAt(project, track.id, t)?.id ?? null)),
    ),
    pictures: projects.map((project: any) => inputs.moments.map((t: number) => render.pictureAt(project, t)?.id ?? null)),
    titles: projects.map((project: any) =>
        inputs.moments.map((t: number) => render.titlesAt(project, t).map((clip: any) => clip.id)),
    ),
    freeStart: inputs.freeStarts.map(([track, at, duration, ignore]: [string, number, number, string?]) =>
        model.freeStart(projects[0], track, at, duration, ignore),
    ),
}

// --- Edits through undo and redo --------------------------------------------------------------------
function runEdits(project: any, steps: any[]) {
    now = inputs.now
    const view = mount(() => useEditHistory({ tracks: project.tracks, clips: project.clips }))
    const out: unknown[] = []
    for (const step of steps) {
        now = inputs.now + step.at
        const history = view.current()
        let made: string | null = null
        const take = (done: { edit: any; id: string } | undefined) => {
            if (done === undefined) return undefined
            made = done.id
            return done.edit
        }
        switch (step.op) {
            case 'patch':
                history.apply((edit: any) => model.patchClip(edit, step.id, step.patch), `${step.id}:${step.field}`)
                break
            case 'split':
                history.apply((edit: any) => model.splitAll(edit, step.t, step.only))
                break
            case 'splitClip':
                history.apply((edit: any) => take(model.splitClip(edit, step.id, step.t)), `split:${String(step.t)}`)
                break
            case 'delete':
                history.apply((edit: any) => model.deleteClips(edit, step.ids, step.ripple))
                break
            case 'changes':
                history.apply((edit: any) => model.applyChanges(edit, step.changes))
                break
            case 'addSource': {
                const next = take(model.addSource(history.edit, sourceOf(step.source), step.t))
                if (next !== undefined) history.apply(next)
                break
            }
            case 'dropSource': {
                const next = take(model.dropSource(history.edit, sourceOf(step.source), step.track, step.t))
                if (next !== undefined) history.apply(next)
                break
            }
            case 'addTitle': {
                const next = take(model.addTitle(history.edit, step.t, step.text, step.duration))
                if (next !== undefined) history.apply(next)
                break
            }
            case 'track':
                history.apply((edit: any) => ({
                    ...edit,
                    tracks: edit.tracks.map((track: any) => (track.id === step.id ? { ...track, ...step.change } : track)),
                }))
                break
            case 'undo':
                history.undo()
                break
            case 'redo':
                history.redo()
                break
            case 'copy':
                break
        }
        view.render()
        const after = view.current()
        out.push({
            edit: step.op === 'copy' ? model.copyEdit(after.edit) : after.edit,
            canUndo: after.canUndo,
            canRedo: after.canRedo,
            made,
            length: model.lengthOf(after.edit),
        })
    }
    return out
}
const edits = inputs.edits.map((run: any) =>
    runEdits(
        projects.find((one: any) => one.id === run.project),
        run.steps,
    ),
)

// --- Playback ---------------------------------------------------------------------------------------
function runPlayback(project: any, steps: any[]) {
    perf = 0
    frames = []
    sounds.length = 0
    let loop = false
    const view = mount(() => {
        const history = useEditHistory({ tracks: project.tracks, clips: project.clips })
        const playback = usePlayback(history.edit, model.lengthOf(history.edit), loop)
        return { history, playback }
    })
    const out: unknown[] = []
    for (const step of steps) {
        perf = step.at
        const { history, playback } = view.current()
        switch (step.op) {
            case 'frame': {
                const due = frames
                frames = []
                for (const frame of due) frame.fn(perf)
                break
            }
            case 'toggle':
                playback.toggle()
                break
            case 'pause':
                playback.pause()
                break
            case 'forward':
                playback.forward()
                break
            case 'backward':
                playback.backward()
                break
            case 'step':
                playback.step(step.frames)
                break
            case 'seek':
                playback.seek(step.t)
                break
            case 'loop':
                loop = step.loop
                break
            case 'delete':
                history.apply((edit: any) => model.deleteClips(edit, step.ids, false))
                break
            case 'mute':
                history.apply((edit: any) => ({
                    ...edit,
                    tracks: edit.tracks.map((track: any) => (track.id === step.track ? { ...track, muted: step.muted } : track)),
                }))
                break
        }
        view.render()
        const after = view.current().playback
        out.push({ playhead: after.playhead, rate: after.rate, playing: after.playing, waiting: frames.length })
    }
    return { looks: out, sounds: structuredClone(sounds) }
}
const playback = inputs.playback.map((run: any) =>
    runPlayback(
        projects.find((one: any) => one.id === run.project),
        run.steps,
    ),
)

// --- Sound ------------------------------------------------------------------------------------------
const sound = media.SOURCES.map((source: any) => {
    const [left, right] = synthesize(media.specOf(source))
    const every = (list: Float32Array) => Array.from({ length: Math.ceil(list.length / inputs.stride) }, (_, i) => list[i * inputs.stride])
    return {
        id: source.id,
        frames: left.length,
        left: every(left),
        right: every(right),
        peaks: source.kind === 'audio' ? media.peaksOf(source) : null,
    }
})
const clipPeaks = projects.flatMap((project: any) =>
    project.clips
        .filter((clip: any) => sourceOf(clip.source)?.kind === 'audio')
        .map((clip: any) => media.clipPeaks(sourceOf(clip.source), clip.inPoint, clip.duration)),
)
const mixes = projects.map((project: any) => {
    const [left, right] = mixdown(project, model.lengthOf(project))
    const every = (list: Float32Array) => Array.from({ length: Math.ceil(list.length / inputs.stride) }, (_, i) => list[i * inputs.stride])
    return { frames: left.length, left: every(left), right: every(right) }
})

// --- Export: no video encoder here, so every frame is drawn and counted --------------------------
const { exportEdit } = await import(`${ROOT}export.ts`)
const nothing: any = new Proxy(function () {}, {
    get: (_target, key) => (key === 'width' ? 0 : nothing),
    apply: () => nothing,
    set: () => true,
})
Object.assign(globalThis, {
    OffscreenCanvas: class {
        getContext() {
            return nothing
        }
    },
})
const exports: unknown[] = []
for (const project of projects) {
    for (const size of ['720', '480', '360']) {
        const calls: [number, number][] = []
        let result: any
        exportEdit(project, size, (share: number, count: number) => calls.push([share, count]), new AbortController().signal).then(
            (done: any) => (result = done),
        )
        while (result === undefined) await advance(now + 1)
        exports.push({ project: project.id, size, calls, frames: result.frames, file: result.file ?? null })
    }
}

// --- The backend, as the projects page and the editor change it ----------------------------------
const { projects: store, prefs: prefsStore } = await import(`${ROOT}collections.ts`)
const actions = await import(`${ROOT}actions.ts`)
now = inputs.now
const looks: unknown[] = []
for (const step of inputs.backend) {
    await advance(inputs.now + step.at)
    const rows: any[] = await (async () => {
        let list: any
        store.list().then((found: any) => (list = found))
        await advance(now + 150)
        return list
    })()
    const named = (name: string) => rows.find((one) => one.name === name)
    switch (step.do) {
        case 'create':
            void actions.createProject(step.name)
            break
        case 'rename':
            void actions.renameProject(named(step.name) ?? { id: 'gone' }, step.to)
            break
        case 'duplicate':
            void actions.duplicateProject(named(step.name))
            break
        case 'delete':
            void actions.deleteProject(named(step.name))
            break
        case 'save': {
            const project = named(step.name)
            void actions.saveEdit(project.id, model.deleteClips(project, step.ids, step.ripple))
            break
        }
        case 'prefs':
            void prefsStore.update('prefs', step.patch)
            break
    }
    await advance(now + 400)
    looks.push({
        projects: JSON.parse(localStorage.getItem('tidevann-demo:cut:projects') ?? 'null'),
        order: actions.byUpdated(JSON.parse(localStorage.getItem('tidevann-demo:cut:projects') ?? '[]')).map((one: any) => one.id),
        prefs: JSON.parse(localStorage.getItem('tidevann-demo:cut:prefs') ?? 'null'),
        toasts: toasts.splice(0).map((one: any) => ({
            type: one.type ?? null,
            title: one.title,
            description: one.description ?? null,
            action: one.actionProps?.children ?? null,
        })),
        went: went.splice(0),
    })
}

process.stdout.write(
    JSON.stringify({
        projects,
        sources: media.SOURCES,
        prefs: model.DEFAULT_PREFS,
        exportSizes: model.EXPORT_SIZES,
        tracks: model.defaultTracks(),
        words,
        edits,
        playback,
        sound,
        clipPeaks,
        mixes,
        exports,
        looks,
    }),
)
