/**
 * What the web's music modules give for the inputs in `inputs.json`, as JSON on stdout: run by
 * `tests/web.rs` with bun from the repository's root, the zone UTC, against `.parity/sokkel-ui` at
 * the commit `parity/music.json` names. The clock and the timers are held here, and `Math.random`
 * is a seeded run while the steps play. The player's two audio elements are stand-ins: a source
 * set loads at once and stops what played, `play` and `pause` say so as a browser does, a time set says so, and the end
 * of a song is a step of its own. The songs' sound is made in full, as the page makes it.
 */

const ROOT = new URL('../../../../.parity/sokkel-ui/examples/music/web/src/', import.meta.url).pathname
const MODULES = `${ROOT}../node_modules/`
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

// `new Date()` with no arguments reads the held clock too, as a page's own "today" does
const RealDate = Date
globalThis.Date = class extends RealDate {
    constructor(...args: unknown[]) {
        if (args.length === 0) super(RealDate.now())
        else super(...(args as [string]))
    }
} as DateConstructor
const kept = new Map<string, string>()

/** A stand-in for an audio element: loads at once, and says what a browser says. */
class FakeAudio {
    preload = ''
    volume = 1
    muted = false
    playbackRate = 1
    preservesPitch = true
    paused = true
    ended = false
    readyState = 0
    duration = Number.NaN
    error = null
    #src = ''
    #time = 0
    #listeners = new Map<string, { fn: () => void; once: boolean }[]>()
    get src() {
        return this.#src
    }
    set src(value: string) {
        this.#src = value
        this.readyState = 0
    }
    removeAttribute(name: string) {
        if (name === 'src') this.src = ''
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
        this.#listeners.set(name, (this.#listeners.get(name) ?? []).filter((one) => one.fn !== fn))
    }
    #emit(name: string) {
        const list = this.#listeners.get(name) ?? []
        this.#listeners.set(
            name,
            list.filter((one) => !one.once),
        )
        for (const one of list) one.fn()
    }
    /** A new source stops what played without a word, as a browser's load does. */
    load() {
        this.readyState = 1
        this.paused = true
        this.ended = false
        this.#emit('loadedmetadata')
    }
    play() {
        this.paused = false
        this.ended = false
        this.#emit('play')
        return Promise.resolve()
    }
    pause() {
        if (this.paused) return
        this.paused = true
        this.#emit('pause')
    }
    /** The song played to its end. */
    end() {
        this.ended = true
        this.paused = true
        this.#emit('pause')
        this.#emit('ended')
    }
}
const elements: FakeAudio[] = []
let urls = 0

Object.assign(globalThis, {
    setTimeout: setTimer,
    clearTimeout: clearTimer,
    setInterval: setEvery,
    clearInterval: clearTimer,
    requestAnimationFrame: (fn: (at: number) => void) => setTimer(() => fn(now), 16),
    cancelAnimationFrame: clearTimer,
    queueMicrotask: (fn: () => void) => void Promise.resolve().then(fn),
    Audio: class extends FakeAudio {
        constructor() {
            super()
            elements.push(this)
        }
    },
    window: { setTimeout: setTimer, clearTimeout: clearTimer, addEventListener: () => {}, removeEventListener: () => {} },
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
URL.createObjectURL = () => {
    urls += 1
    return `blob:song-${String(urls)}`
}
URL.revokeObjectURL = () => {}

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
        now = next.at
        if (next.every === undefined) timers.shift()
        else next.at += next.every
        next.fn()
    }
    now = to
    await settle()
}

const { configureDemo } = await import(`${MODULES}tidevann-shared/backend/index.ts`)
configureDemo({ app: 'music' })
const synth = await import(`${ROOT}synth.ts`)
const catalog = await import(`${ROOT}catalog.ts`)
const data = await import(`${ROOT}data.ts`)
const { download, removeDownload, removeAllDownloads, resumeDownloads } = await import(`${ROOT}downloads.ts`)
const { lyricsOf } = await import(`${ROOT}lyrics.ts`)
const look = await import(`${ROOT}look.ts`)
const { CARRIED } = await import(`${ROOT}themes.ts`)
const { find, searchGroups } = await import(`${ROOT}search.tsx`)
const player = await import(`${ROOT}player.ts`)
const { startRadio } = await import(`${ROOT}parts/actions.tsx`)
const queue = await import(`${MODULES}@winterop-com/sokkel-ui-media/dist/queue.js`)
const { parseLyrics, lyricLineAt } = await import(`${MODULES}@winterop-com/sokkel-ui-media/dist/lrc.js`)
const { toast } = await import(`${MODULES}@winterop-com/sokkel-ui/dist/components/ui/toast.js`)
toast.add = () => ''

