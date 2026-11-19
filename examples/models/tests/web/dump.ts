/**
 * What the web's models modules give for the inputs in `inputs.json`, as JSON on stdout: run by
 * `tests/web.rs` with bun from the repository's root, the zone UTC, against `.parity/sokkel-ui` at
 * the commit `parity/models.json` names. The clock and the timers are held here and `Math.random`
 * is a seeded run while the steps play; between two timers every microtask runs, as a browser
 * runs them. The engines start as the web's `App` starts them, and each step calls what a page
 * calls (`engine/*`, `actions.tsx`, the backend's collections), the collections, the answers being
 * written and what the promises settled with read after it.
 */

const ROOT = new URL('../../../../.parity/sokkel-ui/examples/models/web/src/', import.meta.url).pathname
const MODULES = `${ROOT}../node_modules/`
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
    timers.push({ at: now + Math.max(0, ms ?? 0), seq, fn })
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
const realImmediate = setImmediate
Date.now = () => now
const RealDate = Date
globalThis.Date = class extends RealDate {
    constructor(...args: any[]) {
        if (args.length === 0) super(now)
        else super(...(args as []))
    }
    static now() {
        return now
    }
} as DateConstructor

const kept = new Map<string, string>()
let hash = '#/'
let went: string[] = []
const location = {
    href: 'http://localhost:5173/#/',
    get hash() {
        return hash
    },
    set hash(next: string) {
        hash = next.startsWith('#') ? next : `#${next}`
        went.push(next)
    },
    replace: (next: string) => went.push(`replace ${next}`),
    reload: () => went.push('reload'),
}
Object.assign(globalThis, {
    setTimeout: setTimer,
    clearTimeout: clearTimer,
    setInterval: setEvery,
    clearInterval: clearTimer,
    window: {
        location,
        innerWidth: 1440,
        setTimeout: setTimer,
        clearTimeout: clearTimer,
        addEventListener: () => {},
        removeEventListener: () => {},
        matchMedia: () => ({ matches: false, addEventListener() {}, removeEventListener() {} }),
    },
    document: {
        head: null,
        body: { style: {}, appendChild() {}, append() {} },
        documentElement: { style: { setProperty() {} }, dataset: {}, classList: { add() {}, remove() {}, toggle() {} } },
        createElement: () => ({ style: {} }),
        querySelector: () => null,
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
// A change refused while nobody waits for it, as the engines leave some, is not the run's end.
process.on('unhandledRejection', () => {})

/** Every microtask queued, and those they queue, run. */
const settle = () => new Promise<void>((done) => realImmediate(done))
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
    now = Math.max(now, to)
    await settle()
}

const { configureDemo } = await import(`${MODULES}tidevann-shared/backend/index.ts`)
configureDemo({ app: 'models' })

// The engines load as the web's App loads them: the chat (with the stream and the runner), then the pulls.
const chat = await import(`${ROOT}engine/chat.ts`)
const downloads = await import(`${ROOT}engine/downloads.ts`)
const runner = await import(`${ROOT}engine/runner.ts`)
const stream = await import(`${ROOT}engine/stream.ts`)
const generate = await import(`${ROOT}engine/generate.ts`)
const backend = await import(`${ROOT}data/backend.ts`)
const model = await import(`${ROOT}data/model.ts`)
const catalog = await import(`${ROOT}data/catalog.ts`)
const seed = await import(`${ROOT}data/seed.ts`)
const disk = await import(`${ROOT}data/disk.ts`)
const format = await import(`${ROOT}data/format.ts`)
const { CARRIED } = await import(`${ROOT}themes.ts`)
const actions = await import(`${ROOT}actions.tsx`)
const pullParts = await import(`${ROOT}pages/pull.tsx`)
const history = await import(`${MODULES}@winterop-com/sokkel-ui/src/components/ConversationHistory.tsx`)

const json = (value: unknown) => JSON.parse(JSON.stringify(value === undefined ? null : value))
const names = ['library', 'conversations', 'downloads', 'runner', 'loaded', 'events', 'comparisons', 'prefs', 'views']
const collections: Record<string, any> = {
    library: backend.models,
    conversations: backend.conversations,
    downloads: backend.downloads,
    runner: backend.runner,
    loaded: backend.loaded,
    events: backend.events,
    comparisons: backend.comparisons,
    prefs: backend.prefs,
    views: backend.views,
}
// Each collection's rows as it starts, read once; a change keeps them in storage after.
const first: Record<string, unknown[]> = {}
for (const name of names) void collections[name].list().then((rows: unknown[]) => (first[name] = rows))

chat.settleCutOff()
runner.startRunnerEngine()
downloads.startDownloadEngine()
let pulled: unknown[] = []
downloads.onPulled((one: unknown) => pulled.push(json(one)))
await advance(inputs.now + 160)

const rowsOf = (name: string): any[] => JSON.parse(kept.get(`tidevann-demo:models:${name}`) ?? 'null') ?? first[name]

// --- What the modules give --------------------------------------------------------------------------
const library: any[] = first.library as any[]
const sampleDownloads = inputs.downloads.map((one: any) => ({ ...one }))
const core: Record<string, any> = {
    model: {
        formats: model.FORMATS,
        formatNotes: model.FORMAT_NOTES,
        sources: model.SOURCES,
        sourceIds: model.SOURCE_IDS,
        capabilities: model.CAPABILITIES,
        sampling: model.DEFAULT_SAMPLING,
        prefs: model.DEFAULT_PREFS,
    },
    themes: CARRIED,
    catalog: catalog.CATALOG,
    quants: Object.entries(catalog.QUANTS),
    modelIds: inputs.modelIds.map(([name, quant]: [string, string]) => catalog.modelId(name, quant)),
    files: catalog.CATALOG.map((entry: any) => entry.variants.map((variant: any) => catalog.filesOf(entry, variant))),
    entries: inputs.entryIds.map((id: string) => catalog.entryOf(id)?.id ?? null),
    usual: catalog.CATALOG.map((entry: any) => pullParts.usualVariant(entry)),
    searches: [] as unknown[],
}
for (const query of inputs.searches) {
    core.searches.push(
        await (async () => {
            const asked = Date.now()
            const before = seq
            const answer = catalog.searchCatalog(query)
            let found: any[] | undefined
            void answer.then((rows: any[]) => (found = rows))
            const timer = timers.find((one) => one.seq > before)
            const delay = timer === undefined ? null : timer.at - asked
            if (timer !== undefined) {
                timers = timers.filter((one) => one !== timer)
                timer.fn()
            }
            await settle()
            return { ids: (found ?? []).map((entry: any) => entry.id), delay }
        })(),
    )
}
Object.assign(core, {
    seed: Object.fromEntries(names.map((name) => [name, first[name]])),
    memory: library.map((one: any) => seed.memoryOf(one)),
    format: {
        bytes: inputs.numbers.map((value: number) => format.bytes(value)),
        speed: inputs.numbers.map((value: number) => format.speed(value)),
        context: inputs.numbers.map((value: number) => format.context(value)),
        params: inputs.params.map((value: number) => format.params(value)),
        counted: inputs.counts.map((value: number) => [format.counted(value, 'model'), format.counted(value, 'day', 'days')]),
        remaining: inputs.seconds.map((value: number | null) => format.remaining(value ?? Infinity)),
        day: inputs.times.map((value: number) => format.day(value)),
        clock: inputs.times.map((value: number) => format.clock(value)),
        clockSeconds: inputs.times.map((value: number) => format.clockSeconds(value)),
        uptime: inputs.durations.map((value: number) => format.uptime(value)),
    },
    disk: inputs.roots.map((root: string) => ({
        disk: disk.diskOf(root),
        usage: disk.usageOf(root, library, sampleDownloads),
        path: disk.pathOf(root, library[0]),
        valid: disk.validRoot(root),
    })),
    generate: {
        tokens: inputs.texts.map((text: string) => generate.tokens(text)),
        pieces: inputs.texts.map((text: string) => generate.pieces(text)),
        titles: inputs.texts.map((text: string) => generate.titleOf(text)),
        rates: library.map((one: any) => [generate.rateOf(one), generate.prefillOf(one, 0), generate.prefillOf(one, 1234)]),
        plans: inputs.plans.map((ask: any) =>
            generate.plan({
                prompt: ask.prompt,
                model: library.find((one: any) => one.id === ask.model),
                settings: { ...model.DEFAULT_SAMPLING, ...ask.settings },
                attachments: ask.attachments,
                history: ask.history,
                attempt: ask.attempt,
            }),
        ),
    },
    history: inputs.historyTimes.map((time: number) => [
        history.conversationGroup({ id: 'x', title: 'x', time, pinned: false }, inputs.now),
        history.conversationGroup({ id: 'x', title: 'x', time, pinned: true }, inputs.now),
        history.conversationTime(time, inputs.now),
    ]),
    pulls: inputs.pullOf.map(([entry, quant]: [string, string]) => pullParts.pullOf(sampleDownloads, entry, quant)?.id ?? null),
})


// --- The pages, drawn on a stand-in for React's hooks ------------------------------------------------
const R = await import(new URL('react.ts', import.meta.url).pathname)
const React = await import(`${MODULES}react/index.js`)
;(React.default ?? React).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE.H = R.dispatcher
const pages = {
    library: await import(`${ROOT}pages/Library.tsx`),
    model: await import(`${ROOT}pages/ModelPage.tsx`),
    disk: await import(`${ROOT}pages/Disk.tsx`),
    discover: await import(`${ROOT}pages/Discover.tsx`),
    downloads: await import(`${ROOT}pages/Downloads.tsx`),
    runner: await import(`${ROOT}pages/Runner.tsx`),
    settings: await import(`${ROOT}pages/ChatSettings.tsx`),
    compare: await import(`${ROOT}pages/Compare.tsx`),
    chat: await import(`${ROOT}pages/Chat.tsx`),
    settingsPage: await import(`${ROOT}pages/Settings.tsx`),
}
const { ModelsApp } = await import(`${ROOT}ModelsApp.tsx`)

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
// The library's parts are built and their names shortened, so each is known by the very function
// the pages import, from the module the pages' own imports resolve to.
const PARTS: [string, string][] = [
    ['FilterBar', 'FilterBar'], ['SearchField', 'SearchField'], ['DataTable', 'DataTable'], ['PageHeader', 'PageHeader'],
    ['DescriptionList', 'DescriptionList'], ['Section', 'Section'], ['Stat', 'Stat'], ['DownloadButton', 'DownloadButton'],
    ['StatusChip', 'StatusChip'], ['Segmented', 'Segmented'], ['CheckList', 'CheckList'], ['LogView', 'LogView'],
    ['SidePanel', 'SidePanel'], ['NumberField', 'NumberField'], ['SortableGroup', 'Sortable'], ['Composer', 'Composer'],
    ['IconButton', 'IconButton'], ['Columns', 'Layout'], ['AnswerColumn', 'AnswerColumn'], ['AssistantReply', 'AssistantReply'],
    ['Shell', 'Shell'], ['Rail', 'Rail'], ['TopBar', 'TopBar'], ['ServerMenu', 'ServerMenu'], ['StatusLine', 'StatusLine'],
    ['StatusItem', 'StatusLine'], ['RelativeTime', 'RelativeTime'], ['ConversationHistory', 'ConversationHistory'], ['ConsequenceList', 'ConsequenceList'],
    ['SettingsDialog', 'SettingsDialog'], ['Button', 'primitives/button'],
    ['Progress', 'primitives/progress'], ['ProgressLabel', 'primitives/progress'], ['Select', 'primitives/select'],
    ['SelectGroup', 'primitives/select'], ['SelectLabel', 'primitives/select'], ['SelectItem', 'primitives/select'],
]
const known = new Map<unknown, string>()
for (const [name, path] of PARTS) {
    const module = await import(Bun.resolveSync(`@winterop-com/sokkel-ui/${path}`, ROOT))
    known.set(module[name], name)
}
for (const name of ['Chart', 'ChartFrame']) {
    const module = await import(Bun.resolveSync(`@winterop-com/sokkel-ui-charts/${name}`, ROOT))
    known.set(module[name], name)
}
const nameOf = (type: any): string | undefined =>
    typeof type === 'string' ? undefined : (known.get(type) ?? type?.displayName ?? type?.name ?? type?.render?.name ?? type?.type?.name)
const named = (tree: any, name: string) => all(tree).filter((node) => nameOf(node.type) === name)
const one = (tree: any, name: string) => named(tree, name)[0]
/** What an element says: its text, a chip by its status and label, a badge's format. */
function say(node: any): string {
    if (node === null || node === undefined || typeof node === 'boolean') return ''
    if (typeof node === 'string' || typeof node === 'number') return String(node)
    if (Array.isArray(node)) return node.map(say).join('')
    if (typeof node !== 'object') return ''
    const name = nameOf(node.type)
    if (name === 'StatusChip') return `${String(node.props.status)}|${node.props.label ?? ''}`
    if (name === 'FormatBadge') return node.props.format
    if (name === 'MemoryChip' || name === 'StateChip') return say(node.type(node.props))
    if (name === 'RelativeTime') return ''
    return say(node.props?.children)
}
const statsOf = (tree: any) =>
    named(tree, 'Stat').map((stat: any) => ({
        label: stat.props.label,
        value: typeof stat.props.value === 'object' ? say(stat.props.value) : String(stat.props.value),
        note: stat.props.note ?? null,
        good: stat.props.good ?? null,
    }))
const headerOf = (tree: any) => {
    const p = one(tree, 'PageHeader')?.props
    return p === undefined ? null : { title: p.title ?? null, code: p.code ?? null, count: p.count ?? null, description: p.description ?? null }
}
const itemsOf = (node: any) => node.props.items.map((item: any) => [item.term, say(item.value)])
const sub = (render: () => any) => R.mount(render).current()

function appData() {
    const library = rowsOf('library')
    return {
        models: library,
        conversations: rowsOf('conversations'),
        downloads: rowsOf('downloads'),
        runner: rowsOf('runner')[0],
        loaded: rowsOf('loaded'),
        events: rowsOf('events'),
        comparisons: rowsOf('comparisons'),
        views: rowsOf('views'),
        prefs: rowsOf('prefs')[0],
        setPrefs: () => Promise.resolve(),
        loading: false,
        modelOf: (id: string) => library.find((model: any) => model.id === id),
    }
}

function libraryLook(): unknown {
    const page = R.mount(() => pages.library.LibraryPage())
    const app = (globalThis as any).__app
    const fields = one(page.current(), 'FilterBar').props.fields.map((field: any) => ({
        id: field.id,
        label: field.label,
        type: field.type,
        options: (field.options ?? []).map((option: any) => [option.value, option.label ?? option.value]),
        unit: field.unit ?? null,
    }))
    const shown = inputs.library.map((ask: any) => {
        one(page.current(), 'FilterBar').props.onFiltersChange(ask.filters)
        one(page.current(), 'SearchField').props.onChange(ask.query)
        R.renderStale()
        const table = one(page.current(), 'DataTable').props
        return {
            ids: table.rows.map((model: any) => model.id),
            total: say(table.columns.find((column: any) => column.id === 'size').aggregate(table.rows)),
        }
    })
    const table = one(page.current(), 'DataTable').props
    return {
        count: headerOf(page.current()).count,
        fields,
        shown,
        cells: app.models.map((model: any) => table.columns.map((column: any) => say(column.cell(model)))),
        actions: app.models.map((model: any) =>
            pages.library.modelActions(model, app.loaded, () => {}).map((action: any) => [action.id, action.label ?? '']),
        ),
    }
}

function modelLook(id: string): unknown {
    const tree = sub(() => pages.model.ModelPage({ id }))
    const lists = named(tree, 'DescriptionList')
    const files = named(tree, 'DataTable')[0]?.props
    if (files === undefined) return null
    return {
        header: headerOf(tree),
        details: itemsOf(lists[1]),
        inUse: itemsOf(lists[0]),
        filesMeta: named(tree, 'Section').find((section: any) => section.props.title === 'Files').props.meta,
        files: files.rows.map((file: any) => files.columns.map((column: any) => say(column.cell(file)))),
        load: named(tree, 'Button').some((button: any) => say(button.props.children).trim() === 'Load') ? 'load' : 'unload',
    }
}

function diskLook(): unknown {
    const tree = sub(() => pages.disk.DiskPage())
    const charts = named(tree, 'ChartFrame')
    const copies = named(tree, 'DataTable')[0].props
    return {
        description: headerOf(tree).description,
        stats: statsOf(tree),
        subtitle: charts[0].props.subtitle,
        whole: charts[0].props.children({}).props.rows,
        rows: charts[3].props.children({}).props.rows,
        copies: copies.rows.map((row: any) => ({
            name: row.name,
            kept: row.kept.map((model: any) => model.id),
            said: say(copies.columns[1].cell(row)),
            reclaim: say(copies.columns[2].cell(row)),
        })),
        copiesMeta: named(tree, 'Section')[0].props.meta ?? null,
        path: say(all(tree).find((node: any) => node.type === 'p')),
    }
}

function discoverLook(): unknown {
    const page = R.mount(() => pages.discover.DiscoverPage())
    const tree = page.current()
    const table = one(tree, 'DataTable').props
    const shown = catalog.CATALOG.map((entry: any) =>
        table.columns.filter((column: any) => column.id !== 'pull').map((column: any) => say(column.cell(entry))),
    )
    const sheets = inputs.sheets.map((id: string) => {
        const entry = catalog.entryOf(id)
        one(page.current(), 'DataTable').props.onOpen(entry)
        R.renderStale()
        const now = page.current()
        const variants = sub(() => one(now, 'Variants').type({ entry }))
        const rows = one(variants, 'DataTable').props
        return {
            about: all(now).filter((node: any) => node.type === 'p').map(say),
            memory: rows.rows.map((variant: any) => say(rows.columns.find((column: any) => column.id === 'fits').cell(variant))),
        }
    })
    const buttons = catalog.CATALOG.flatMap((entry: any) =>
        entry.variants.map((variant: any) => {
            const button = one(sub(() => pullParts.PullButton({ entry, variant })), 'DownloadButton')?.props ?? {}
            const words = button.words === undefined ? null : { ...button.words, share: button.words.share === undefined ? undefined : button.words.share(0) }
            return {
                state: button.state,
                keptDetail: button.keptDetail ?? null,
                error: button.error ?? null,
                value: button.value ?? null,
                words,
            }
        }),
    )
    const header = headerOf(tree)
    return {
        count: header.count,
        description: header.description,
        downloads: say(named(tree, 'Button')[0].props.children).trim(),
        shown,
        sheets,
        buttons,
    }
}

function downloadsLook(): unknown {
    const tree = sub(() => pages.downloads.DownloadsPage())
    const table = one(tree, 'DataTable').props
    const header = headerOf(tree)
    const column = (id: string) => table.columns.find((one: any) => one.id === id)
    return {
        count: header.count,
        description: header.description,
        canClear: !named(tree, 'Button')[0].props.disabled,
        stats: statsOf(tree),
        rows: table.rows.map((pull: any) => {
            const progress = column('progress').cell(pull)
            const failed = nameOf(progress.type) !== 'Progress'
            return {
                id: pull.id,
                group: table.groupBy(pull),
                name: say(column('name').cell(pull)),
                from: say(column('source').cell(pull)),
                state: say(column('state').cell(pull)),
                progress: failed ? say(progress) : say(one(progress, 'ProgressLabel')),
                value: failed ? null : progress.props.value,
                speed: say(column('speed').cell(pull)),
                left: say(column('left').cell(pull)),
                actions: table.rowActions(pull).map((action: any) => action.id),
            }
        }),
    }
}

function runnerLook(source: string, query: string): unknown {
    const page = R.mount(() => pages.runner.RunnerPage())
    one(page.current(), 'Segmented').props.onValueChange(source)
    one(page.current(), 'SearchField').props.onChange(query)
    R.renderStale()
    const tree = page.current()
    const table = one(tree, 'DataTable').props
    const progress = one(tree, 'Progress')
    const button = named(one(tree, 'PageHeader').props.actions, 'Button')[0] ?? one(tree, 'PageHeader').props.actions
    return {
        code: headerOf(tree).code,
        button: [say(button.props.children).trim(), button.props.disabled !== true],
        stats: statsOf(tree),
        memoryMeta: named(tree, 'Section')[0].props.meta,
        memoryLabel: say(one(progress, 'ProgressLabel')),
        memoryValue: progress.props.value,
        loaded: table.rows.map((row: any) => table.columns.map((column: any) => say(column.cell(row)))),
        empty: table.words.empty,
        checks: one(tree, 'CheckList').props.checks.map((check: any) => ({ ...check, hint: check.hint ?? null })),
        lines: one(tree, 'LogView').props.lines,
    }
}

function settingsLook(conversation: any): unknown {
    const tree = sub(() => pages.settings.ChatSettingsPanel({ conversation }))
    const content = one(tree, 'SidePanel').props.tabs[0].content
    const select = sub(() => one(content, 'ModelSelect').type(one(content, 'ModelSelect').props))
    const facts = one(content, 'ModelFacts')
    const prompt = sub(() => one(content, 'SystemPrompt').type(one(content, 'SystemPrompt').props))
    const back = named(content, 'Button').find((button: any) => say(button.props.children).includes('defaults'))
    return {
        options: named(select, 'SelectGroup').map((group: any) => [
            say(one(group, 'SelectLabel').props.children),
            named(group, 'SelectItem').map((item: any) => [item.props.value, say(item.props.children)]),
        ]),
        facts: facts === undefined ? null : itemsOf(one(sub(() => facts.type(facts.props)), 'DescriptionList')),
        maxHint: one(content, 'NumberField').props.hint ?? null,
        back: back === undefined ? null : say(back.props.children).trim(),
        presets: named(prompt, 'Button').map((button: any) => say(button.props.children)),
    }
}

function compareLook(id: string | undefined): unknown {
    const tree = sub(() => pages.compare.ComparePage({ id }))
    const group = one(tree, 'SortableGroup')
    const composer = one(tree, 'Composer').props
    const asked = named(tree, 'Section').find((section: any) => section.props.title === 'Asked')
    const answers = named(tree, 'Answer').map((answer: any) => {
        const column = one(sub(() => answer.type(answer.props)), 'AnswerColumn').props
        return {
            model: column.model,
            detail: column.detail,
            state: column.state,
            reasoning: column.reasoning ?? null,
            content: column.content,
            usage: column.usage ?? null,
            problem: column.problem ?? null,
            removable: column.onRemove !== undefined,
        }
    })
    const select = one(tree, 'Select')
    return {
        count: headerOf(tree).count,
        earlier: select === undefined ? [] : select.props.items.map((item: any) => [item.value, item.label]),
        keys: group.props.ids.map((key: string) => [key, group.props.label(key)]),
        canAdd: named(tree, 'Button').some((button: any) => say(button.props.children).trim() === 'Add a model'),
        canRemove: named(tree, 'IconButton').some((button: any) => String(button.props.label).startsWith('Take model')),
        placeholder: composer.words.placeholder,
        disabled: composer.disabled === true,
        running: composer.busy,
        asked: asked?.props.description ?? null,
        columns: named(tree, 'Columns').at(-1).props.count,
        answers,
    }
}

function chatLook(id: string): unknown {
    const tree = sub(() => pages.chat.ChatPage({ id }))
    const replies = named(tree, 'Reply').map((reply: any) => {
        const said = one(sub(() => reply.type(reply.props)), 'AssistantReply').props
        return {
            name: say(said.name),
            text: said.content,
            reasoning: said.reasoning ?? null,
            state: said.state,
            usage: said.children === false || said.children === undefined || said.children === null ? null : said.children.props,
            problem: said.problem?.detail ?? null,
            thinking: said.words.thinking,
            versions: said.versions === undefined ? null : [said.versions.at, said.versions.of],
        }
    })
    return { replies, placeholder: one(tree, 'Composer')?.props.words.placeholder ?? null }
}

function appLook(route: string[]): unknown {
    const frame = {
        route,
        palette: [],
        themes: CARRIED,
        appearance: { settings: {}, setSettings: () => {} },
        dialogs: null,
    }
    const tree = sub(() => ModelsApp({ frame }))
    const shell = one(tree, 'Shell').props
    const rail = one(shell.rail, 'Rail').props
    const top = one(shell.top, 'TopBar').props
    const server = one(top.actions, 'ServerMenu').props
    const foot = one(shell.foot, 'StatusLine').props
    const actions = shell.palette.find((group: any) => group.title === 'Actions').commands
    const conversations = shell.palette.find((group: any) => group.title === 'Conversations').commands
    return {
        rail: rail.groups.map((group: any) => [
            group.id,
            group.title,
            group.entries.map((entry: any) => ({
                id: entry.id,
                label: entry.label,
                href: entry.href,
                active: entry.active,
                count: entry.count ?? null,
                keywords: entry.keywords ?? [],
            })),
        ]),
        trail: top.trail.map((crumb: any) => [crumb.label, crumb.href ?? null, crumb.code === true]),
        server: { state: server.state, facts: server.facts, checking: server.words.checking },
        status: {
            live: foot.live,
            warn: foot.tone === 'warn',
            note: foot.note,
            items: named(foot.children, 'StatusItem').map((item: any) => [item.props.label, say(item.props.children)]),
        },
        actions: actions.map((command: any) => [command.id, command.label, command.hint ?? null]),
        conversations: conversations.map((command: any) => [command.id, command.label, command.hint]),
    }
}

function historyLook(): unknown {
    const app = (globalThis as any).__app
    const current = app.conversations[0]?.id
    const tree = sub(() => pages.chat.ChatPage({ id: current }))
    const history = one(tree, 'History')
    if (history === undefined) return null
    const inner = one(sub(() => history.type(history.props)), 'ConversationHistory')
    const table = one(sub(() => inner.type(inner.props)), 'DataTable').props
    return table.rows.map((entry: any) => [
        table.groupBy(entry),
        table.groupLabel(table.groupBy(entry)),
        entry.id,
        say(table.columns[1].cell(entry)),
        table.rowActions(entry).filter((action: any) => !action.separator).map((action: any) => action.label),
    ])
}

function deleteLook(): unknown {
    const app = (globalThis as any).__app
    return inputs.deletes.map((ids: string[]) => {
        const mounted = R.mount(() => actions.useDeleteModels())
        mounted.current().ask(ids.map((id: string) => app.models.find((model: any) => model.id === id)).filter(Boolean))
        R.renderStale()
        const dialog = mounted.current().dialog
        const list = one(dialog, 'ConsequenceList')
        return {
            question: dialog.props.question,
            detail: dialog.props.detail,
            items: list.props.items.map((item: any) => [item.label, item.count ?? null, item.detail ?? null]),
        }
    })
}

function settingsPageLook(): unknown {
    const app = (globalThis as any).__app
    const saved: unknown[] = []
    app.setPrefs = (patch: unknown) => {
        saved.push(patch)
        return Promise.resolve()
    }
    const frame = { route: ['settings'], palette: [], themes: CARRIED, appearance: { settings: {}, setSettings: () => {} }, dialogs: null }
    const tree = sub(() => pages.settingsPage.SettingsPage({ frame, open: true, onClose: () => {} }))
    const dialog = one(tree, 'SettingsDialog').props
    const sections = dialog.groups
        .flatMap((group: any) => group.sections)
        .filter((section: any) => section.id !== 'appearance')
        .map((section: any) => ({
            id: section.id,
            label: section.label,
            settings: section.settings.map((setting: any) => ({ id: setting.id, title: setting.title, description: setting.description })),
        }))
    for (const section of ['library', 'models', 'downloads', 'runner', 'demo', 'nothing']) dialog.onReset(section)
    return { sections, resets: saved }
}

function pagesLook(): unknown {
    ;(globalThis as any).__app = appData()
    const app = (globalThis as any).__app
    return {
        library: libraryLook(),
        models: inputs.modelPages.map((id: string) => modelLook(id)),
        disk: diskLook(),
        discover: discoverLook(),
        downloads: downloadsLook(),
        runner: inputs.logViews.map(([source, query]: [string, string]) => runnerLook(source, query)),
        settings: app.conversations.map((conversation: any) => settingsLook(conversation)),
        compare: [undefined, 'new', ...app.comparisons.map((one: any) => one.id)].map((id) => compareLook(id)),
        chat: app.conversations.map((conversation: any) => chatLook(conversation.id)),
        app: inputs.routes.map((route: string[]) => appLook(route)),
        history: historyLook(),
        deletes: deleteLook(),
        settingsPage: settingsPageLook(),
    }
}

// --- The steps --------------------------------------------------------------------------------------
const settled: unknown[] = []
const pageLooks: unknown[] = []
const follow = (what: string, answer: Promise<unknown> | void) => {
    if (answer === undefined) return
    answer.then(
        (value: any) => settled.push({ what, ok: value?.id ?? null }),
        (error: unknown) => settled.push({ what, error: error instanceof Error ? error.message : String(error) }),
    )
}
const conversationOf = (ref: string | number) => (typeof ref === 'number' ? rowsOf('conversations')[ref].id : ref)
const comparisonOf = (ref: string | number) => (typeof ref === 'number' ? rowsOf('comparisons')[ref].id : ref)
const pullIdOf = (entry: string, quant: string) => rowsOf('downloads').find((one: any) => one.entry === entry && one.quant === quant)?.id
const modelRow = (id: string) => rowsOf('library').find((one: any) => one.id === id)
const messageOf = (conversation: string, index: number) =>
    rowsOf('conversations').find((one: any) => one.id === conversation).messages[index].id

function act(step: any): void {
    switch (step.op) {
        case 'pull': {
            const entry = catalog.entryOf(step.entry)
            return follow(step.op, downloads.pull(entry, entry.variants.find((one: any) => one.quant === step.quant)))
        }
        case 'pause':
            return follow(step.op, downloads.pausePull(pullIdOf(step.entry, step.quant)))
        case 'resume':
            return follow(step.op, downloads.resumePull(pullIdOf(step.entry, step.quant)))
        case 'cancel':
            return follow(step.op, downloads.cancelPull(pullIdOf(step.entry, step.quant)))
        case 'retry':
            return follow(step.op, downloads.retryPull(pullIdOf(step.entry, step.quant)))
        case 'clearEnded':
            return follow(step.op, downloads.clearEnded())
        case 'removePull':
            return follow(step.op, backend.downloads.remove(pullIdOf(step.entry, step.quant)))
        case 'prefs':
            return follow(step.op, backend.prefs.update('prefs', step.patch))
        case 'startRunner':
            return follow(step.op, runner.startRunner())
        case 'stopRunner':
            return follow(step.op, runner.stopRunner())
        case 'load':
            return follow(step.op, runner.ensureLoaded(modelRow(step.model)))
        case 'unload':
            return follow(step.op, runner.unloadModel(step.model, step.why))
        case 'chatWith':
            return actions.chatWith(modelRow(step.model))
        case 'send':
            return follow(step.op, chat.send(conversationOf(step.conversation), step.text, step.attachments ?? []))
        case 'regenerate': {
            const id = conversationOf(step.conversation)
            return follow(step.op, chat.regenerate(id, messageOf(id, step.message)))
        }
        case 'stop':
            return chat.stopConversation(conversationOf(step.conversation))
        case 'showVersion': {
            const id = conversationOf(step.conversation)
            return chat.showVersion(id, messageOf(id, step.message), step.version)
        }
        case 'feedback': {
            const id = conversationOf(step.conversation)
            return chat.giveFeedback(id, messageOf(id, step.message), step.feedback ?? undefined)
        }
        case 'react': {
            const id = conversationOf(step.conversation)
            return chat.react(id, messageOf(id, step.message), step.emoji, step.name)
        }
        case 'settings':
            return chat.setSettings(conversationOf(step.conversation), step.patch)
        case 'conversation':
            return follow(step.op, backend.conversations.update(conversationOf(step.conversation), step.patch))
        case 'deleteConversation': {
            const id = conversationOf(step.conversation)
            chat.stopConversation(id)
            return follow(step.op, backend.conversations.remove(id))
        }
        case 'compare':
            return follow(step.op, chat.compare(step.prompt, step.models, step.temperature, step.maxTokens))
        case 'rerun':
            return follow(step.op, chat.rerunColumn(comparisonOf(step.comparison), step.index))
        case 'stopColumn':
            return chat.stopColumn(comparisonOf(step.comparison), step.index)
        case 'stopComparison':
            return chat.stopComparison(comparisonOf(step.comparison))
        case 'removeColumn': {
            const id = comparisonOf(step.comparison)
            const shown = rowsOf('comparisons').find((one: any) => one.id === id)
            chat.stopColumn(id, step.index)
            return follow(step.op, backend.comparisons.update(id, { columns: shown.columns.filter((_: unknown, at: number) => at !== step.index) }))
        }
        case 'deleteComparison': {
            const id = comparisonOf(step.comparison)
            chat.stopComparison(id)
            return follow(step.op, backend.comparisons.remove(id))
        }
        case 'deleteModels': {
            const all = rowsOf('library')
            const gone = step.models.map((id: string) => all.find((one: any) => one.id === id))
            return follow(step.op, actions.deleteModels(gone, all, rowsOf('prefs')[0].defaultModel))
        }
        case 'createView':
            return follow(step.op, backend.views.create({ name: step.name, state: step.state }))
        case 'renameView':
            return follow(step.op, backend.views.update(step.view, { name: step.name }))
        case 'removeView':
            return follow(step.op, backend.views.remove(step.view))
        case 'look':
            return
        case 'pages':
            pageLooks.push(pagesLook())
            return
        default:
            throw new Error(`no step ${String(step.op)}`)
    }
}

let last: Record<string, string> = {}
function look(): Record<string, unknown> {
    const out: Record<string, unknown> = Object.fromEntries(names.map((name) => [name, rowsOf(name)]))
    out.live = Object.entries(stream.live.get())
    out.settled = settled.splice(0)
    out.went = went.splice(0)
    out.pulled = pulled.splice(0)
    out.pages = pageLooks.splice(0)
    const changed: Record<string, unknown> = {}
    for (const [key, value] of Object.entries(json(out))) {
        const written = JSON.stringify(value)
        if (written !== last[key]) changed[key] = value
        last[key] = written
    }
    return changed
}

let random = inputs.random >>> 0
Math.random = () => {
    random = (Math.imul(random, 1664525) + 1013904223) >>> 0
    return random / 4294967296
}
const steps: unknown[] = [look()]
for (const step of inputs.steps) {
    await advance(inputs.now + step.at)
    act(step)
    await advance(inputs.now + step.at + (step.wait ?? 0))
    steps.push(look())
}

process.stdout.write(JSON.stringify({ core: json(core), steps }))
