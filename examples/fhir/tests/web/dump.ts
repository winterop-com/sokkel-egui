/**
 * What the web's fhir modules give for the inputs in `inputs.json`, as JSON on stdout: run by
 * `tests/web.rs` with bun from the repository's root, the zone UTC, against `.parity/sokkel-ui`.
 * The clock, the timers and `performance.now` are held here and `Math.random` is a seeded run
 * while the steps play. The app and its pages run on a stand-in for React's hooks (`react.ts`),
 * each component the pages draw with hooks of their own (the side panel's tabs, the save dialog)
 * mounted on its own. Carried from the dhis2 example's (`examples/dhis2/tests/web/dump.ts`).
 */

const ROOT = new URL('../../../../.parity/sokkel-ui/examples/fhir/web/src/', import.meta.url).pathname
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
performance.now = () => 0
// FHIRPath's `trace` logs to the console; stdout carries the answer alone.
console.log = () => {}
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
const listeners: [string, (event: unknown) => void][] = []
const location = {
    href: `${inputs.origin}${inputs.pathname}#/`,
    origin: inputs.origin,
    pathname: inputs.pathname,
    get hash() {
        return hash
    },
    set hash(next: string) {
        hash = next.startsWith('#') ? next : `#${next}`
        went.push(next)
        for (const [kind, listener] of listeners) if (kind === 'hashchange') listener({})
    },
    replace: (next: string) => went.push(`replace ${next}`),
    reload: () => went.push('reload'),
}
const element = () => ({
    style: { setProperty() {}, removeProperty() {} },
    dataset: {},
    classList: { add() {}, remove() {}, toggle() {}, contains: () => false },
    setAttribute() {},
    removeAttribute() {},
    getAttribute: () => null,
    appendChild() {},
    remove() {},
    click() {},
})
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
        addEventListener: (kind: string, listener: (event: unknown) => void) => listeners.push([kind, listener]),
        removeEventListener: (kind: string, listener: (event: unknown) => void) => {
            const at = listeners.findIndex((one) => one[0] === kind && one[1] === listener)
            if (at >= 0) listeners.splice(at, 1)
        },
        matchMedia: () => ({ matches: false, addEventListener() {}, removeEventListener() {} }),
        history: { go: () => {} },
    },
    document: {
        head: null,
        body: { style: {}, appendChild() {}, append() {} },
        documentElement: element(),
        createElement: element,
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

const { configureDemo } = await import(`${MODULES}tidevann-shared/backend/index.ts`)
configureDemo({ app: 'fhir' })

const modelModule = await import(`${ROOT}data/model.ts`)
const typesModule = await import(`${ROOT}data/types.ts`)
const seedModule = await import(`${ROOT}data/seed.ts`)
const validateModule = await import(`${ROOT}data/validate.ts`)
const evaluateModule = await import(`${ROOT}data/evaluate.ts`)
const server = await import(`${ROOT}data/server.ts`)
const searchModule = await import(`${ROOT}data/search.ts`)
const editing = await import(`${ROOT}editing.ts`)
const { CARRIED } = await import(`${ROOT}themes.ts`)
const UI = `${MODULES}@winterop-com/sokkel-ui/dist/components/`
const { diffLines, foldDiff } = await import(`${UI}DiffView.js`)

const json = (value: unknown) => JSON.parse(JSON.stringify(value))
const seeded: any[] = seedModule.seed()
const seedResources = seeded.map((one: any) => one.resource)
const keyOf = (resource: any) => `${resource.resourceType}/${resource.id}`
const seedKeys = new Set(seedResources.map(keyOf))
const byKey = (key: string) => seedResources.find((one: any) => keyOf(one) === key)

// --- The data layer -----------------------------------------------------------------------------------

/** A resource an input names: a seed one with fields set and dropped, or one written in full. */
function resourceOf(input: any): unknown {
    if (input.raw !== undefined) return input.raw
    const base = input.base === undefined ? {} : structuredClone(byKey(input.base))
    for (const [name, value] of Object.entries(input.set ?? {})) base[name] = value
    for (const name of input.drop ?? []) delete base[name]
    return base
}

const model = {
    issuePaths: inputs.issuePaths.map(([type, path]: [string, (string | number)[] | null]) => modelModule.issuePath(type, path ?? undefined)),
    outcome: modelModule.outcome('Patient', inputs.outcomeIssues),
    refused: inputs.refusals.map(([status, issues]: [number, unknown[]]) => new modelModule.Refused(status, issues).message),
    concepts: inputs.concepts.map(modelModule.conceptText),
    names: inputs.names.map(modelModule.humanName),
    quantities: inputs.quantities.map(modelModule.quantityText),
    references: inputs.references.map(modelModule.referenceText),
    base64: inputs.texts.map((text: string) => {
        const data = modelModule.toBase64(text)
        return [data, modelModule.fromBase64(data)]
    }),
}

const typeTable = {
    names: typesModule.TYPE_NAMES,
    types: typesModule.TYPES,
    datatypes: typesModule.DATATYPES,
    schemas: [...typesModule.TYPE_NAMES, 'Nothing'].map((name: string) => typesModule.schemaFor(name) ?? null),
    patterns: inputs.patterns.map(([type, value]: [string, string]) => typesModule.patternOf(type)?.test(value) ?? null),
}

const exists = (key: string) => seedKeys.has(key)
const validations = inputs.validations.map((input: any) =>
    validateModule.validate(resourceOf(input), input.type, input.id ?? undefined, exists),
)
const everySeed = seedResources.map((resource: any) => validateModule.validate(resource, resource.resourceType, resource.id, exists))
const headers = inputs.cql.map((text: string) => validateModule.cqlHeader(text) ?? null)

const subjects: Record<string, unknown> = {
    bundle: {
        resourceType: 'Bundle',
        id: 'server',
        type: 'collection',
        total: seedResources.length,
        entry: seedResources.map((resource: any) => ({ resource })),
    },
}
const subjectOf = (name: string) => subjects[name] ?? byKey(name)
const fhirpath = {
    evaluations: inputs.fhirpath.map(([subject, expression]: [string, string]) => evaluateModule.evaluate(expression, subjectOf(subject))),
    none: evaluateModule.evaluate('id', undefined),
    functions: evaluateModule.FUNCTIONS,
    variables: evaluateModule.VARIABLES,
    elementNames: inputs.elementNames.map((name: string) =>
        evaluateModule.elementNames(name === 'bundle' ? seedResources.slice(0, 80) : subjectOf(name)),
    ),
    words: inputs.words.map((word: string) => evaluateModule.describeWord(word)),
}

const stored = seedResources.map((resource: any) => ({ id: keyOf(resource), type: resource.resourceType, resource }))
const searchTable = {
    params: [...typesModule.TYPE_NAMES, 'Nothing'].map((name: string) => searchModule.paramsOf(name)),
    resultParams: searchModule.RESULT_PARAMS,
    queries: inputs.queries.map((text: string) => searchModule.readQuery(text)),
    results: inputs.searches.map(([text, size]: [string, number]) => {
        const result = searchModule.search(stored, text, size)
        return { ...result, matches: result.matches.map((row: any) => row.id) }
    }),
}

const serverTable = {
    server: server.SERVER,
    me: server.ME,
    pointing: inputs.pointing.map((key: string) => server.pointingAt(stored, key)),
    parsed: inputs.json.map((text: string) => {
        try {
            return { ok: server.parse(text) }
        } catch (error: any) {
            return { refused: { status: error.status, issues: error.issues, message: error.message } }
        }
    }),
}

const diffs = inputs.diffs.map(([before, after, context]: [string, string, number]) => {
    const lines = diffLines(before, after)
    return { lines, rows: foldDiff(lines, context) }
})

const seedTable = {
    vitals: seedModule.VITALS,
    templates: [...typesModule.TYPE_NAMES, 'Nothing'].map((name: string) => seedModule.template(name)),
    seeded,
}

// --- The app, mounted on the stand-in ----------------------------------------------------------------
const R = await import(new URL('react.ts', import.meta.url).pathname)
const React = await import(`${MODULES}react/index.js`)
;(React.default ?? React).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE.H = R.dispatcher

const ui = await import(`${ROOT}ui.tsx`)
const uiTable = {
    statuses: ui.STATUSES,
    when: inputs.instants.map((one: string | null) => ui.when(one ?? undefined)),
    onDay: inputs.days.map((one: string | null) => ui.onDay(one ?? undefined)),
    counts: inputs.counts.map(([value, one, many]: [number, string, string | null]) => ui.count(value, one, many ?? undefined)),
    titles: seedResources.map(ui.titleOf),
    states: seedResources.map((resource: any) => ui.stateOf(resource) ?? null),
    extraTitles: inputs.titled.map((input: any) => ui.titleOf(resourceOf(input))),
    extraStates: inputs.titled.map((input: any) => ui.stateOf(resourceOf(input)) ?? null),
    prefs: ui.DEFAULT_PREFS,
}

const { Shell } = await import(`${UI}Shell.js`)
const { Rail } = await import(`${UI}Rail.js`)
const { TopBar } = await import(`${UI}TopBar.js`)
const { StatusItem } = await import(`${UI}StatusLine.js`)
const { PageHeader } = await import(`${UI}PageHeader.js`)
const { PageState } = await import(`${UI}PageState.js`)
const { DataTable } = await import(`${UI}DataTable.js`)
const { SearchField } = await import(`${UI}SearchField.js`)
const { Segmented } = await import(`${UI}Segmented.js`)
const { StatusChip } = await import(`${UI}StatusChip.js`)
const { Stat } = await import(`${UI}Stat.js`)
const { ConfirmDialog, DialogShell } = await import(`${UI}DialogShell.js`)
const { TextField } = await import(`${UI}TextField.js`)
const { SelectField } = await import(`${UI}SelectField.js`)
const { Banner } = await import(`${UI}Banner.js`)
const { ApiLink } = await import(`${UI}ApiLink.js`)
const { CodeEditor } = await import(`${UI}CodeEditor.js`)
const { DiffView } = await import(`${UI}DiffView.js`)
const { SidePanel, PanelSection } = await import(`${UI}SidePanel.js`)
const { SettingsDialog } = await import(`${UI}SettingsDialog.js`)
const { NotFound } = await import(`${UI}NotFound.js`)
const { Button } = await import(`${UI}ui/button.js`)
const { DropdownMenuItem } = await import(`${UI}ui/dropdown-menu.js`)
const { toast } = await import(`${UI}ui/toast.js`)
const { ChartFrame } = await import(`${MODULES}@winterop-com/sokkel-ui-charts/dist/ChartFrame.js`)
const { App } = await import(`${ROOT}App.tsx`)

let toasts: unknown[] = []
toast.add = (one: any) => {
    toasts.push({ type: one.type ?? null, title: one.title, description: one.description ?? null })
    return ''
}
let foot: { note: string | null; items: string | null } = { note: null, items: null }

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
const named = (tree: any, name: string) => all(tree).filter((node) => typeof node.type === 'function' && node.type.name === name)
function text(node: any): string {
    if (node === null || node === undefined || typeof node === 'boolean') return ''
    if (typeof node === 'string' || typeof node === 'number') return String(node)
    if (Array.isArray(node)) return node.map(text).join('')
    if (typeof node === 'object') {
        if (node.type === StatusChip) return `[${String(node.props.status)}]`
        return text(node.props?.children)
    }
    return ''
}
ui.FootContext._currentValue = (next: { note?: unknown; items?: unknown }) => {
    foot = { note: typeof next.note === 'string' ? next.note : null, items: next.items === undefined ? null : text(next.items) }
}

// Components with hooks of their own, each mounted once under its type and key and drawn again
// with the props its parent hands it now.
const subs = new Map<unknown, Map<string, { element: any; mounted: any }>>()
function sub(element: any): any {
    let mine = subs.get(element.type)
    if (mine === undefined) {
        mine = new Map()
        subs.set(element.type, mine)
    }
    const key = String(element.key ?? element.props.id ?? '')
    let found = mine.get(key)
    if (found === undefined) {
        const made: { element: any; mounted: any } = { element, mounted: undefined }
        made.mounted = R.mount(() => made.element.type(made.element.props))
        mine.set(key, made)
        found = made
    } else {
        found.element = element
        found.mounted.render()
    }
    return found.mounted.current()
}
function dropSubs() {
    for (const mine of subs.values()) for (const one of mine.values()) one.mounted.unmount()
    subs.clear()
}

const app = R.mount(() => App())
const shell = () => first(app.current(), Shell).props
const pageElement = () => shell().children
const panelElement = () => shell().panel
const keyOfElement = (element: any) =>
    element === undefined || element === null
        ? ''
        : `${typeof element.type === 'function' ? element.type.name : String(element.type)}:${String(element.key)}`
let page: { current: () => any; render: () => void; unmount: () => void } | undefined
let pageKey = ''
let panel: { current: () => any; render: () => void; unmount: () => void } | undefined
let panelKey = ''
function mountPage() {
    const key = `${keyOfElement(pageElement())}|${String(shell().contentKey)}`
    if (key !== pageKey || page === undefined) {
        page?.unmount()
        dropSubs()
        pageKey = key
        foot = { note: null, items: null }
        let last: any = null
        page = R.mount(() => {
            const current = pageElement()
            // A page left draws nothing more: the route moved on before it was unmounted.
            if (`${keyOfElement(current)}|${String(shell().contentKey)}` !== key) return last
            last = typeof current.type === 'function' ? current.type(current.props) : current
            return last
        })
    }
    const pk = keyOfElement(panelElement())
    if (pk !== panelKey) {
        panel?.unmount()
        panel = undefined
        panelKey = pk
        let last: any = null
        if (pk !== '')
            panel = R.mount(() => {
                const current = panelElement()
                if (keyOfElement(current) !== pk) return last
                last = current.type(current.props)
                return last
            })
    }
}
async function flush() {
    for (let round = 0; round < 100; round += 1) {
        for (let i = 0; i < 30; i += 1) await Promise.resolve()
        const any = R.renderStale()
        mountPage()
        page?.render()
        panel?.render()
        // The components the page and the panel draw with hooks of their own, mounted as they are drawn.
        if (panel !== undefined) for (const tab of first(panel.current(), SidePanel).props.tabs) sub(tab.content)
        for (const dialog of named(page?.current(), 'SaveDialog')) sub(dialog)
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
const tree = () => page!.current()

// --- Reading ---------------------------------------------------------------------------------------
const headerOf = (node: any) => {
    const p = first(node, PageHeader)?.props
    return p === undefined
        ? null
        : { title: p.title, code: p.code ?? null, description: p.description ?? null, meta: p.meta === undefined ? null : text(p.meta) }
}
const buttonsOf = (node: any) =>
    find(node, Button).map((one: any) => ({
        label: text(one.props.children).trim() || one.props['aria-label'] || '',
        reason: one.props.reason ?? null,
        busy: one.props.busy ?? false,
    }))
const editorOf = (one: any) =>
    one === undefined ? null : { label: one.props.label, value: one.props.value, marks: one.props.marks ?? [], readOnly: one.props.readOnly ?? false }
const diffOf = (node: any) => {
    const view = first(node, DiffView)
    if (view === undefined) return null
    const lines = diffLines(view.props.before, view.props.after)
    return {
        rows: foldDiff(lines, view.props.context ?? 3),
        added: lines.filter((line: any) => line.kind === 'added').length,
        removed: lines.filter((line: any) => line.kind === 'removed').length,
        unchanged: view.props.words?.unchanged ?? null,
    }
}
const cells = (table: any) =>
    table.rows.map((row: any) => table.columns.map((column: any) => text(column.cell(row))))
const tableOf = (table: any, id: (row: any) => string) => ({
    ids: table.rows.map(id),
    headers: table.columns.map((column: any) => column.header),
    cells: cells(table),
    empty: table.empty ?? null,
})

const frameLook = () => {
    const sh = shell()
    return {
        rail: first(sh.rail, Rail).props.groups.map((group: any) => ({
            id: group.id,
            title: group.title ?? null,
            entries: group.entries.map((one: any) => ({
                id: one.id,
                label: one.label,
                description: one.description ?? null,
                icon: one.icon?.displayName ?? one.icon?.name ?? null,
                href: one.href,
                active: one.active,
                count: one.count ?? null,
                keywords: one.keywords ?? [],
                entries: (one.entries ?? []).map((child: any) => ({ id: child.id, label: child.label, href: child.href, active: child.active, count: child.count ?? null })),
            })),
        })),
        trail: first(sh.top, TopBar).props.trail.map((one: any) => ({ label: one.label, href: one.href ?? null, code: one.code ?? false })),
        palette: sh.palette
            .filter((group: any) => ['Pages', 'Actions'].includes(group.title))
            .map((group: any) => ({
                title: group.title,
                verb: group.verb ?? null,
                commands: group.commands.map((one: any) => ({ id: one.id, label: one.label, hint: one.hint ?? null, verb: one.verb ?? null, keywords: one.keywords ?? [] })),
            })),
        search: inputs.paletteSearches.map((query: string) =>
            sh.paletteSearch.search(query).map((group: any) => ({ kind: group.kind, title: group.title, results: group.results.map((one: any) => ({ id: one.id, title: one.title, meta: one.meta })) })),
        ),
        contentKey: sh.contentKey,
        settingsOpen: first(app.current(), SettingsDialog).props.open,
    }
}

const readers: Record<string, () => unknown> = {
    Overview() {
        const node = tree()
        const state = first(node, PageState).props
        const frame = first(node, ChartFrame)
        const chart = frame.props.children({}).props
        const table = first(node, DataTable).props
        return {
            header: headerOf(node),
            loading: state.loading,
            stats: find(node, Stat).map((one: any) => ({ label: one.props.label, value: one.props.value })),
            chart: { title: frame.props.title, subtitle: frame.props.subtitle, label: chart.label, rows: chart.rows, x: chart.x, y: chart.y, series: chart.series },
            changes: tableOf(table, (row) => row.id),
        }
    },
    Resources() {
        const node = tree()
        const table = first(node, DataTable).props
        const segmented = first(table.toolbar, Segmented).props
        return {
            header: headerOf(node),
            api: first(node, ApiLink).props.path,
            loading: table.loading,
            query: first(table.search, SearchField).props.value,
            types: segmented.options.map((one: any) => ({ value: one.value, label: one.label, count: one.count })),
            type: segmented.value,
            table: tableOf(table, (row) => row.id),
            sortValues: table.rows.map((row: any) => table.columns.map((column: any) => column.sortValue?.(row) ?? null)),
            defaultSort: table.defaultSort,
        }
    },
    ResourcePage() {
        const node = tree()
        const state = first(node, PageState).props
        const confirm = first(node, ConfirmDialog).props
        return {
            header: headerOf(node),
            buttons: buttonsOf(first(node, PageHeader)?.props.actions),
            api: first(node, ApiLink)?.props.path ?? null,
            state: { loading: state.loading, error: state.error ?? null, status: state.status ?? null },
            editor: editorOf(first(node, CodeEditor)),
            confirm: { open: confirm.open, question: confirm.question, detail: confirm.detail, busy: confirm.confirm.busy ?? false },
        }
    },
    FhirPathPage() {
        const node = tree()
        const select = first(node, SelectField).props
        const editors = find(node, CodeEditor)
        const banner = first(node, Banner)
        return {
            header: headerOf(node),
            buttons: buttonsOf(first(node, PageHeader)?.props.actions),
            examples: find(node, DropdownMenuItem).map((one: any) => text(one.props.children)),
            subject: select.value,
            options: select.options,
            panes: named(node, 'Pane').map((one: any) => ({ title: one.props.title, meta: one.props.meta ?? null })),
            expression: editorOf(editors[0]),
            words: editors[0].props.completions.map((one: any) => one.label),
            result: banner === undefined ? { editor: editorOf(editors[1]) } : { banner: text(banner.props.children), tone: banner.props.tone },
            subjectEditor: editors[editors.length - 1].props.label,
        }
    },
    CqlLibraries() {
        const node = tree()
        const table = first(node, DataTable).props
        return { header: headerOf(node), buttons: buttonsOf(first(node, PageHeader)?.props.actions), table: tableOf(table, (row) => row.id) }
    },
    CqlEditor() {
        const node = tree()
        const state = first(node, PageState).props
        const table = first(node, DataTable)?.props
        return {
            header: headerOf(node),
            buttons: buttonsOf(first(node, PageHeader)?.props.actions),
            state: { loading: state.loading, error: state.error ?? null, status: state.status ?? null },
            panes: named(node, 'Pane').map((one: any) => ({ title: one.props.title, meta: one.props.meta ?? null })),
            editor: editorOf(first(node, CodeEditor)),
            versions: table === undefined ? null : { ...tableOf(table, (row) => row.id), opened: table.rows.filter((row: any) => table.opened(row)).map((row: any) => row.id) },
            diff: diffOf(node),
        }
    },
    SearchPage() {
        const node = tree()
        const tables = find(node, DataTable).map((one: any) => one.props)
        const editors = find(node, CodeEditor)
        const banner = first(node, Banner)
        const segments = find(node, Segmented).map((one: any) => one.props.value)
        const dialog = named(node, 'SaveDialog')[0]
        const inner = dialog === undefined ? undefined : sub(dialog)
        const shellProps = inner === undefined ? undefined : first(inner, DialogShell).props
        return {
            header: headerOf(node),
            buttons: buttonsOf(node),
            api: first(node, ApiLink)?.props.path ?? null,
            examples: find(node, DropdownMenuItem).map((one: any) => text(one.props.children)),
            query: editorOf(editors[0]),
            banner: banner === undefined ? null : { text: text(banner.props.children), tone: banner.props.tone },
            panes: named(node, 'Pane').map((one: any) => ({ title: one.props.title, meta: one.props.meta ?? null })),
            segments,
            tables: tables.map((table: any) => tableOf(table, (row) => row.id ?? row.name)),
            bundle: editors.length > 1 ? editors[1].props.value : null,
            dialog:
                shellProps === undefined
                    ? null
                    : { open: shellProps.open, query: text(shellProps.description), label: first(inner, TextField).props.value, why: shellProps.confirm.why ?? null, busy: shellProps.confirm.busy },
        }
    },
}

const panelLook = () => {
    if (panel === undefined) return null
    const side = first(panel.current(), SidePanel).props
    return {
        default: side.defaultActive,
        tabs: side.tabs.map((tab: any) => {
            const inner = sub(tab.content)
            const sections = find(inner, PanelSection).map((one: any) => {
                const table = named(one.props.children, 'PanelList')[0]?.props
                return { title: one.props.title, table: table === undefined ? null : tableOf(table, (row) => row.id ?? row.key) }
            })
            const charts = find(inner, ChartFrame).map((one: any) => {
                const chart = one.props.children({}).props
                return { title: one.props.title, subtitle: one.props.subtitle, empty: one.props.empty, rows: chart.rows, label: chart.label }
            })
            return { id: tab.id, label: tab.label, sections, charts, diff: diffOf(inner) }
        }),
    }
}

const pageLook = () => {
    const element = pageElement()
    const name = element.type === NotFound ? 'NotFound' : typeof element.type === 'function' ? element.type.name : 'Page'
    const read = readers[name]
    return { page: name, ...(read === undefined ? {} : (read() as object)), foot }
}

// --- Acting ----------------------------------------------------------------------------------------
const buttonIn = (node: any, label: string) =>
    find(node, Button).find((one: any) => text(one.props.children).trim() === label || one.props['aria-label'] === label)
const tableIn = (node: any, index = 0) => find(node, DataTable)[index].props
const press = (node: any, label: string) => {
    const found = buttonIn(node, label)
    if (found === undefined) throw new Error(`no button ${label}`)
    if (found.props.reason === undefined) found.props.onClick()
}
const panelTab = (id: string) => sub(first(panel!.current(), SidePanel).props.tabs.find((tab: any) => tab.id === id).content)
let calls: unknown[] = []

/** The text an edit step leaves: its own, or the editor's with a part replaced, or its JSON with fields set and dropped. */
function edited(current: string, step: any): string {
    if (step.value !== undefined) return step.value
    if (step.replace !== undefined) return current.replace(step.replace[0], step.replace[1])
    const parsed = JSON.parse(current)
    for (const [name, value] of Object.entries(step.set ?? {})) parsed[name] = value
    for (const name of step.drop ?? []) delete parsed[name]
    return JSON.stringify(parsed, null, 2)
}

async function act(step: any): Promise<void> {
    const node = tree()
    switch (step.op) {
        case 'go':
            location.hash = step.path
            return
        case 'prefs': {
            const settings = first(app.current(), SettingsDialog).props.groups[0].sections.flatMap((section: any) => section.settings)
            const control = settings.find((one: any) => one.id === step.id).control.props
            if (control.onCheckedChange !== undefined) control.onCheckedChange(step.value)
            else control.onValueChange(step.value)
            return
        }
        case 'stat':
            find(node, Stat).find((one: any) => one.props.label === step.label).props.onSelect()
            return
        case 'open': {
            const table = tableIn(node, step.table ?? 0)
            table.onOpen(table.rows.find((row: any) => (row.id ?? row.name) === step.id))
            return
        }
        case 'search':
            first(tableIn(node).search, SearchField).props.onChange(step.value)
            return
        case 'segment':
            find(node, Segmented)[step.index ?? 0].props.onValueChange(step.value)
            return
        case 'button':
            press(node, step.label)
            return
        case 'edit': {
            const editor = find(node, CodeEditor)[step.index ?? 0].props
            editor.onChange(edited(editor.value, step))
            return
        }
        case 'run':
            find(node, CodeEditor)[0].props.onRun?.(step.value)
            return
        case 'confirm':
            first(node, ConfirmDialog).props.confirm.onConfirm()
            return
        case 'cancel':
            first(node, ConfirmDialog).props.onOpenChange(false)
            return
        case 'example':
            find(node, DropdownMenuItem)
                .find((one: any) => text(one.props.children) === step.label)
                .props.onClick()
            return
        case 'subject':
            first(node, SelectField).props.onChange(step.value)
            return
        case 'version': {
            const table = named(panelTab('history'), 'PanelList')[0].props
            table.onOpen(table.rows.find((row: any) => row.versionId === step.value))
            return
        }
        case 'reference': {
            const tables = named(panelTab('references'), 'PanelList').map((one: any) => one.props)
            const table = tables[step.table ?? 0]
            table.onOpen(table.rows.find((row: any) => row.key === step.key))
            return
        }
        case 'remove': {
            const table = tableIn(node, 1)
            table.rowActions(table.rows.find((row: any) => row.id === step.id))[0].onSelect()
            return
        }
        case 'reorder':
            tableIn(node, 1).onReorder(step.from, step.to)
            return
        case 'label':
            first(sub(named(node, 'SaveDialog')[0]), TextField).props.onChange(step.value)
            return
        case 'dialog': {
            const props = first(sub(named(node, 'SaveDialog')[0]), DialogShell).props
            if (step.close === true) props.onOpenChange(false)
            else props.confirm.onConfirm()
            return
        }
        case 'server': {
            const record = (promise: Promise<any>) => {
                promise.then(
                    (value) => calls.push({ ok: value ?? null }),
                    (error: any) => calls.push({ failed: { status: error.status ?? null, message: error.message, issues: error.issues ?? null } }),
                )
            }
            if (step.call === 'update') record(server.update(step.type, step.id, resourceOf(step.resource), step.readAt))
            else if (step.call === 'create') record(server.create(step.type, resourceOf(step.resource)))
            else record(server.remove(step.key))
            return
        }
        case 'wait':
            return
        default:
            throw new Error(`no step ${String(step.op)}`)
    }
}

const backendRows = (name: string): any[] | null => JSON.parse(kept.get(`tidevann-demo:fhir:${name}`) ?? 'null')
const seedHistory = server.history === undefined ? 0 : (await drain(server.history.list())).length
async function drain<T>(answer: Promise<T>): Promise<T> {
    let result: { value: T } | undefined
    void answer.then((value) => (result = { value }))
    while (result === undefined) {
        timers.sort((a, b) => a.at - b.at || a.seq - b.seq)
        const next = timers.shift()
        if (next !== undefined) next.fn()
        for (let i = 0; i < 10; i += 1) await Promise.resolve()
    }
    return result.value
}
const seedJson = new Map(stored.map((row: any) => [row.id, JSON.stringify(row)]))
function serverLook() {
    const rows = backendRows('resources') ?? stored
    const history = backendRows('history')
    const saved = backendRows('saved-searches')
    return {
        resources: rows.map((row: any) => `${row.id}@${String(row.resource.meta?.versionId ?? '')}`),
        touched: rows.filter((row: any) => seedJson.get(row.id) !== JSON.stringify(row)),
        history: history === null ? [] : history.slice(0, history.length - seedHistory),
        saved: saved === null ? null : saved,
    }
}

/** The values the app keeps in the browser with `useStored`. */
const STORED = ['prefs', 'search-query', 'search-ran', 'search-side', 'fhirpath-expression', 'fhirpath-subject']

async function look(): Promise<Record<string, unknown>> {
    const stores = {
        drafts: editing.drafts.get(),
        answers: editing.answers.get(),
        reveal: editing.reveals.get(),
        stored: Object.fromEntries(
            STORED.filter((key) => kept.has(`tidevann-demo:fhir:${key}`)).map((key) => [key, JSON.parse(kept.get(`tidevann-demo:fhir:${key}`)!)]),
        ),
    }
    const out = {
        route: hash,
        server: serverLook(),
        stores,
        toasts,
        went,
        calls,
        frame: frameLook(),
        view: pageLook(),
        panel: panelLook(),
    }
    toasts = []
    went = []
    calls = []
    return json(out)
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
now = inputs.now
await advance(inputs.now + 900)
mountPage()
await advance(inputs.now + 1800)
const steps: unknown[] = [changed(await look())]
for (const step of inputs.steps) {
    await advance(inputs.now + step.at)
    try {
        await act(step)
    } catch (error) {
        throw new Error(`step ${JSON.stringify(step)} on ${pageKey}: ${String(error)}\n${(error as Error).stack}`)
    }
    await flush()
    await advance(inputs.now + step.at + (step.wait ?? 900))
    steps.push(changed(await look()))
}

const core = {
    model: json(model),
    types: json(typeTable),
    seed: json(seedTable),
    validations: json(validations),
    everySeed: json(everySeed),
    headers: json(headers),
    fhirpath: json(fhirpath),
    search: json(searchTable),
    server: json(serverTable),
    diffs: json(diffs),
    ui: json(uiTable),
    themes: CARRIED,
}
process.stdout.write(JSON.stringify({ core, steps }))