/** FNV-1a over 32-bit words. */
function hash32(words: Uint32Array | Uint8Array): number {
    let hash = 0x811c9dc5
    for (let index = 0; index < words.length; index += 1) {
        hash ^= words[index]
        hash = Math.imul(hash, 0x01000193) >>> 0
    }
    return hash >>> 0
}

// --- The synthesizer --------------------------------------------------------------------------------
const sound = {
    random: inputs.seeds.map((seed: number) => {
        const next = synth.seeded(seed)
        return Array.from({ length: 6 }, () => next())
    }),
    lengths: inputs.lengthSeeds.map((seed: number) =>
        Object.keys(catalog.STYLE_NAMES).map((style) => ({ ...synth.barsOf(seed, style), length: synth.lengthOf(seed, style) })),
    ),
    renders: await Promise.all(
        inputs.renders.map(async (id: string) => {
            const song = catalog.songOf(id)
            const samples: Float32Array = synth.render(song.seed, song.style)
            const wav = new Uint8Array(await synth.wav(samples).arrayBuffer())
            const at = [0, 1000, 22050, 100000, samples.length - 1]
            return {
                id,
                length: samples.length,
                hash: hash32(new Uint32Array(samples.buffer)),
                samples: at.map((index) => samples[index]),
                waveform: synth.waveformOf(samples),
                coarse: synth.waveformOf(samples, 7),
                wavHead: Array.from(wav.slice(0, 44)),
                wavHash: hash32(wav),
            }
        }),
    ),
}

// --- The catalog --------------------------------------------------------------------------------------
const random = (seed: number) => synth.seeded(seed)
const library = {
    styles: catalog.STYLE_NAMES,
    artists: catalog.ARTISTS,
    albums: catalog.ALBUMS,
    songs: catalog.SONGS,
    stations: catalog.STATIONS,
    titles: inputs.titleSeeds.map((seed: number) => {
        const next = random(seed)
        return [catalog.titleFrom(next), catalog.titleFrom(next)]
    }),
    stationFrom: inputs.stationsFrom.map(([kind, ref]: [string, string]) => catalog.stationFrom(kind, ref) ?? null),
    radio: catalog.STATIONS.map((station: any) => [0, 1, 2, 3].map((index) => catalog.radioSong(station, index))),
    songOf: inputs.songIds.map((id: string) => catalog.songOf(id) ?? null),
    albumOf: inputs.songIds.flatMap((id: string) => {
        const song = catalog.songOf(id)
        return song === undefined ? [] : [catalog.albumOf(song)]
    }),
    albumsOfArtist: catalog.ARTISTS.map((artist: any) => catalog.albumsOfArtist(artist.id).map((album: any) => album.id)),
    songsOfArtist: catalog.ARTISTS.map((artist: any) => catalog.songsOfArtist(artist.id).length),
    lengthOfAlbums: catalog.ALBUMS.map((album: any) => catalog.lengthOfSongs(catalog.songsOfAlbum(album.id))),
    minutes: inputs.minutes.map(catalog.minutes),
    albumMinutes: catalog.ALBUMS.map((album: any) => catalog.minutes(catalog.lengthOfSongs(catalog.songsOfAlbum(album.id)))),
    bytesOf: catalog.SONGS.slice(0, 12).map(catalog.bytesOf),
    formatBytes: inputs.bytes.map(catalog.formatBytes),
}

// --- Words ----------------------------------------------------------------------------------------------
const sung = [...catalog.SONGS.filter((_: unknown, index: number) => index % 3 === 0), ...library.radio.flat()]
const lyrics = sung.map((song: any) => {
    const text = lyricsOf(song)
    if (text === undefined) return { id: song.id, text: null }
    const parsed = parseLyrics(text)
    return { id: song.id, text, parsed, at: inputs.lyricPositions.map((position: number) => lyricLineAt(parsed.lines, position)) }
})
const lrc = inputs.lrc.map((text: string) => {
    const parsed = parseLyrics(text)
    return { parsed, at: inputs.lyricPositions.map((position: number) => lyricLineAt(parsed.lines, position)) }
})

