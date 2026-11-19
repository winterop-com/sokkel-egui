/**
 * What the web's dataviz modules give for the inputs in `inputs.json`, as JSON on stdout: run by
 * `tests/web.rs` with bun from the repository's root, the zone UTC, against `.parity/sokkel-ui` at
 * the commit `parity/dataviz.json` names. The clock and the timers are held here and
 * `Math.random` is a seeded run while the steps play.
 */

const ROOT = new URL('../../../../.parity/sokkel-ui/examples/dataviz/web/src/', import.meta.url).pathname
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
const RealDate = Date
// `new Date()` reads the held clock too.
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
    href: `${inputs.origin}${inputs.pathname}#/`,
    origin: inputs.origin,
    pathname: inputs.pathname,
    get hash() {
        return hash
    },
    set hash(next: string) {
        hash = next
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
    requestAnimationFrame: (fn: (at: number) => void) => setTimer(() => fn(now), 16),
    cancelAnimationFrame: clearTimer,
    window: {
        location,
        innerWidth: 1440,
        setTimeout: setTimer,
        clearTimeout: clearTimer,
        addEventListener: () => {},
        removeEventListener: () => {},
        matchMedia: () => ({ matches: false, addEventListener() {}, removeEventListener() {} }),
        history: { go: () => {} },
    },
    document: {
        createElement: () => ({ getContext: () => null, click() {}, remove() {} }),
        addEventListener: () => {},
        removeEventListener: () => {},
        body: { appendChild() {} },
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

const { configureDemo } = await import(`${MODULES}tidevann-shared/backend/index.ts`)
configureDemo({ app: 'dataviz' })

const model = await import(`${ROOT}model.ts`)
const seed = await import(`${ROOT}seed.ts`)
const parse = await import(`${ROOT}parse.ts`)
const shape = await import(`${ROOT}shape.ts`)
const share = await import(`${ROOT}share.ts`)
const views = await import(`${ROOT}views.ts`)
const { CARRIED } = await import(`${ROOT}themes.ts`)

/** A dataset made from a CSV text, as the import dialog makes one. */
function imported(id: string, text: string) {
    const result = parse.parseTable(text)
    const table = result.ok ? result.table : { names: text.split(/[;,]/).map((one: string) => one.trim()), rows: [] }
    const columns = parse.columnsOf(table)
    return { id, name: id, description: '', source: 'paste', columns, rows: parse.rowsOf(table, columns, id), updatedAt: '' }
}
const datasets = [...seed.seedDatasets(), ...Object.entries(inputs.extra).map(([id, text]) => imported(id, text as string))]
const byId = (id: string) => datasets.find((one: any) => one.id === id)
const BASE = { view: 'bar', aggregate: 'sum', bin: 'month', sort: 'natural', limit: 0, stack: false, horizontal: false, palette: 'default' }

/** A shaped view in brief: everything but the chart's rows, which are counted and sampled. */
function brief(shaped: any) {
    const { rows, regions, ...rest } = shaped
    return { ...rest, regions: [...regions.entries()], rowCount: rows.length, rowsHead: rows.slice(0, 3), rowsLast: rows.at(-1) ?? null }
}

const prefsOf = ([numbers, decimals]: [string, string]) => ({ ...model.DEFAULT_PREFERENCES, numbers, decimals })
const core = {
    model: {
        columnTypes: model.COLUMN_TYPES,
        aggregates: model.AGGREGATES,
        bins: model.BINS,
        sorts: model.SORTS,
        limits: model.LIMITS,
        palettes: model.PALETTES,
        themeTokens: model.THEME_TOKENS,
        defaultPrefs: model.DEFAULT_PREFERENCES,
        paletteOf: inputs.paletteOf.map(([id, chosen]: [string, string]) =>
            model.paletteOf(id, { ...model.DEFAULT_PREFERENCES, palette: chosen }) ?? null,
        ),
        orderBoards: inputs.boardOrders.map((order: string[] | null) =>
            model.orderBoards(seed.seedBoards(), order ?? undefined).map((board: any) => board.id),
        ),
        counted: inputs.counted.map(([count, noun, plural]: [number, string, string | null]) =>
            model.counted(count, noun, plural ?? undefined),
        ),
    },
    views: { views: views.VIEWS, labels: views.VIEWS.map((one: any) => views.viewLabel(one.value)) },
    themes: CARRIED,
    seed: { datasets: seed.seedDatasets(), charts: seed.seedCharts(), boards: seed.seedBoards(), regions: seed.REGION_NAMES },
    parse: {
        tables: inputs.parse.map((text: string) => {
            const result = parse.parseTable(text)
            if (!result.ok) return { error: result.error }
            const columns = parse.columnsOf(result.table)
            return { format: result.format, table: result.table, columns, rows: parse.rowsOf(result.table, columns, 'p') }
        }),
        delimited: inputs.parse.map((text: string) => parse.readDelimited(text)),
        names: inputs.names.map(parse.uniqueNames),
        numbers: inputs.numbers.map(parse.numberOf),
        days: inputs.days.map(parse.dayOf),
        regions: inputs.days.concat(['Bomi', ' grand kru ', 'Nowhere']).map(parse.isRegion),
        normalized: inputs.numbers.concat(inputs.days).flatMap((text: string) =>
            ['text', 'number', 'date', 'region'].map((type) => parse.normalize(text, type)),
        ),
        csv: inputs.csv.map(([names, rows]: [string[], unknown[][]]) => parse.toCsv(names, rows)),
        files: inputs.files.map(parse.fileName),
    },
    shape: {
        typed: datasets.map((one: any) => shape.typedRows(one).slice(0, 12)),
        binKeys: inputs.binKeys.map(([day, bin]: [string, string]) => shape.binKey(day, bin)),
        binLabels: inputs.binLabels.map(([key, bin, withYear]: [string, string, boolean]) => shape.binLabel(key, bin, withYear)),
        formats: inputs.prefs.map((pair: [string, string]) => inputs.formats.map(shape.formatter(prefsOf(pair)))),
        combos: inputs.combos.map(([id, spec]: [string, any]) => {
            const dataset = byId(id)
            const full = { ...BASE, ...spec }
            return {
                measure: shape.measureName(dataset, full),
                shaped: brief(shape.shape(dataset, full)),
                suggest: shape.suggest(dataset, full),
            }
        }),
        filtered: seed.seedCharts().flatMap((chart: any) =>
            inputs.filters.map((filters: any[]) => {
                const dataset = byId(chart.datasetId)
                const own = shape.filtersFor(dataset, filters)
                return { own, shaped: brief(shape.shape(dataset, chart.spec, own)) }
            }),
        ),
        fields: datasets.map((one: any) => shape.filterFields(one).map(({ value, ...field }: any) => field)),
        collated: inputs.collate.map((names: string[]) =>
            shape
                .filterFields({ ...imported('c', 'Name\nx'), rows: names.map((name, at) => ({ id: `r${at}`, c1: name })) })[0]
                .options.map((one: any) => one.value),
        ),
        starting: datasets.flatMap((one: any) =>
            ['sum', 'avg'].map((aggregate) => shape.startingSpec(one, { ...model.DEFAULT_PREFERENCES, aggregate })),
        ),
        breaks: inputs.breaks.map(shape.breaksOf),
    },
}

// --- Sharing ----------------------------------------------------------------------------------------
async function unpacked(text: string) {
    try {
        return { ok: await share.unpackChart(text) }
    } catch (error) {
        return { error: (error as Error).message }
    }
}
const packs = await Promise.all(seed.seedCharts().map(async (chart: any) => {
    const dataset = byId(chart.datasetId)
    const packed = await share.packChart(chart.name, chart.spec, dataset)
    const json = new TextDecoder().decode(Bun.inflateSync(Uint8Array.from(atob(packed.slice(1).replaceAll('-', '+').replaceAll('_', '/')), (c) => c.charCodeAt(0)), { windowBits: -15 } as any))
    return { kind: packed.charAt(0), packed, json: JSON.parse(json), back: await unpacked(packed) }
}))
const sharing = {
    packs: packs.map(({ packed, ...rest }) => rest),
    webPacks: packs.map(({ packed }) => packed),
    unpacked: await Promise.all(inputs.shares.map(unpacked)),
    datasets: (await Promise.all(packs.map((one) => unpacked(one.packed)))).map((one: any) => share.sharedDataset(one.ok, 'shared', 'at')),
    links: ['#/shared/zabc', ''].map((hash) => share.linkTo(hash)),
}

// --- The app, mounted on a stand-in for React's hooks ----------------------------------------------
const R = await import(new URL('react.ts', import.meta.url).pathname)
const React = await import(`${MODULES}react/index.js`)
;(React.default ?? React).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE.H = R.dispatcher

const UI = `${MODULES}@winterop-com/sokkel-ui/dist/components/`
const CHARTS = `${MODULES}@winterop-com/sokkel-ui-charts/dist/`
const { Visualizer } = await import(`${ROOT}Visualizer.tsx`)
const { ChartView } = await import(`${ROOT}ChartView.tsx`)
const { ImportDialog } = await import(`${ROOT}ImportDialog.tsx`)
const dialogs = await import(`${ROOT}dialogs.tsx`)
const { SettingsPage } = await import(`${ROOT}pages/Settings.tsx`)
const { Shell } = await import(`${UI}Shell.js`)
const { Rail } = await import(`${UI}Rail.js`)
const { TopBar } = await import(`${UI}TopBar.js`)
const { StatusLine, StatusItem } = await import(`${UI}StatusLine.js`)
const { PageHeader } = await import(`${UI}PageHeader.js`)
const { PageState } = await import(`${UI}PageState.js`)
const { DataTable } = await import(`${UI}DataTable.js`)
const { FilterBar } = await import(`${UI}FilterBar.js`)
const { SearchField } = await import(`${UI}SearchField.js`)
const { SelectField } = await import(`${UI}SelectField.js`)
const { Segmented } = await import(`${UI}Segmented.js`)
const { SwitchField } = await import(`${UI}SwitchField.js`)
const { TextField } = await import(`${UI}TextField.js`)
const { InlineEdit } = await import(`${UI}InlineEdit.js`)
const { ItemCard } = await import(`${UI}ItemCard.js`)
const { FormBar } = await import(`${UI}FormBar.js`)
const { ChoiceList } = await import(`${UI}ChoiceList.js`)
const { Input } = await import(`${UI}ui/input.js`)
const { Button } = await import(`${UI}ui/button.js`)
const { ToggleGroup } = await import(`${UI}ui/toggle-group.js`)
const { DropdownMenuItem } = await import(`${UI}ui/dropdown-menu.js`)
const { TabsTrigger } = await import(`${UI}ui/tabs.js`)
const { ConfirmDialog, DialogShell } = await import(`${UI}DialogShell.js`)
const { SettingsDialog } = await import(`${UI}SettingsDialog.js`)
const { DashboardPanel } = await import(`${UI}dashboard/DashboardGrid.js`)
const { DashboardArrange } = await import(`${UI}dashboard/DashboardArrange.js`)
const { StatPanel } = await import(`${UI}dashboard/StatPanel.js`)
const { CommandRegistry } = await import(`${MODULES}@winterop-com/sokkel-ui/dist/hooks/use-commands.js`)
const { toast } = await import(`${UI}ui/toast.js`)
const { ChartPanel } = await import(`${CHARTS}dashboard/ChartPanel.js`)
const { ChartFrame } = await import(`${CHARTS}ChartFrame.js`)
const backend = await import(`${ROOT}backend.ts`)
const { savingNow, savingNote } = await import(`${MODULES}tidevann-shared/backend/index.ts`)

let toasts: unknown[] = []
let actions: (() => void)[] = []
toast.add = (one: any) => {
    toasts.push({ type: one.type ?? null, title: one.title, description: one.description ?? null, action: one.actionProps?.children ?? null })
    actions.push(one.actionProps?.onClick ?? (() => {}))
    return ''
}
// The commands a page adds to the palette, kept by its id.
const commands = new Map<string, unknown>()
CommandRegistry._currentValue = { set: (id: string, groups: unknown) => commands.set(id, groups), remove: (id: string) => commands.delete(id) }

// Files handed out: what each holds, read once the step is done.
const blobs = new Map<string, Blob>()
let downloads: { name: string; href: string }[] = []
URL.createObjectURL = (blob: Blob) => {
    const url = `blob:${String(blobs.size + 1)}`
    blobs.set(url, blob)
    return url
}
URL.revokeObjectURL = () => {}
;(globalThis as any).document.createElement = () => {
    const anchor: any = { click: () => downloads.push({ name: anchor.download, href: anchor.href }), remove() {} }
    return anchor
}
;(globalThis as any).document.body = { append() {}, appendChild() {} }

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
function text(node: any): string {
    if (node === null || node === undefined || typeof node === 'boolean') return ''
    if (typeof node === 'string' || typeof node === 'number') return String(node)
    if (Array.isArray(node)) return node.map(text).join('')
    if (typeof node === 'object') return text(node.props?.children)
    return ''
}
/** A component run once on its own, for what it draws. */
const run = (element: any) => R.mount(() => element.type(element.props)).current()

async function flush() {
    for (let round = 0; round < 100; round += 1) {
        for (let i = 0; i < 30; i += 1) await Promise.resolve()
        const any = R.renderStale()
        if (page !== undefined) {
            if (keyOf(pageElement()) !== pageKey) mountPage()
            else page.render()
        }
        if (!any) return
    }
    throw new Error('the app never settled')
}
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

// --- The frame and the page it draws ---------------------------------------------------------------
let route: string[] = ['datasets']
const frame = () => ({
    route,
    palette: [],
    desktop: undefined,
    themes: [],
    appearance: { settings: {}, setSettings: () => {} },
    askReset: () => {},
    dialogs: null,
})
const app = R.mount(() => Visualizer({ frame: frame() }))
const shell = () => first(app.current(), Shell)
const pageElement = () => shell().props.children
let page: { current: () => any; render: () => void } | undefined
let pageKey = ''
const keyOf = (element: any) => `${typeof element.type === 'function' ? element.type.name : String(element.type)}:${String(element.key)}`
function mountPage() {
    const element = pageElement()
    const key = keyOf(element)
    if (key === pageKey && page !== undefined) return
    pageKey = key
    commands.clear()
    page = R.mount(() => {
        const current = pageElement()
        return typeof current.type === 'function' && current.type !== Shell ? current.type(current.props) : current
    })
}
const tree = () => page!.current()
async function go(next: string[]) {
    route = next
    app.render()
    await flush()
    mountPage()
    await flush()
}
// The dialogs the frame and the pages open, each mounted on its own.
let importDialog = R.mount(() => ImportDialog({ open: true, onOpenChange: () => {} }))
let boardDialog: any
let addDialog: any
let shareDialog: any
let settingsPage: any
let confirming: any

// --- Reading ---------------------------------------------------------------------------------------
const header = () => first(tree(), PageHeader)?.props
const titleOf = (title: any) => (typeof title === 'string' ? title : title?.props?.value ?? text(title))
function frameLook() {
    const node = app.current()
    const rail = first(first(node, Shell).props.rail, Rail).props.groups.map((group: any) => ({
        id: group.id,
        title: group.title ?? null,
        entries: group.entries.map((one: any) => ({ id: one.id, label: one.label, href: one.href, active: one.active ?? false, count: one.count ?? null })),
    }))
    const groups = first(node, Shell).props.palette.filter((group: any) => ['Actions', 'Datasets', 'Charts', 'Boards'].includes(group.title))
    const palette = groups.map((group: any) => ({
        title: group.title,
        rank: group.rank,
        commands: group.commands.map((one: any) => ({ id: one.id, label: one.label, hint: one.hint ?? null, keywords: one.keywords ?? [] })),
    }))
    const trail = first(first(node, Shell).props.top, TopBar).props.trail.map((one: any) => ({ label: one.label, href: one.href ?? null }))
    const foot = first(node, Shell).props.foot
    const status = find(foot, StatusItem).map((one: any) => one.props.children)
    return { rail, palette, trail, status, note: foot.props.note }
}
/** A cell a table column draws, as the Rust reads it. */
function shown(node: any) {
    if (typeof node === 'string' || typeof node === 'number') return { kind: 'value', text: String(node) }
    const style = node?.props?.className ?? ''
    if (style.includes('muted')) return { kind: 'empty' }
    if (style.includes('critical')) return { kind: 'unread', text: text(node) }
    return { kind: 'value', text: text(node) }
}
function chartLook(element: any) {
    const node = run(element)
    if (node.type === ChartPanel) {
        const p = node.props
        return { kind: 'chart', label: p.label, empty: p.empty, words: p.frameWords.empty, spec: specLook(p) }
    }
    if (node.type === ChartFrame) {
        const inner = node.props.children({ handle: { current: null } })
        const chart = all(inner).find((one: any) => one.props?.kind !== undefined)
        return { kind: 'chart', label: chart.props.label, empty: node.props.empty, words: node.props.words.empty, spec: specLook(chart.props) }
    }
    if (node.type === StatPanel) {
        const p = node.props
        return { kind: 'stat', value: p.value, sparkline: p.sparkline ?? null, delta: p.delta ?? null, color: p.color, note: p.note }
    }
    return { kind: 'panel', empty: node.props.empty ?? false, words: node.props.words?.empty ?? null }
}
function specLook(p: any) {
    return {
        x: p.x ?? null,
        y: p.y ?? null,
        series: p.series ?? null,
        value: p.value ?? null,
        order: p.order ?? null,
        aggregate: p.aggregate,
        stack: p.stack ?? null,
        horizontal: p.horizontal,
        palette: p.palette ?? null,
        fieldNames: p.fieldNames === undefined ? null : Object.entries(p.fieldNames)[0],
        showScale: p.showScale,
        fit: p.fit,
        rowCount: p.rows.length,
    }
}
const readers: Record<string, () => unknown> = {
    DatasetsPage() {
        return { count: header().count, shown: first(tree(), DataTable).props.rows.map((one: any) => one.id) }
    },
    DatasetPage() {
        const node = tree()
        if (header() === undefined) return { error: first(node, PageState).props.error }
        const [rows, columns, charts] = find(node, DataTable)
        const lines = rows.props.rows
        return {
            count: header().count,
            tab: text(find(node, TabsTrigger)[2]),
            rows: lines.length,
            ids: lines.slice(0, 5).map((line: any) => line.row.id),
            headers: rows.props.columns.map((one: any) => one.header),
            cells: lines.slice(0, 3).map((line: any) => rows.props.columns.map((one: any) => shown(one.cell(line)))),
            stats: columns.props.rows.map((line: any) => ({
                range: text(columns.props.columns[2].cell(line)),
                empty: Number(text(columns.props.columns[3].cell(line))),
                unread: Number(text(columns.props.columns[4].cell(line))),
            })),
            drawn: charts.props.rows.map((one: any) => one.id),
            empty: rows.props.empty,
        }
    },
    ChartsPage() {
        const table = first(tree(), DataTable).props
        return {
            count: header().count,
            shown: table.rows.map((one: any) => one.id),
            boards: table.rows.map((one: any) => table.columns[3].cell(one)),
            cards: table.rows.map((one: any) => table.card(one).props.description),
        }
    },
    BuilderPage() {
        const node = tree()
        if (first(node, FormBar) === undefined) return { empty: first(node, PageState).props.emptyMessage }
        const value = (id: string) => find(node, Segmented).find((one: any) => one.props.id === id)?.props
        const sw = (id: string) => find(node, SwitchField).find((one: any) => one.props.id === id)?.props
        const preview = find(node, ChartView)[0]
        return {
            header: header().title,
            slots: find(node, SelectField)
                .filter((one: any) => one.props.id.startsWith('slot-'))
                .map((one: any) => ({
                    field: one.props.id.slice(5),
                    label: one.props.label,
                    hint: one.props.hint ?? null,
                    description: one.props.description ?? null,
                    value: one.props.value,
                    options: one.props.options.map((o: any) => ({ value: o.value, label: o.label, detail: o.detail ?? null })),
                    placeholder: one.props.placeholder ?? null,
                    disabled: one.props.disabled,
                })),
            combine: value('chart-aggregate') === undefined ? null : [value('chart-aggregate').value, !value('chart-aggregate').options[0].disabled],
            bin: value('chart-bin')?.value ?? null,
            sort: value('chart-sort') === undefined ? null : value('chart-sort').options.map((one: any) => one.label),
            palettes: find(node, SelectField).find((one: any) => one.props.id === 'chart-palette').props.options.map((one: any) => [one.value, one.label]),
            stack: sw('chart-stack') === undefined ? null : [sw('chart-stack').checked, sw('chart-stack').disabled],
            horizontal: sw('chart-horizontal')?.checked ?? null,
            note: text(all(node).find((one: any) => one.type === 'p')),
            suggestions: find(node, ItemCard).map((one: any) => ({ title: one.props.title, reason: one.props.description })),
            formNote: first(node, FormBar).props.note,
            save: find(first(node, FormBar), Button).map((one: any) => [text(one), one.props.reason ?? null]),
            others: find(node, DropdownMenuItem).filter((one: any) => typeof one.key === 'string' && one.key.startsWith('bd')).map((one: any) => one.key),
            preview: chartLook(preview),
            draft: { name: find(node, TextField)[0].props.value, dataset: find(node, SelectField).find((one: any) => one.props.id === 'chart-dataset').props.value },
        }
    },
    BoardsPage() {
        return { count: header().count, shown: first(tree(), DataTable).props.rows.map((one: any) => one.id) }
    },
    BoardPage() {
        const node = tree()
        if (header() === undefined) return { error: first(node, PageState).props.error }
        const grid = all(node).find((one: any) => typeof one.type === 'function' && one.type.name === 'BoardGrid')
        const panels = grid === undefined ? [] : (() => {
            const arrange = run(grid)
            return arrange.props.layout.map((item: any) => {
                const drawn = arrange.props.renderPanel(item)
                if (drawn.type === DashboardPanel) return { kind: 'missing', title: drawn.props.title, text: text(drawn.props.children) }
                return { kind: 'chart', title: drawn.props.title, subtitle: drawn.props.subtitle, filters: drawn.props.filters, file: drawn.props.file, view: chartLook(drawn) }
            })
        })()
        const bar = first(node, FilterBar)
        return {
            count: header().count,
            description: header().description ?? null,
            items: grid?.props.items.map((one: any) => one.id) ?? [],
            arranging: grid?.props.arranging ?? false,
            fields: bar === undefined ? null : bar.props.fields.map((one: any) => ({ id: one.id, label: one.label, type: one.type, ...(one.options === undefined ? {} : { options: one.options.map((o: any) => o.value) }) })),
            note: bar === undefined ? null : text(all(node).find((one: any) => one.type === 'span' && String(one.props.className).includes('text-xs'))),
            panels,
            palette: grid?.props.palette.map((one: any) => one.id) ?? [],
        }
    },
    SharedPage() {
        const node = tree()
        const state = first(node, PageState)
        if (state !== undefined) return { loading: state.props.loading, error: state.props.error ?? null }
        return { title: header().title, description: header().description, chart: chartLook(first(node, ChartView)) }
    },
}
function pageLook() {
    const element = pageElement()
    const name = typeof element.type === 'function' ? element.type.name : 'Page'
    const read = readers[name]
    if (read === undefined) {
        const state = first(element, PageState)?.props
        return { page: name, loading: state?.loading ?? false, message: state?.emptyMessage ?? null }
    }
    return { page: name, ...(read() as object) }
}
function dialogsLook() {
    // Each dialog drawn again with what its page hands it now.
    importDialog.render()
    boardDialog?.render()
    addDialog?.render()
    shareDialog?.render()
    const imp = importDialog.current().props
    const lines = first(imp.children, DataTable)?.props.rows ?? []
    return {
        import: {
            why: imp.confirm.why ?? null,
            footer: imp.footer === undefined ? null : text(imp.footer),
            name: find(imp.children, TextField).find((one: any) => one.props.id === 'import-name')?.props.value ?? null,
            lines: lines.map((one: any) => ({ name: one.column.name, type: one.column.type, sample: one.sample })),
        },
        board: boardDialog === undefined ? null : (() => {
            const p = boardDialog.current().props
            return { title: p.title, description: p.description, confirm: p.confirm.label, why: p.confirm.why ?? null }
        })(),
        add: addDialog === undefined ? null : (() => {
            const p = addDialog.current().props
            const list = first(p.children, ChoiceList)?.props
            return { description: p.description, why: p.confirm.why ?? null, chosen: list?.value ?? null, offered: list?.options ?? [] }
        })(),
        share: shareDialog === undefined ? null : (() => {
            const p = shareDialog.current().props
            return { description: p.description, link: first(p.children, Input).props.value, footer: p.footer === undefined ? null : text(p.footer) }
        })(),
        settings: settingsPage === undefined ? null : (() => {
            settingsPage.render()
            const groups = first(settingsPage.current(), SettingsDialog).props.groups
            const byId: Record<string, any> = {}
            for (const group of groups) for (const section of group.sections) for (const one of section.settings) byId[one.id] = one
            return { style: byId.style.description, reset: byId.reset.description }
        })(),
        confirm: confirming === undefined ? null : { question: confirming.props.question, detail: confirming.props.detail },
    }
}

// --- Acting ----------------------------------------------------------------------------------------
const menuItem = (node: any, label: string) => find(node, DropdownMenuItem).find((one: any) => text(one.props.children).includes(label))
const tableOf = (at = 0) => find(tree(), DataTable)[at].props
const rowAction = (table: any, row: any, id: string) => table.rowActions(row).find((one: any) => one.id === id).onSelect()
async function act(step: any): Promise<void> {
    const node = tree()
    switch (step.op) {
        case 'go':
            await go(step.route)
            return
        case 'search':
            first(node, SearchField).props.onChange(step.value)
            return
        case 'rowAction': {
            const table = tableOf(step.table ?? 0)
            const row = table.rows.find((one: any) => (one.row?.id ?? one.column?.id ?? one.id) === step.row)
            rowAction(table, row, step.id)
            await flush()
            confirming = find(tree(), ConfirmDialog).find((one: any) => one.props.open)
            return
        }
        case 'confirm':
            confirming = find(tree(), ConfirmDialog).find((one: any) => one.props.open)
            confirming.props.confirm.onConfirm()
            confirming = undefined
            return
        case 'cell': {
            const table = tableOf(step.table ?? 0)
            const row = table.rows.find((one: any) => (one.row?.id ?? one.column?.id) === step.row)
            const column = table.columns.find((one: any) => one.id === step.column)
            column.edit.save(row, step.value)
            return
        }
        case 'bulk': {
            const table = tableOf(0)
            first(table.bulkActions(new Set(step.ids), () => {}), Button).props.onClick()
            return
        }
        case 'button':
            find(node, Button).find((one: any) => text(one.props.children).trim() === step.label).props.onClick()
            return
        case 'menu':
            menuItem(node, step.label).props.onClick()
            await flush()
            confirming = find(tree(), ConfirmDialog).find((one: any) => one.props.open)
            return
        case 'inline':
            find(node, InlineEdit)[step.index ?? 0].props.onSave(step.value)
            return
        case 'filters':
            first(node, FilterBar).props.onFiltersChange(step.filters)
            return
        case 'builder': {
            const select = (id: string) => find(node, SelectField).find((one: any) => one.props.id === id).props
            const segment = (id: string) => find(node, Segmented).find((one: any) => one.props.id === id).props
            const sw = (id: string) => find(node, SwitchField).find((one: any) => one.props.id === id).props
            if (step.do === 'name') find(node, TextField)[0].props.onChange(step.value)
            else if (step.do === 'dataset') select('chart-dataset').onChange(step.value)
            else if (step.do === 'view') first(node, ToggleGroup).props.onValueChange([step.value])
            else if (step.do === 'slot') select(`slot-${step.field}`).onChange(step.value)
            else if (step.do === 'aggregate') segment('chart-aggregate').onValueChange(step.value)
            else if (step.do === 'bin') segment('chart-bin').onValueChange(step.value)
            else if (step.do === 'sort') segment('chart-sort').onValueChange(step.value)
            else if (step.do === 'limit') select('chart-limit').onChange(step.value)
            else if (step.do === 'palette') select('chart-palette').onChange(step.value)
            else if (step.do === 'stack') sw('chart-stack').onCheckedChange(step.value)
            else if (step.do === 'horizontal') sw('chart-horizontal').onCheckedChange(step.value)
            else if (step.do === 'suggestion') find(node, ItemCard)[step.value].props.onSelect()
            else if (step.do === 'addTo') find(node, DropdownMenuItem).find((one: any) => one.key === step.value).props.onClick()
            else throw new Error(`no builder step ${String(step.do)}`)
            return
        }
        case 'board': {
            const grid = all(node).find((one: any) => typeof one.type === 'function' && one.type.name === 'BoardGrid')
            const arrange = run(grid)
            const item = arrange.props.layout.find((one: any) => one.id === step.item)
            const menu = (label: string) => menuItem(arrange.props.renderPanel(item).props.menu, label).props.onClick()
            if (step.do === 'takeOff') menu('Remove from board')
            else if (step.do === 'csv') menu('Download CSV')
            else if (step.do === 'edit') first(node, PageHeader).props.actions.props.edit.edit()
            else if (step.do === 'done') first(node, PageHeader).props.actions.props.edit.done()
            else if (step.do === 'cancel') first(node, PageHeader).props.actions.props.edit.cancel()
            else if (step.do === 'add') grid.props.onAdd(step.chart, step.span, step.index)
            else throw new Error(`no board step ${String(step.do)}`)
            return
        }
        case 'undo':
            actions[actions.length - 1 - step.index]()
            return
        case 'reorder':
            first(node, DataTable).props.onReorder(step.from, step.to)
            return
        case 'addDialog': {
            const element = find(node, dialogs.AddChartDialog)[0] ?? all(node).find((one: any) => one.type === dialogs.AddChartDialog)
            addDialog = R.mount(() => dialogs.AddChartDialog({ ...pageAdd(), open: true, onOpenChange: () => {} }))
            await flush()
            if (step.pick !== undefined) first(addDialog.current().props.children, ChoiceList).props.onValueChange(step.pick)
            if (step.span !== undefined) first(addDialog.current().props.children, Segmented).props.onValueChange(step.span)
            await flush()
            if (step.add === true) addDialog.current().props.confirm.onConfirm()
            void element
            return
        }
        case 'boardDialog': {
            boardDialog = R.mount(() => dialogs.BoardDialog({ open: true, onOpenChange: () => {}, onMade: (made: any) => (location.hash = `/boards/${made.id}`) }))
            await flush()
            const fields = find(boardDialog.current().props.children, TextField)
            fields[0].props.onChange(step.name)
            fields[1].props.onChange(step.description)
            await flush()
            if (step.make === true) boardDialog.current().props.confirm.onConfirm()
            return
        }
        case 'share': {
            const chart = backendRows('charts').find((one: any) => one.id === step.chart)
            const dataset = backendRows('datasets').find((one: any) => one.id === chart.datasetId)
            shareDialog = R.mount(() => dialogs.ShareDialog({ open: true, onOpenChange: () => {}, name: chart.name, spec: chart.spec, dataset }))
            return
        }
        case 'openShared': {
            const link = first(shareDialog.current().props.children, Input).props.value
            await go(['shared', step.payload ?? link.split('#/shared/')[1]])
            await advance(now + 1)
            if (step.keep === true) find(tree(), Button).find((one: any) => text(one.props.children).includes('Keep a copy')).props.onClick()
            return
        }
        case 'settings': {
            const frameWith = { ...frame(), route: ['settings', 'charts'] }
            settingsPage ??= R.mount(() => SettingsPage({ prefs: currentPrefs(), frame: frameWith, datasets: backendRows('datasets'), open: true, onClose: () => {} }))
            const dialog = first(settingsPage.current(), SettingsDialog).props
            if (step.do === 'reset') dialog.onReset(step.section)
            else if (step.do === 'resetAll') {
                const confirm = find(settingsPage.current(), ConfirmDialog)[0]
                confirm.props.confirm.onConfirm()
            } else {
                const byId: Record<string, any> = {}
                for (const group of dialog.groups) for (const section of group.sections) for (const one of section.settings) byId[one.id] = one
                const control = byId[step.id].control
                if (control.props.onChange !== undefined) control.props.onChange(step.value)
                else if (control.props.onValueChange !== undefined) control.props.onValueChange(step.value)
                else control.props.onCheckedChange(step.value)
            }
            return
        }
        case 'import': {
            const p = () => importDialog.current().props
            const children = () => p().children
            if (step.do === 'source') first(children(), Segmented).props.onValueChange(step.value)
            else if (step.do === 'paste') find(children(), TextField).find((one: any) => one.props.id === 'import-paste').props.onChange(step.value)
            else if (step.do === 'file')
                (all(children()).find((one: any) => one.props?.id === 'import-file')).props.onChange([new File([step.content], step.name)])
            else if (step.do === 'name') find(children(), TextField).find((one: any) => one.props.id === 'import-name').props.onChange(step.value)
            else if (step.do === 'column') {
                const table = first(children(), DataTable).props
                const line = table.rows[step.index]
                table.columns.find((one: any) => one.id === step.field).edit.save(line, step.value)
            } else if (step.do === 'save') p().confirm.onConfirm()
            else if (step.do === 'reopen') importDialog = R.mount(() => ImportDialog({ open: true, onOpenChange: () => {} }))
            return
        }
        default:
            throw new Error(`no step ${String(step.op)}`)
    }
}
const backendRows = (name: string) => JSON.parse(kept.get(`tidevann-demo:dataviz:${name}`) ?? 'null') ?? ({ datasets: seed.seedDatasets, charts: seed.seedCharts, boards: seed.seedBoards, preferences: () => [model.DEFAULT_PREFERENCES] } as any)[name]()
const currentPrefs = () => backendRows('preferences')[0]
/** What the board page hands its add dialog. */
let lastAdd: any
function pageAdd() {
    const element = all(tree()).find((one: any) => one.type === dialogs.AddChartDialog)
    if (element !== undefined) lastAdd = { board: element.props.board, charts: element.props.charts, datasets: element.props.datasets }
    return lastAdd
}

/** FNV-1a over a text's UTF-8 bytes. */
function hash32(text: string): number {
    const bytes = new TextEncoder().encode(text)
    let value = 0x811c9dc5
    for (const byte of bytes) value = Math.imul(value ^ byte, 0x01000193) >>> 0
    return value >>> 0
}
const summary = (dataset: any) => {
    const { rows, ...rest } = dataset
    return { ...rest, rowCount: rows.length, rowsHead: rows.slice(0, 2), rowsHash: hash32(JSON.stringify(rows)) }
}
async function look(): Promise<Record<string, unknown>> {
    const files = await Promise.all(downloads.map(async (one) => ({ name: one.name, text: await blobs.get(one.href)!.text() })))
    const out = {
        datasets: backendRows('datasets').map(summary),
        charts: backendRows('charts'),
        boards: backendRows('boards'),
        preferences: backendRows('preferences'),
        saving: savingNote(savingNow()),
        toasts,
        went,
        downloads: files,
        frame: frameLook(),
        view: pageLook(),
        dialogs: dialogsLook(),
    }
    toasts = []
    went = []
    downloads = []
    return out
}
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

let random = inputs.random >>> 0
Math.random = () => {
    random = (Math.imul(random, 1664525) + 1013904223) >>> 0
    return random / 4294967296
}
await advance(inputs.now + 900)
mountPage()
await flush()
const steps: unknown[] = [changed(await look())]
for (const step of inputs.steps) {
    await advance(inputs.now + step.at)
    try {
        await act(step)
    } catch (error) {
        throw new Error(`step ${JSON.stringify(step)} on ${keyOf(pageElement())}: ${String(error)}\n${(error as Error).stack}`)
    }
    await flush()
    if (step.go === true && went.length > 0) {
        const target = went[went.length - 1].replace(/^replace /, '').replace(/^#?\/?/, '')
        await go(target.split('/').filter((part: string) => part !== ''))
    }
    await advance(inputs.now + step.at + (step.wait ?? 900))
    if (step.follow === true) {
        const target = went.filter((one) => one !== 'reload').at(-1)
        if (target !== undefined) await go(target.replace(/^replace /, '').replace(/^#?\/?/, '').split('/').filter((part: string) => part !== ''))
        await advance(inputs.now + step.at + (step.wait ?? 900) + 1)
    }
    steps.push(changed(await look()))
}

process.stdout.write(JSON.stringify({ core, sharing, steps }))
