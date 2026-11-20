/**
 * What the web's video modules give for the inputs in `inputs.json`, as JSON on stdout: run by
 * `tests/web.rs` with bun from the repository's root, the zone UTC, against `.parity/sokkel-ui` at
 * the commit `parity/video.json` names. The clock and the timers are held here, `performance.now`
 * reads the same clock, and `Math.random` is a seeded run while the steps play. The pages run as
 * components on a stand-in for React's hooks (`react.ts`), each mounted once and rendered again
 * after each change as React would; what a page shows is read from what it draws, and what its
 * buttons do is done by calling the handlers it hands its parts.
 *
 * The browser's clip maker is a stand-in: a worker that answers each clip at once, its file as
 * large as the rendition's bitrate over the video's length, as the egui site paints each clip as it
 * plays and needs none made. IndexedDB is missing, as in a private window, so nothing is kept
 * between visits.
 */

const ROOT = new URL('../../../../.parity/sokkel-ui/examples/video/web/src/', import.meta.url).pathname
const MODULES = `${ROOT}../node_modules/`
const UI = `${MODULES}@winterop-com/sokkel-ui/dist/components/`
const MEDIA = `${MODULES}@winterop-com/sokkel-ui-media/dist/`
const inputs = JSON.parse(await Bun.file(new URL('inputs.json', import.meta.url).pathname).text())

// --- The clock and the timers, moved by hand ------------------------------------------------------
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
performance.now = () => now

const kept = new Map<string, string>()
const location = {
    href: `${inputs.base}#/`,
    get hash() {
        return hash
    },
    set hash(next: string) {
        hash = next
        went.push(next)
    },
    replace: (next: string) => went.push(`replace ${next}`),
}
let hash = '#/'
let went: string[] = []
const blobs = new Map<string, Blob>()
let urls = 0

/** The worker that makes clips, answering each at once with a file of the rendition's size. */
class FakeWorker {
    onmessage: ((event: { data: unknown }) => void) | null = null
    postMessage(job: any) {
        setTimer(() => {
            const size = Math.round((job.bitrate * job.spec.duration) / 8)
            this.onmessage?.({ data: { id: job.id, kind: 'done', blob: { size }, sound: true } })
        }, 0)
    }
}