// --- Queues ----------------------------------------------------------------------------------------------
let added = 0
const queues = inputs.queues.map((script: any) => {
    const items = Array.from({ length: script.items }, (_, index) => `i${String(index)}`)
    let now = queue.createQueue(items, script.options)
    const out: unknown[] = [now]
    for (const [op, a, b] of script.ops) {
        let next: any
        if (op === 'next') next = queue.queueNext(now)
        else if (op === 'previous') next = queue.queuePrevious(now)
        else if (op === 'ended') next = queue.queueEnded(now)
        else if (op === 'play') next = queue.queuePlay(now, a)
        else if (op === 'shuffle') next = queue.queueShuffle(now, a, { seed: b })
        else if (op === 'repeat') next = queue.queueRepeat(now, a)
        else if (op === 'add')
            next = queue.queueAdd(
                now,
                Array.from({ length: a }, () => {
                    added += 1
                    return `a${String(added)}`
                }),
                b,
            )
        else if (op === 'remove') next = queue.queueRemove(now, a)
        else if (op === 'move') next = queue.queueMove(now, a, b)
        out.push(next ?? null)
        if (next !== undefined) now = next
    }
    return out
})
const shuffles = inputs.shuffles.map(([count, first, seed]: [number, number, number | string]) =>
    queue.shuffledOrder(count, first, queue.seededRandom(seed)),
)

// --- Search and looks ---------------------------------------------------------------------------------------
const ids = (found: any) => ({
    artists: found.artists.map((one: any) => one.id),
    albums: found.albums.map((one: any) => one.id),
    songs: found.songs.map((one: any) => one.id),
})
const search = inputs.queries.map((query: string) => ({
    all: ids(find(query)),
    three: ids(find(query, 3)),
    groups: searchGroups(query).map((group: any) => ({
        kind: group.kind,
        title: group.title,
        total: group.total,
        results: group.results.map((one: any) => ({ id: one.id, title: one.title, detail: one.detail, round: one.round === true })),
    })),
}))
const looks = {
    looks: look.LOOKS,
    ramps: look.RAMPS,
    effects: look.EFFECTS,
    isLook: inputs.looks.map(look.isLook),
    props: look.LOOKS.flatMap((one: any) =>
        look.RAMPS.flatMap((ramp: any) => look.EFFECTS.map((effect: any) => look.lookProps(one.id, ramp.value, effect.value))),
    ),
    themes: CARRIED,
}

// --- The library and the player, step by step ---------------------------------------------------------------
const rows = (name: string) => JSON.parse(kept.get(`tidevann-demo:music:${name}`) ?? 'null')
function look_(): unknown {
    const state = player.player.get()
    const keptPlayer = (rows('player') ?? []).find((row: any) => row.id === 'now') ?? null
    const song = player.currentSong(state)
    return {
        favorites: data.favorites.store.get(),
        playlists: data.playlists.store.get(),
        downloads: data.downloads.store.get(),
        stations: data.stations.store.get(),
        history: data.history.store.get(),
        settings: data.settings.get(),
        queue: state.queue,
        source: state.source,
        radio: state.radio,
        playing: state.playing,
        duration: state.duration,
        position: player.position.get(),
        song: song?.id ?? null,
        hasNext: player.hasNext(state),
        upNext: player.upNext(state).map((one: any) => one.position),
        kept: keptPlayer,
        keptSettings: (rows('settings') ?? [null])[0],
    }
}

const sourceOf = (step: any) => {
    if (step.album !== undefined) {
        const album = catalog.ALBUM_BY_ID.get(step.album)
        return {
            songs: catalog.songsOfAlbum(step.album).map((song: any) => song.id),
            source: { kind: 'album', label: album.title, href: `/albums/${album.id}` },
        }
    }
    if (step.artist !== undefined) {
        const artist = catalog.ARTIST_BY_ID.get(step.artist)
        return {
            songs: catalog.songsOfArtist(step.artist).map((song: any) => song.id),
            source: { kind: 'artist', label: artist.name, href: `/artists/${artist.id}` },
        }
    }
    const playlist = data.playlists.store.get().find((row: any) => row.id === step.playlist)
    return {
        songs: playlist.entries.map((entry: any) => entry.song),
        source: { kind: 'playlist', label: playlist.name, href: `/playlists/${playlist.id}` },
    }
}

// Started as the app starts: the stations known, the kept rows read, the player picked up.
const startedRandom = Math.random
Math.random = synth.seeded(inputs.random)
data.stations.store.subscribe(() => catalog.knowStations(data.stations.store.get() ?? []))
resumeDownloads()
let ready: any
void data.stations.source
    .list()
    .then((rows: any) => {
        catalog.knowStations(rows)
        return data.loadKept()
    })
    .then((row: any) => {
        ready = row
        player.restore(row, data.settings.get().resume)
    })
await advance(inputs.now + 900)
/** Each step's look holds only what changed since the step before; the first holds everything. */
let last: Record<string, string> = {}
function changed(seen: any): Record<string, unknown> {
    const out: Record<string, unknown> = {}
    const written: Record<string, string> = {}
    for (const [key, value] of Object.entries(seen)) {
        written[key] = JSON.stringify(value)
        if (written[key] !== last[key]) out[key] = value
    }
    last = written
    return out
}
const steps: unknown[] = [changed(look_())]

for (const step of inputs.steps) {
    await advance(inputs.now + step.at)
    switch (step.op) {
        case 'fav':
            data.setFavorite(step.kind, step.ref, step.on)
            break
        case 'createPlaylist':
            data.createPlaylist(step.name, step.songs)
            break
        case 'addToPlaylist':
            data.addToPlaylist(step.id, step.songs)
            break
        case 'renamePlaylist':
            data.changePlaylist(step.id, { name: step.name.trim() })
            break
        case 'describePlaylist':
            data.changePlaylist(step.id, { description: step.description.trim() })
            break
        case 'movePlaylist': {
            const playlist = data.playlists.store.get().find((row: any) => row.id === step.id)
            const entries = playlist.entries.filter((entry: any) => catalog.songOf(entry.song) !== undefined)
            const [moved] = entries.splice(step.from, 1)
            entries.splice(step.to, 0, moved)
            data.changePlaylist(step.id, { entries })
            break
        }
        case 'removeFromPlaylist': {
            const playlist = data.playlists.store.get().find((row: any) => row.id === step.id)
            const key = playlist.entries[step.index].key
            data.changePlaylist(step.id, { entries: playlist.entries.filter((entry: any) => entry.key !== key) })
            break
        }
        case 'deletePlaylist':
            data.deletePlaylist(step.id)
            break
        case 'download':
            download(step.album)
            break
        case 'removeDownload':
            removeDownload(step.album)
            break
        case 'removeAllDownloads':
            removeAllDownloads()
            break
        case 'settings':
            data.setSettings(step.patch)
            break
        case 'clearHistory':
            data.clearHistory()
            break
        case 'startRadio':
            startRadio(step.kind, step.ref)
            break
        case 'playSongs': {
            const { songs, source } = sourceOf(step)
            player.playSongs(songs, step.start, source, { shuffle: step.shuffle })
            break
        }
        case 'playStation':
            player.playStation(step.id, step.label)
            break
        case 'next':
            player.next()
            break
        case 'previous':
            player.previous()
            break
        case 'seek':
            player.seek(step.to)
            break
        case 'seekBy':
            player.seekBy(step.by)
            break
        case 'toggle':
            player.togglePlay()
            break
        case 'shuffle':
            player.setShuffle(step.on)
            break
        case 'repeat':
            player.setRepeat(step.mode)
            break
        case 'enqueue':
            player.enqueue(step.songs, step.where)
            break
        case 'dequeue':
            player.dequeue(step.positions)
            break
        case 'move':
            player.moveInQueue(step.from, step.to)
            break
        case 'clearUpNext':
            player.clearUpNext()
            break
        case 'playIndex':
            player.playIndex(step.index)
            break
        case 'playPosition':
            player.playPosition(step.at)
            break
        case 'ended': {
            const sounding = elements.find((element) => element === player.player.get().element) ?? elements[0]
            sounding.end()
            break
        }
        case 'restore':
            player.restore(step.row, step.resume)
            break
    }
    await advance(inputs.now + step.at + 900)
    steps.push(changed(look_()))
}
Math.random = startedRandom

process.stdout.write(
    JSON.stringify({
        sound,
        library,
        lyrics,
        lrc,
        queues,
        shuffles,
        search,
        looks,
        defaults: { player: data.DEFAULT_PLAYER, settings: data.DEFAULT_SETTINGS, restored: ready },
        steps,
    }),
)