Object.assign(globalThis, {
    setTimeout: setTimer,
    clearTimeout: clearTimer,
    setInterval: setEvery,
    clearInterval: clearTimer,
    requestAnimationFrame: (fn: (at: number) => void) => setTimer(() => fn(now), 16),
    cancelAnimationFrame: clearTimer,
    VideoEncoder: class {},
    OffscreenCanvas: class {},
    Worker: FakeWorker,
    window: {
        location,
        innerWidth: inputs.width,
        setTimeout: setTimer,
        clearTimeout: clearTimer,
        setInterval: setEvery,
        clearInterval: clearTimer,
        addEventListener: () => {},
        removeEventListener: () => {},
        matchMedia: () => ({ matches: false, addEventListener() {}, removeEventListener() {} }),
        history: { go: () => {} },
    },
    document: {
        createElement: () => ({ getContext: () => null }),
        addEventListener: () => {},
        removeEventListener: () => {},
        visibilityState: 'visible',
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
URL.createObjectURL = (blob: Blob) => {
    urls += 1
    const url = `blob:video-${String(urls)}`
    blobs.set(url, blob)
    return url
}
URL.revokeObjectURL = () => {}

const { configureDemo } = await import(`${MODULES}tidevann-shared/backend/index.ts`)
configureDemo({ app: 'video' })
const R = await import(new URL('react.ts', import.meta.url).pathname)
// React's own hooks ask the dispatcher React keeps; the stand-in answers them.
const React = await import(`${MODULES}react/index.js`)
;(React.default ?? React).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE.H = R.dispatcher

const catalog = await import(`${ROOT}catalog.ts`)
const format = await import(`${ROOT}format.ts`)
const { CARRIED } = await import(`${ROOT}themes.ts`)
const clips = await import(`${ROOT}clips/index.ts`)
const { clipSource } = await import(`${ROOT}clips/source.ts`)
const data = await import(`${ROOT}data.ts`)
const dialogs = await import(`${ROOT}dialogs.tsx`)
const parts = await import(`${ROOT}parts.tsx`)
const site = await import(`${ROOT}Site.tsx`)
const { Home } = await import(`${ROOT}pages/Home.tsx`)
const { Search } = await import(`${ROOT}pages/Search.tsx`)
const { Subscriptions } = await import(`${ROOT}pages/Subscriptions.tsx`)
const { History } = await import(`${ROOT}pages/History.tsx`)
const { Playlist } = await import(`${ROOT}pages/Playlist.tsx`)
const { Playlists } = await import(`${ROOT}pages/Playlists.tsx`)
const { Library } = await import(`${ROOT}pages/Library.tsx`)
const { Liked } = await import(`${ROOT}pages/Liked.tsx`)
const { Channel } = await import(`${ROOT}pages/Channel.tsx`)
const { Downloads } = await import(`${ROOT}pages/Downloads.tsx`)
const { Watch } = await import(`${ROOT}pages/Watch.tsx`)
const aside = await import(`${ROOT}pages/watch/Aside.tsx`)
const { Comments } = await import(`${ROOT}pages/watch/Comments.tsx`)
const { Settings } = await import(`${ROOT}pages/Settings.tsx`)
const { DEFAULT_APPEARANCE } = await import(`${MODULES}@winterop-com/sokkel-ui/dist/lib/appearance.js`)
const { SettingsDialog } = await import(`${UI}SettingsDialog.js`)
const { synthesize } = await import(`${MODULES}tidevann-shared/clips/sound.ts`)
const { seeded } = await import(`${MODULES}tidevann-shared/clips/scenes.ts`)
const { ChipBar } = await import(`${UI}ChipBar.js`)
const { Segmented } = await import(`${UI}Segmented.js`)
const { SearchField } = await import(`${UI}SearchField.js`)
const { QuickAdd } = await import(`${UI}QuickAdd.js`)
const { InlineEdit } = await import(`${UI}InlineEdit.js`)
const { SortableList } = await import(`${UI}Sortable.js`)
const { ConfirmDialog } = await import(`${UI}DialogShell.js`)
const { PageHeader } = await import(`${UI}PageHeader.js`)
const { PageState } = await import(`${UI}PageState.js`)
const { Section } = await import(`${UI}Section.js`)
const { DescriptionList } = await import(`${UI}DescriptionList.js`)
const { DataTable } = await import(`${UI}DataTable.js`)
const { CommentThread, CommentComposer } = await import(`${UI}CommentThread.js`)
const { MetaLine } = await import(`${UI}MetaLine.js`)
const { SwitchField } = await import(`${UI}SwitchField.js`)
const { StatusLine, StatusItem } = await import(`${UI}StatusLine.js`)
const { CheckboxField } = await import(`${UI}CheckboxField.js`)
const { Button } = await import(`${UI}ui/button.js`)
const { Select } = await import(`${UI}ui/select.js`)
const { DropdownMenuItem } = await import(`${UI}ui/dropdown-menu.js`)
const { EmptyTitle, EmptyDescription } = await import(`${UI}ui/empty.js`)
const { toast } = await import(`${UI}ui/toast.js`)
const { ChannelRow } = await import(`${MEDIA}ChannelRow.js`)
const { MediaCard } = await import(`${MEDIA}MediaCard.js`)
const { MediaActions } = await import(`${MEDIA}MediaActions.js`)
const { UpNext } = await import(`${MEDIA}UpNext.js`)
const { Toolbar, ToolbarTitle } = await import(`${UI}Toolbar.js`)
const { DownloadButton } = await import(`${UI}DownloadButton.js`)

let toasts: unknown[] = []
const undo: (() => void)[] = []
toast.add = (one: any) => {
    toasts.push({
        type: one.type ?? null,
        title: one.title,
        description: one.description ?? null,
        action: one.actionProps?.children ?? null,
    })
    if (one.actionProps?.onClick !== undefined) undo.push(one.actionProps.onClick)
    return ''
}

/** FNV-1a over 32-bit words. */
function hash32(words: Uint32Array | Uint8Array): number {
    let value = 0x811c9dc5
    for (let index = 0; index < words.length; index += 1) {
        value ^= words[index]
        value = Math.imul(value, 0x01000193) >>> 0
    }
    return value >>> 0
}

// --- What the tree draws ------------------------------------------------------------------------------

/** Every element in a tree, depth first, the root first. */
function all(node: any, out: any[] = []): any[] {
    if (node === null || node === undefined || typeof node !== 'object') return out
    if (Array.isArray(node)) {
        for (const one of node) all(one, out)
        return out
    }
    out.push(node)
    const props = node.props ?? {}
    for (const key of Object.keys(props)) {
        const value = props[key]
        if (key === 'children' || (typeof value === 'object' && value !== null && (Array.isArray(value) || value.$$typeof !== undefined)))
            all(value, out)
    }
    return out
}
const find = (tree: any, type: unknown) => all(tree).filter((node) => node.type === type)
const first = (tree: any, type: unknown) => find(tree, type)[0]
/** The words of a tree: its strings and numbers, joined. */
function text(node: any): string {
    if (node === null || node === undefined || typeof node === 'boolean') return ''
    if (typeof node === 'string' || typeof node === 'number') return String(node)
    if (Array.isArray(node)) return node.map(text).join('')
    if (typeof node === 'object') return text(node.props?.children)
    return ''
}

/** Lets promises settle and renders again while a state is set, as React would. */
async function flush() {
    for (let round = 0; round < 100; round += 1) {
        for (let i = 0; i < 30; i += 1) await Promise.resolve()
        if (!R.renderStale()) return
    }
    throw new Error('the pages never settled')
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

// --- The catalog, the formats and the clips ------------------------------------------------------------
const video = (id: string) => catalog.videoById(id)
const library = {
    viewer: catalog.VIEWER,
    peopleColors: catalog.PEOPLE_COLORS,
    channels: catalog.CHANNELS,
    topics: catalog.TOPICS,
    videos: catalog.VIDEOS,
    channelOf: catalog.VIDEOS.map((one: any) => catalog.channelOf(one).id),
    videosOf: catalog.CHANNELS.map((one: any) => catalog.videosOf(one).map((v: any) => v.id)),
    byId: [...catalog.VIDEOS.map((one: any) => one.id), 'nowhere', undefined].map((id) => catalog.videoById(id)?.id ?? null),
    channelById: [...catalog.CHANNELS.map((one: any) => one.id), 'nowhere'].map((id) => catalog.channelById(id)?.id ?? null),
    seeds: [...catalog.VIDEOS.map((one: any) => one.id), ...inputs.seeds].map(catalog.seedOf),
    themes: CARRIED,
    renditions: clips.RENDITIONS,
    renditionOf: inputs.renditionOf.map((id: string) => clips.renditionOf(id).id),
    specs: catalog.VIDEOS.map(clips.specOf),
    random: catalog.VIDEOS.map((one: any) => {
        const next = seeded(catalog.seedOf(one.id))
        return Array.from({ length: 8 }, () => next())
    }),
    upNext: catalog.VIDEOS.map((one: any) => aside.upNextFor(one).map((v: any) => v.id)),
    videoText: catalog.VIDEOS.map(site.videoText),
    hrefs: inputs.hrefs.map(([id, list]: [string, string | null]) => parts.watchHref(video(id), list ?? undefined)),
    links: inputs.links.map(([id, at]: [string, number | null]) => dialogs.linkOf(video(id), at ?? undefined)),
}
const captions = await Promise.all(catalog.VIDEOS.map((one: any) => blobs.get(clips.captionsOf(one))!.text()))
const words = {
    short: inputs.short.map(format.short),
    plural: inputs.plural.map(([count, one, many]: [number, string, string | null]) => format.plural(count, one, many ?? undefined)),
    viewsOf: inputs.short.map(format.viewsOf),
    daysAgo: inputs.daysAgo.map(format.daysAgo),
    bytesOf: inputs.bytes.map(format.bytesOf),
    bitrateOf: inputs.bitrates.map(format.bitrateOf),
    clockOf: inputs.clocks.map(format.clockOf),
    captions,
}

/** A clip's sound in brief: how long, a hash of each channel's bits and a few samples. */
const sound = inputs.sound.map((id: string) => {
    const [left, right] = synthesize(clips.specOf(video(id)))
    const at = [0, 1000, 48000, 480000, left.length - 1]
    return {
        id,
        frames: left.length,
        left: hash32(new Uint32Array(left.buffer)),
        right: hash32(new Uint32Array(right.buffer)),
        samples: at.map((index) => [left[index] ?? null, right[index] ?? null]),
    }
})

// The source's stats, its bandwidth read at each second given.
const fakeClip = (id: string, rendition: string) => {
    const one = video(id)
    const chosen = clips.renditionOf(rendition)
    const size = Math.round((chosen.bitrate * one.duration) / 8)
    return { url: 'blob:x', blob: { size }, rendition: chosen, bitrate: Math.round((size * 8) / one.duration), sound: true, made: 0, kept: false }
}
const sources = inputs.bandwidth.map(([id, rendition, seconds]: [string, string, number]) => {
    now = seconds * 1000
    return clipSource(fakeClip(id, rendition)).stats()
})
now = inputs.now

// --- The pages, mounted once ---------------------------------------------------------------------------
const props: Record<string, any> = {
    search: '',
    channel: 'harbor',
    playlist: 'later',
    comments: 'first-light',
    aside: 'first-light',
    asideList: null,
    route: [],
    save: 'first-light',
    share: { video: 'first-light', at: undefined },
    card: { id: 'first-light', layout: 'stack', bare: false, list: undefined, playing: false },
    menu: 'first-light',
}
// Each collection's rows as the backend first answers, for the steps' first look.
const seeds: Record<string, unknown[]> = {}
for (const [name, collection] of [
    ['follows', data.follows],
    ['ratings', data.ratings],
    ['history', data.watched],
    ['playlists', data.playlists],
    ['comments', data.comments],
    ['downloads', data.downloads],
] as const)
    void collection.list().then((rows: unknown[]) => (seeds[name] = rows))
const prefsView = R.mount(() => data.usePrefs())
const pages = {
    site: R.mount(() => site.Site({ frame: { route: props.route, palette: [], appearance: { settings: {}, setSettings: () => {} } } })),
    home: R.mount(() => Home()),
    search: R.mount(() => Search({ query: props.search })),
    subscriptions: R.mount(() => Subscriptions()),
    history: R.mount(() => History({ clearing: props.clearing === true })),
    playlist: R.mount(() => Playlist({ id: props.playlist })),
    playlists: R.mount(() => Playlists()),
    library: R.mount(() => Library()),
    liked: R.mount(() => Liked()),
    channel: R.mount(() => Channel({ id: props.channel, tab: 'videos' })),
    about: R.mount(() => Channel({ id: props.channel, tab: 'about' })),
    downloads: R.mount(() => Downloads()),
    comments: R.mount(() => {
        const one = video(props.comments)
        return Comments({ video: one, channel: catalog.channelOf(one) })
    }),
    aside: R.mount(() => {
        const history = data.useById(data.watched)
        const { rows } = data.useRows(data.playlists)
        const list = props.asideList === null ? undefined : rows.find((one: any) => one.id === props.asideList)
        return aside.Aside({ video: video(props.aside), list, history })
    }),
}
const settingsPage = R.mount(() =>
    Settings({
        frame: { route: ['settings'], palette: [], themes: [], appearance: { settings: DEFAULT_APPEARANCE, setSettings: () => {} } },
        open: true,
        onClose: () => {},
    }),
)
/** The Settings page's settings, by id. */
function settingsById(): Record<string, any> {
    const dialog = first(tree(settingsPage), SettingsDialog).props
    const out: Record<string, any> = { onReset: dialog.onReset }
    for (const group of dialog.groups) for (const section of group.sections) for (const one of section.settings) out[one.id] = one
    return out
}
const provider = R.mount(() => dialogs.DialogsProvider({ children: null })).current()
const SaveDialog = provider.props.children[1].type
const ShareDialog = provider.props.children[2].type
const savePage = R.mount(() => SaveDialog({ video: video(props.save), onClose: () => {} }))
const sharePage = R.mount(() => {
    const shared = { video: video(props.share.video), at: props.share.at }
    return ShareDialog({ shared, onClose: () => {} })
})
const cardPage = R.mount(() =>
    parts.VideoCard({
        video: video(props.card.id),
        layout: props.card.layout,
        bare: props.card.bare,
        list: props.card.list,
        playing: props.card.playing,
    }),
)
const menuPage = R.mount(() => parts.VideoMenu({ video: video(props.menu) }))
const playlistRows = R.mount(() => data.useRows(data.playlists))

/** The watch page open now: its video and playlist, and the state of the player it is handed. */
let watching: { page: any; id: string; list: string | null; player: { time: number; playing: boolean } } | undefined
const handle = (state: { time: number; playing: boolean }) => ({
    time: () => state.time,
    playing: () => state.playing,
    play: () => {},
    pause: () => {},
    toggle: () => {},
    seekTo: () => {},
    seekBy: () => {},
    toggleMute: () => {},
    toggleFullscreen: () => {},
    stats: () => null,
})
function openWatch(id: string, list: string | null, at: number | null) {
    const outer = R.mount(() => Watch({ id, list: list ?? undefined, at: at ?? undefined }))
    const element = outer.current()
    const state = { time: 0, playing: false }
    const page = R.mount(() => element.type(element.props))
    watching = { page, id, list, player: state }
}
const tree = (page: any) => page.current()
const player = () => tree(watching!.page).props.player
const attachHandle = () => {
    const ref = player().props.ref
    if (ref !== undefined && ref !== null) ref.current = watching === undefined ? null : handle(watching.player)
}

// --- Reading the pages ----------------------------------------------------------------------------------
const empty = (node: any) => {
    const title = first(node, EmptyTitle)
    return title === undefined ? null : [text(title), text(first(node, EmptyDescription))]
}
const cardIds = (node: any) => all(node).filter((one) => one.props?.video !== undefined && typeof one.type === 'function' && one.type === parts.VideoCard).map((one) => one.props.video.id)
const loading = (node: any) => first(node, PageState)?.props.loading === true

async function chooseIn(page: any, type: unknown, value: unknown, pick: (node: any) => boolean = () => true) {
    const node = find(tree(page), type).find(pick)
    if (node === undefined) return
    if (type === SearchField) node.props.onChange(value)
    else node.props.onValueChange(value)
    await flush()
}

const readers: Record<string, (view: any) => Promise<unknown>> = {
    async home(view) {
        await chooseIn(pages.home, ChipBar, view.chip)
        const node = tree(pages.home)
        const shown = cardIds(node)
        const said = empty(node)
        return { loading: loading(node), videos: shown, empty: said === null ? null : said[1] }
    },
    async search(view) {
        props.search = view.query
        pages.search.render()
        await flush()
        await chooseIn(pages.search, Segmented, view.sort)
        await chooseIn(pages.search, ChipBar, view.kind)
        const node = tree(pages.search)
        const header = first(node, PageHeader).props
        const channels = find(node, ChannelRow).map((row: any) => ({
            id: row.props.href.replace('#/channel/', ''),
            detail: row.props.detail,
            following: row.props.following,
        }))
        return {
            title: header.title,
            count: header.count,
            chips: first(node, ChipBar).props.chips.map((one: any) => one.label),
            channels,
            videos: cardIds(node),
            empty: empty(node),
        }
    },
    async subscriptions(view) {
        await chooseIn(pages.subscriptions, ChipBar, view.chosen)
        const node = tree(pages.subscriptions)
        const sections = find(node, Section)
        const chips = first(node, ChipBar)
        const ids = (title: string) => {
            const section = sections.find((one: any) => one.props.title === title)
            return section === undefined ? [] : cardIds(section)
        }
        const said = empty(node)
        return {
            loading: loading(node),
            count: first(node, PageHeader).props.count,
            chosen: chips?.props.value ?? 'all',
            chips: chips === undefined ? [['all', 'All']] : chips.props.chips.map((one: any) => [one.value, one.label]),
            week: ids('This week'),
            earlier: ids('Earlier'),
            none_followed: said !== null && said[0] === 'You follow no channels yet',
            no_videos: said !== null && said[0] === 'No videos yet',
        }
    },
    async history(view) {
        await chooseIn(pages.history, SearchField, view.search)
        const node = tree(pages.history)
        const header = first(node, PageHeader).props
        return {
            loading: loading(node),
            count: header.count,
            clear_disabled: header.actions.props.disabled,
            keep: first(node, SwitchField).props.checked,
            days: find(node, Section).map((one: any) => ({ label: one.props.title, videos: cardIds(one) })),
            empty: empty(node),
        }
    },
    async playlist(view) {
        props.playlist = view.id
        pages.playlist.render()
        await flush()
        const node = tree(pages.playlist)
        const header = first(node, PageHeader)
        if (header === undefined) return null
        const later = typeof header.props.title === 'string'
        const buttons = find(header.props.actions, Button)
        const sortable = first(node, SortableList)
        const videos =
            sortable === undefined
                ? []
                : sortable.props.items.map((one: any, index: number) => {
                      const card = sortable.props.renderItem(one, { index })
                      const [up, down, remove] = card.props.extra.props.children
                      return {
                          id: one.id,
                          up_disabled: up.props.disabled,
                          down_disabled: down.props.disabled,
                          remove: remove.props.label,
                      }
                  })
        const said = empty(node)
        return {
            name: later ? header.props.title : header.props.title.props.value,
            later,
            count: header.props.count,
            description: later ? header.props.description : header.props.description.props.value,
            play_disabled: buttons[0].props.disabled,
            shuffle_disabled: buttons[1].props.disabled,
            deletable: buttons.length > 2,
            videos,
            empty: said === null ? null : said[1],
            question: first(node, ConfirmDialog).props.question,
        }
    },
    async playlists() {
        const node = tree(pages.playlists)
        return {
            loading: loading(node),
            count: first(node, PageHeader).props.count,
            cards: all(node)
                .filter((one) => one.type === pagesPlaylistCard)
                .map((one) => playlistCard(one.props.list)),
        }
    },
    async library() {
        const node = tree(pages.library)
        return {
            loading: loading(node),
            parts: find(node, Section).map((section: any) => ({
                title: section.props.title,
                meta: section.props.meta,
                all: (() => {
                    const before = went.length
                    section.props.actions.props.onClick()
                    return went.splice(before)[0] ?? null
                })(),
                videos: cardIds(section),
                lists: all(section)
                    .filter((one) => one.type === pagesPlaylistCard)
                    .map((one) => playlistCard(one.props.list)),
                empty: (() => {
                    const p = all(section).find((one) => one.type === 'p')
                    return p === undefined ? null : text(p)
                })(),
            })),
        }
    },
    async liked() {
        const node = tree(pages.liked)
        return cardIds(node)
    },
    async channel(view) {
        props.channel = view.id
        pages.channel.render()
        await flush()
        await chooseIn(pages.channel, Segmented, view.sort)
        const node = tree(pages.channel)
        const row = first(node, ChannelRow).props
        // About is the other tab: the same page drawn under its own tab.
        pages.about.render()
        await flush()
        const items = first(tree(pages.about), DescriptionList).props.items
        return {
            detail: row.detail,
            following: row.following,
            videos: cardIds(node),
            about: items.map((one: any) => [one.term, typeof one.value === 'string' ? one.value : text(first(one.value, 'span'))]),
        }
    },
    async downloads() {
        const node = tree(pages.downloads)
        const table = first(node, DataTable).props
        const columns = table.columns
        return {
            loading: table.loading,
            count: first(node, PageHeader).props.count,
            rows: table.rows.map((one: any) => ({
                id: one.video.id,
                title: one.video.title,
                channel: catalog.channelOf(one.video).name,
                length: columns[1].cell(one),
                quality: columns[2].cell(one),
                size: columns[3].cell(one),
                at: one.download.at,
            })),
        }
    },
    async comments(view) {
        props.comments = view.video
        pages.comments.render()
        await flush()
        const select = first(tree(pages.comments), Select)
        select.props.onValueChange(view.order)
        await flush()
        const node = tree(pages.comments)
        const thread = first(node, CommentThread).props
        const menu = (comment: any) => text(thread.menu(comment))
        const read = (comment: any): unknown => ({
            id: comment.id,
            author: comment.author,
            color: comment.color,
            at: comment.time.props.date.getTime(),
            time_title: comment.timeTitle,
            body: comment.body,
            likes: comment.likes ?? null,
            liked: comment.liked,
            disliked: comment.disliked,
            pinned: comment.pinned,
            creator: comment.creator,
            reactions: comment.reactions.map((one: any) => [one.id, one.label, one.count, one.mine]),
            replies: comment.replies.map(read),
            menu: menu(comment),
        })
        return { title: text(first(node, ToolbarTitle)), threads: thread.comments.map(read) }
    },
    async aside(view) {
        props.aside = view.video
        props.asideList = view.list ?? null
        pages.aside.render()
        await flush()
        const chips = first(tree(pages.aside), ChipBar)
        if (chips !== undefined) {
            chips.props.onValueChange(view.filter)
            await flush()
        }
        const node = tree(pages.aside)
        const meta = first(node, MetaLine)
        const chipBar = first(node, ChipBar)
        return {
            title: text(first(node, ToolbarTitle)),
            meta: meta === undefined ? null : meta.props.items,
            chips: chipBar === undefined ? [] : chipBar.props.chips.map((one: any) => one.label),
            autoplay: first(node, SwitchField).props.checked,
            videos: all(node)
                .filter((one) => one.type === parts.VideoCard)
                .map((one) => ({ id: one.props.video.id, list: one.props.list ?? null, playing: one.props.playing === true })),
        }
    },
    async card(view) {
        props.card = { id: view.id, layout: view.layout, bare: view.bare === true, list: view.list ?? undefined, playing: view.playing === true }
        cardPage.render()
        await flush()
        const card = tree(cardPage).props
        const subtitle = card.subtitle
        const line = first(subtitle, MetaLine)
        const row = subtitle.type !== MetaLine
        return {
            id: view.id,
            title: card.title,
            channel: row ? text(subtitle.props.children[0]) : null,
            meta: line.props.items,
            badge: card.badge,
            progress: card.progress ?? null,
            playing: card.playing,
            href: card.href,
        }
    },
    async menu(view) {
        props.menu = view.id
        menuPage.render()
        await flush()
        return find(tree(menuPage), DropdownMenuItem).map(text)
    },
    async save(view) {
        props.save = view.id
        savePage.render()
        await flush()
        const node = tree(savePage)
        const order = sortedLists()
        return find(node, CheckboxField).map((one: any, index: number) => ({
            id: order[index],
            label: one.props.label,
            description: one.props.description,
            checked: one.props.checked,
        }))
    },
    async share(view) {
        props.share = { video: view.id, at: view.second ?? undefined }
        sharePage.render()
        await flush()
        const box = first(tree(sharePage), CheckboxField)
        if (box !== undefined && view.fromHere === true) {
            box.props.onCheckedChange(true)
            await flush()
        }
        const node = tree(sharePage)
        const input = all(node).find((one) => one.props?.['aria-label'] === 'Link')
        const start = first(node, CheckboxField)
        return { title: node.props.description, link: input.props.value, start_at: start === undefined ? null : start.props.label }
    },
    async site(view) {
        props.route = view.route
        pages.site.render()
        await flush()
        const node = tree(pages.site).props
        const groups = node.rail.props.groups.map((group: any) => ({
            id: group.id,
            title: group.title ?? null,
            entries: group.entries.map((one: any) => ({
                id: one.id,
                label: one.label,
                description: one.description ?? null,
                href: one.href,
                active: one.active,
                count: one.count ?? null,
                dot: one.dot === undefined ? null : one.dot.replace('var(--kind-', '').replace(')', ''),
                keywords: one.keywords ?? [],
            })),
        }))
        const foot = node.foot.props
        const items = find(foot.children, StatusItem)
        const command = (one: any) => ({ id: one.id, label: one.label, hint: one.hint ?? null, keywords: one.keywords ?? [] })
        const group = (title: string) => node.palette.find((one: any) => one.title === title)?.commands.map(command) ?? []
        return {
            rail: groups,
            trail: node.top.props.trail.map((one: any) => ({ label: one.label, href: one.href ?? null })),
            playlists: group('Playlists'),
            player: group('Player'),
            status: {
                note: foot.note,
                live: foot.live,
                watched: items[0].props.children,
                downloaded: items[1].props.children,
            },
        }
    },
    async settings() {
        const found = settingsById()
        const control = (id: string) => found[id].control.props
        const options = (id: string) => control(id).options.map((one: any) => [one.value, one.label])
        const select = (id: string) => (control(id).children as any[]).map((one: any) => [one.props.value, text(one)])
        return {
            toggles: ['autoplay', 'theater', 'stats', 'captions-on', 'keep-history'].map((id) => control(id).on),
            speed: control('speed').value,
            speeds: select('speed'),
            quality: control('quality').value,
            qualities: options('quality'),
            captions: {
                size: control('caption-size').value,
                color: control('caption-color').value,
                background: control('caption-background').value,
                edge: control('caption-edge').value,
            },
            sizes: options('caption-size'),
            colors: select('caption-color'),
            backgrounds: options('caption-background'),
            edges: options('caption-edge'),
            history: found['clear-history'].description,
            clear_disabled: control('clear-history').disabled,
            downloads: found.downloads.description,
        }
    },
    async watch() {
        attachHandle()
        const node = tree(watching!.page)
        const view = node.props
        const play = view.player.props
        const row = first(view.children, ChannelRow).props
        const actions = row.actions.props
        const download = actions.actions.find((one: any) => one.id === 'download')
        const button = download.render.props
        const stats = play.overlay?.props.rows ?? null
        const cover = play.cover
        const meta = first(view.children, MetaLine)
        const upNext = cover !== undefined && cover.type === UpNext ? cover.props : undefined
        return {
            title: text(all(view.children).find((el) => el.type === 'h1')),
            channel: row.name,
            followers: row.followers,
            following: row.following,
            rating: actions.rating,
            likes: actions.likes,
            saved_label: actions.actions.find((a: any) => a.id === 'save').label,
            download_label: download.label,
            download_state: button.state,
            download_value: button.value ?? null,
            kept_detail: button.keptDetail ?? null,
            meta: meta.props.items,
            attached: play.source !== undefined,
            start_at: play.startAt ?? null,
            play_on_load: play.autoPlay,
            rate: play.defaultRate,
            quality: play.quality,
            theater: view.theater,
            qualities: play.qualities.map((q: any) => [q.value, q.label, q.detail]),
            stats: stats === null ? null : stats.map((r: any) => [r.label, r.value]),
            up_next:
                upNext === undefined
                    ? null
                    : { id: catalog.VIDEOS.find((v: any) => v.title === upNext.title).id, title: upNext.title, subtitle: upNext.subtitle, countdown: upNext.countdown ?? null },
            next: nextOfWatching(),
        }
    },
}

const { PlaylistCard: pagesPlaylistCard } = await import(`${ROOT}pages/Playlists.tsx`)
function playlistCard(list: any) {
    const card = (pagesPlaylistCard as any)({ list }).props
    const items = find(card.menu, DropdownMenuItem)
    return {
        id: list.id,
        title: card.title,
        subtitle: card.subtitle,
        cover: list.videos[0] !== undefined && video(list.videos[0]) !== undefined ? list.videos[0] : null,
        badge: card.badge,
        href: card.href,
        deletable: items.length > 1,
    }
}
/** The playlists' ids in the order Save to lists them, which its boxes do not say. */
function sortedLists(): string[] {
    const shown = playlistRows.current().rows
    return [...shown]
        .sort((left: any, right: any) => Number(right.id === data.WATCH_LATER) - Number(left.id === data.WATCH_LATER) || left.created - right.created)
        .map((one: any) => one.id)
}
/** What plays after the video open, by Aside's own `nextOf` and `upNextFor`, as the page asks them. */
function nextOfWatching(): string | null {
    const one = video(watching!.id)
    const { rows } = playlistRows.current()
    const list = watching!.list === null ? undefined : rows.find((l: any) => l.id === watching!.list)
    return aside.nextOf(one, list, aside.upNextFor(one))?.id ?? null
}

// --- Doing what the pages' buttons do ----------------------------------------------------------------------
async function act(step: any): Promise<unknown> {
    const nodeOf = (page: any) => tree(page)
    switch (step.op) {
        case 'view':
            return readers[step.page](step)
        case 'follow': {
            props.channel = step.channel
            pages.channel.render()
            await flush()
            first(nodeOf(pages.channel), ChannelRow).props.onFollowingChange(step.on)
            return undefined
        }
        case 'setFollowing':
            void data.setFollowing(step.channel, step.on)
            return undefined
        case 'prefs':
            data.setPrefs(step.patch)
            return undefined
        case 'keepWatched':
            data.keepWatched(step.id, step.patch)
            return undefined
        case 'menuLater': {
            props.menu = step.id
            menuPage.render()
            await flush()
            find(nodeOf(menuPage), DropdownMenuItem)[0].props.onClick()
            return undefined
        }
        case 'menuChannel': {
            props.menu = step.id
            menuPage.render()
            await flush()
            find(nodeOf(menuPage), DropdownMenuItem)[3].props.onClick()
            return undefined
        }
        case 'saveToggle': {
            props.save = step.video
            savePage.render()
            await flush()
            const order = sortedLists()
            const at = order.indexOf(step.list)
            find(nodeOf(savePage), CheckboxField)[at].props.onCheckedChange(step.on)
            return undefined
        }
        case 'saveNew': {
            props.save = step.video
            savePage.render()
            await flush()
            const add = first(nodeOf(savePage), QuickAdd).props
            const said = add.validate(step.name)
            if (said === null) void add.onAdd(step.name)
            return said
        }
        case 'playlistsNew': {
            const add = first(nodeOf(pages.playlists), QuickAdd).props
            const said = add.validate(step.name)
            if (said === null) void add.onAdd(step.name)
            return said
        }
        case 'playlistsDelete': {
            const card = all(nodeOf(pages.playlists)).find((one) => one.type === pagesPlaylistCard && one.props.list.name === step.name)
            const items = find((pagesPlaylistCard as any)({ list: card.props.list }).props.menu, DropdownMenuItem)
            items[1].props.onClick()
            return undefined
        }
        case 'playlist': {
            props.playlist = step.id
            pages.playlist.render()
            await flush()
            const node = nodeOf(pages.playlist)
            const header = first(node, PageHeader).props
            const buttons = find(header.actions, Button)
            const sortable = first(node, SortableList)
            const entry = (index: number) => sortable.props.renderItem(sortable.props.items[index], { index }).props.extra.props.children
            switch (step.do) {
                case 'rename': {
                    const said = header.title.props.validate(step.name)
                    if (said === null) void header.title.props.onSave(step.name)
                    return said
                }
                case 'describe':
                    void header.description.props.onSave(step.text)
                    return undefined
                case 'up':
                    entry(step.index)[0].props.onClick()
                    return undefined
                case 'down':
                    entry(step.index)[1].props.onClick()
                    return undefined
                case 'remove':
                    entry(step.index)[2].props.onRemove()
                    return undefined
                case 'drag':
                    sortable.props.onMove(step.from, step.to)
                    return undefined
                case 'play':
                    buttons[0].props.onClick()
                    return undefined
                case 'shuffle':
                    buttons[1].props.onClick()
                    return undefined
                case 'delete':
                    first(node, ConfirmDialog).props.confirm.onConfirm()
                    return undefined
            }
            throw new Error(`no playlist step ${String(step.do)}`)
        }
        case 'historyRemove': {
            const node = nodeOf(pages.history)
            const card = all(node).find((one) => one.type === parts.VideoCard && one.props.video.id === step.id)
            card.props.extra.props.onRemove()
            return undefined
        }
        case 'historyClear': {
            props.clearing = step.clearing === true
            pages.history.render()
            await flush()
            first(nodeOf(pages.history), ConfirmDialog).props.confirm.onConfirm()
            props.clearing = false
            return undefined
        }
        case 'historyKeep':
            first(nodeOf(pages.history), SwitchField).props.onCheckedChange(step.on)
            return undefined
        case 'likedRemove': {
            const card = all(nodeOf(pages.liked)).find((one) => one.type === parts.VideoCard && one.props.video.id === step.id)
            card.props.extra.props.onRemove()
            return undefined
        }
        case 'likedPlay':
            first(nodeOf(pages.liked), PageHeader).props.actions.props.onClick()
            return undefined
        case 'downloadsRemove': {
            const table = first(nodeOf(pages.downloads), DataTable).props
            const row = table.rows.find((one: any) => one.video.id === step.id)
            table.rowActions(row).find((one: any) => one.id === 'remove').onSelect()
            return undefined
        }
        case 'channelPlay': {
            props.channel = step.id
            pages.channel.render()
            await flush()
            await chooseIn(pages.channel, Segmented, step.sort)
            const toolbar = first(nodeOf(pages.channel), Toolbar)
            toolbar.props.end.props.onClick()
            return undefined
        }
        case 'comment': {
            props.comments = step.video
            pages.comments.render()
            await flush()
            const node = nodeOf(pages.comments)
            const thread = first(node, CommentThread).props
            const target = (id: string | undefined) => {
                const look = (list: any[]): any => {
                    for (const one of list) {
                        if (one.id === id || (id === undefined && one.body === step.body)) return one
                        const found = look(one.replies)
                        if (found !== undefined) return found
                    }
                    return undefined
                }
                return look(thread.comments) ?? { id }
            }
            switch (step.do) {
                case 'add':
                    first(node, CommentComposer).props.onSubmit(step.text)
                    break
                case 'reply':
                    thread.onReply(target(step.id), step.text)
                    break
                case 'like':
                    thread.onLike(target(step.id))
                    break
                case 'dislike':
                    thread.onDislike(target(step.id))
                    break
                case 'react':
                    thread.onReaction(target(step.id), step.emoji, step.name)
                    break
                case 'menu':
                    thread.menu(target(step.id)).props.onClick()
                    break
                case 'undo':
                    undo.pop()?.()
                    break
                default:
                    throw new Error(`no comment step ${String(step.do)}`)
            }
            return undefined
        }
        case 'palette': {
            props.route = step.route ?? props.route
            pages.site.render()
            await flush()
            const node = tree(pages.site).props
            const command = node.palette.flatMap((one: any) => one.commands ?? []).find((one: any) => one.id === step.id)
            command.onSelect()
            return undefined
        }
        case 'paletteSearch': {
            const node = tree(pages.site).props
            const groups = node.paletteSearch.search(step.text)
            if (step.open !== undefined) {
                const group = groups.find((one: any) => one.kind === step.open[0])
                node.paletteSearch.onOpen({ id: step.open[1] }, group)
            }
            return groups.map((group: any) => ({
                kind: group.kind,
                title: group.title,
                total: group.total,
                results: group.results.map((one: any) => ({ id: one.id, title: one.title, detail: one.detail, meta: one.meta })),
            }))
        }
        case 'submit': {
            const node = tree(pages.site).props
            node.top.props.search.props.onChange(step.text)
            await flush()
            tree(pages.site).props.top.props.search.props.inputProps.onKeyDown({ key: 'Enter' })
            return undefined
        }
        case 'settings': {
            const found = settingsById()
            const control = (id: string) => found[id].control.props
            switch (step.do) {
                case 'reset':
                    found.onReset(step.section)
                    break
                case 'toggle':
                    control(step.id).onChange(step.on)
                    break
                case 'select':
                    control(step.id).onChange({ target: { value: step.value } })
                    break
                case 'segment':
                    control(step.id).onValueChange(step.value)
                    break
                case 'clear':
                    control('clear-history').onClick()
                    break
                default:
                    throw new Error(`no settings step ${String(step.do)}`)
            }
            return undefined
        }
        case 'open':
            openWatch(step.id, step.list ?? null, step.start ?? null)
            return undefined
        case 'watch': {
            attachHandle()
            const view = tree(watching!.page).props
            const play = view.player.props
            const row = first(view.children, ChannelRow).props
            const actions = row.actions.props
            const download = actions.actions.find((one: any) => one.id === 'download')
            const state = watching!.player
            switch (step.do) {
                case 'play':
                    state.playing = true
                    if (step.time !== undefined) state.time = step.time
                    play.onPlayingChange(true)
                    break
                case 'pause':
                    state.playing = false
                    state.time = step.time
                    play.onPlayingChange(false)
                    break
                case 'time':
                    state.time = step.time
                    play.onTime(step.time)
                    break
                case 'ended':
                    state.playing = false
                    state.time = video(watching!.id).duration
                    play.onPlayingChange(false)
                    play.onEnded()
                    break
                case 'quality':
                    play.onQualityChange(step.quality)
                    break
                case 'rate':
                    play.onRateChange(step.rate)
                    break
                case 'rating':
                    actions.onRatingChange(step.rating)
                    break
                case 'follow':
                    row.onFollowingChange(step.on)
                    break
                case 'download':
                    download.render.props.onDownload()
                    break
                case 'cancel':
                    download.render.props.onCancel()
                    break
                case 'remove':
                    download.render.props.onRemove()
                    break
                case 'downloadEntry':
                    download.onSelect()
                    break
                case 'theater':
                    play.actions.props.onClick()
                    break
                case 'stats':
                    actions.menu.props.onCheckedChange(true)
                    break
                case 'playNext':
                    play.cover.props.onPlay()
                    break
                case 'stay':
                    play.cover.props.onCancel()
                    break
                default:
                    throw new Error(`no watch step ${String(step.do)}`)
            }
            return undefined
        }
        default:
            throw new Error(`no step ${String(step.op)}`)
    }
}

// --- The steps ---------------------------------------------------------------------------------------------
const startedRandom = Math.random
Math.random = seeded(inputs.random)
await advance(inputs.now + 900)

/** A collection's rows as the backend keeps them, or as it first answered while nothing is written yet. */
const rows = (name: string) => JSON.parse(kept.get(`tidevann-demo:video:${name}`) ?? 'null') ?? seeds[name] ?? null
function look(answer: unknown): Record<string, unknown> {
    const out = {
        follows: rows('follows'),
        ratings: rows('ratings'),
        history: rows('history'),
        playlists: rows('playlists'),
        comments: rows('comments'),
        downloads: rows('downloads'),
        prefsRows: rows('prefs') ?? [data.DEFAULT_PREFS],
        prefs: prefsView.current(),
        prefsReady: data.prefsReady(),
        toasts,
        went,
        answer: answer === undefined ? null : answer,
    }
    toasts = []
    went = []
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

const steps: unknown[] = [changed(look(undefined))]
for (const step of inputs.steps) {
    await advance(inputs.now + step.at)
    const answer = await act(step)
    await advance(inputs.now + step.at + (step.wait ?? 900))
    steps.push(changed(look(answer)))
}
Math.random = startedRandom

process.stdout.write(JSON.stringify({ library, words, sound, sources, steps }))
