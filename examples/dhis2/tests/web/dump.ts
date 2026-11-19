/**
 * What the web's dhis2 modules give for the inputs in `inputs.json`, as JSON on stdout: run by
 * `tests/web.rs` with bun from the repository's root, the zone UTC, against `.parity/sokkel-ui` at
 * the commit `parity/dhis2.json` names. The clock and the timers are held here and `Math.random`
 * is a seeded run while the steps play. The app and its pages run on a stand-in for React's hooks
 * (`react.ts`), each component the pages draw with hooks of their own (an editor, the form, an
 * expression field) mounted on its own.
 */

const ROOT = new URL('../../../../.parity/sokkel-ui/examples/dhis2/web/src/', import.meta.url).pathname
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

const { configureDemo, savingNow, savingNote } = await import(`${MODULES}tidevann-shared/backend/index.ts`)
configureDemo({ app: 'dhis2' })

const metadata = await import(`${ROOT}data/metadata.ts`)
const values = await import(`${ROOT}data/values.ts`)
const versions = await import(`${ROOT}data/versions.ts`)
const expression = await import(`${ROOT}data/expression.ts`)
const checking = await import(`${ROOT}data/check.ts`)
const program = await import(`${ROOT}data/program.ts`)
const backend = await import(`${ROOT}data/backend.ts`)
const { CARRIED } = await import(`${ROOT}themes.ts`)

// --- Placeholders in the inputs, as the Rust fills them --------------------------------------------
const DE = metadata.DE
const PLACES: Record<string, string> = {
    visits: DE.visits.id,
    referrals: DE.referrals.id,
    vaccinated: DE.vaccinated.id,
    due: DE.due.id,
    taken: DE.samplesTaken.id,
    failed: DE.samplesFailed.id,
    received: DE.stockReceived.id,
    used: DE.stockUsed.id,
    onHand: DE.stockOnHand.id,
    under5: metadata.AGE_COMBO.combos[0].id,
    over5: metadata.AGE_COMBO.combos[1].id,
    plain: metadata.DEFAULT_COMBO.combos[0].id,
    target: metadata.CONSTANTS[0].id,
    samples: metadata.CONSTANTS[1].id,
    set: metadata.DATA_SET.id,
}
const fill = (text: string): string => text.replace(/\$(\w+)/g, (all, name: string) => PLACES[name] ?? all)
const unitOf = (code: string): string => metadata.ORG_UNITS.find((one: any) => one.code === code)?.id ?? code
const idOf = (ref: string): string => {
    const [kind, code] = ref.split(':')
    if (kind === 'ind') return metadata.uid(`ind:${code}`)
    if (kind === 'vr') return metadata.uid(`vr:${code}`)
    if (kind === 'prr') return metadata.uid(`prr:${code}`)
    if (kind === 'ou') return unitOf(code)
    if (kind === 'de') return (DE as any)[code].id
    return ref
}
const refs = (route: string[]) => route.map((part) => (part.includes(':') ? idOf(part) : part))

// --- The data layer -----------------------------------------------------------------------------------
const UNITS = metadata.ORG_UNITS.map((one: any) => one.id)
const PERIODS_ALL = [...metadata.PERIODS, ...inputs.extraPeriods]
const json = (value: unknown) => JSON.parse(JSON.stringify(value))

const meta = {
    hashed: inputs.hashed.map(metadata.hashed),
    uids: inputs.uids.map(metadata.uid),
    orgUnits: metadata.ORG_UNITS,
    root: metadata.ROOT,
    tree: metadata.ORG_UNIT_TREE,
    levels: metadata.LEVELS,
    categoryCombos: metadata.CATEGORY_COMBOS,
    combos: metadata.COMBOS,
    dataElements: metadata.DATA_ELEMENTS,
    dataSet: metadata.DATA_SET,
    metrics: metadata.REPORTING_METRICS,
    constants: metadata.CONSTANTS,
    periods: metadata.PERIODS,
    today: metadata.TODAY,
    units: UNITS.map((id: string) => ({
        children: metadata.childrenOf(id).map((one: any) => one.id),
        path: metadata.pathOf(id).map((one: any) => one.id),
        sites: metadata.sitesUnder(id).map((one: any) => one.id),
    })),
    unknownUnit: { found: metadata.orgUnit('nothing') ?? null, path: metadata.pathOf('nothing'), sites: metadata.sitesUnder('nothing') },
    lookups: {
        elements: [...metadata.DATA_ELEMENTS.map((one: any) => one.id), 'nothing'].map((id: string) => metadata.dataElement(id)?.code ?? null),
        combos: [...metadata.COMBOS.map((one: any) => one.id), 'nothing'].map((id: string) => metadata.optionCombo(id)?.name ?? null),
        constants: [...metadata.CONSTANTS.map((one: any) => one.id), 'nothing'].map((id: string) => metadata.constant(id)?.code ?? null),
    },
    periodLabels: metadata.PERIODS.map(metadata.periodLabel),
    shifted: inputs.shifts.map(([id, by]: [string, number]) => metadata.shiftPeriod(id, by)),
    days: inputs.days.map(metadata.daysIn),
}

const valueTable = {
    values: metadata.DATA_ELEMENTS.flatMap((element: any) =>
        // Every combo of its own, all of them summed, and one it is not split by.
        [null, ...element.categoryCombo.combos.map((one: any) => one.id), element.categoryCombo.name === 'default' ? PLACES.under5 : PLACES.plain].map(
            (combo: string | null) => UNITS.map((unit: string) => PERIODS_ALL.map((period: string) => values.dataValue(element.id, combo, unit, period))),
        ),
    ),
    unknown: values.dataValue('nothing', null, metadata.ROOT, metadata.PERIODS[0]),
    reports: UNITS.map((unit: string) => PERIODS_ALL.map((period: string) => values.reports(unit, period))),
}

const versionTable = {
    versions: versions.VERSIONS,
    latest: versions.LATEST,
    added: versions.ADDED,
    names: [...versions.VERSIONS, '2.40'].map((one: string) => versions.namesOf(one)),
}

// --- Expressions ----------------------------------------------------------------------------------------
/** A backend call's answer, its timer fired at once without moving the clock. */
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
const indicatorSeed: any[] = await drain(backend.indicators.list())
const ruleSeed: any[] = await drain(backend.validationRules.list())
const programSeed: any[] = await drain(backend.programRules.list())
const heldIds = indicatorSeed.map((one: any) => one.id)
const tried = <T,>(work: () => T): { ok: T } | { error: { message: string; span: unknown } } => {
    try {
        return { ok: work() }
    } catch (error) {
        // A span the parser hands as the token it stopped at carries the token's own fields too; its place is what counts.
        if (error instanceof expression.ExpressionError) return { error: { message: error.message, span: { from: error.span.from, to: error.span.to } } }
        throw error
    }
}
/** What a call gives, or that it throws, as the web's evaluator does on an argument it does not have. */
const throwing = <T,>(work: () => T): T | { throws: true } => {
    try {
        return work()
    } catch {
        return { throws: true }
    }
}
const PLACES_TRIED = inputs.places.map(([code, period]: [string, string]) => [unitOf(code), period])
const scopeOf = (form: Record<string, unknown>, eventDate: string) => ({
    item: (node: { kind: string; body: string }) => {
        const body = node.body.trim()
        if (node.kind === '#' || node.kind === 'A') return (form[body] as any) ?? null
        if (node.kind === 'C') return metadata.constant(body)?.value ?? null
        if (node.kind === 'V') return body === 'event_date' ? eventDate : body === 'current_date' ? metadata.TODAY : null
        return null
    },
})
const contextOf = (language: string, version: string) =>
    language === 'rule'
        ? { version, variables: program.VARIABLES, evaluatedVariables: program.EVALUATED_VARIABLES }
        : { version, indicators: heldIds }
const expressions = inputs.expressions.map((raw: string) => {
    const source = fill(raw)
    const parsed = tried(() => expression.parse(source))
    const node = 'ok' in parsed ? parsed.ok : null
    return {
        tokens: tried(() => expression.tokenize(source)),
        parsed,
        checks: ['expression', 'rule'].flatMap((language) =>
            [...versions.VERSIONS].map((version: string) => checking.check(source, language, contextOf(language, version)).findings),
        ),
        bare: checking.check(source, 'expression', { version: versions.LATEST }).findings,
        aggregate: node === null ? null : PLACES_TRIED.map(([unit, period]: [string, string]) => throwing(() => backend.aggregate(node, unit, period))),
        forms: node === null ? null : inputs.forms.map(([form, date]: [Record<string, unknown>, string]) => throwing(() => expression.evaluate(node, scopeOf(form, date)))),
        described: expression.describe(source, (kind: string, body: string) => checking.itemName(kind, body.trim())),
        withoutComments: expression.withoutBlockComments(source),
        withoutCommentsBy: expression.withoutBlockComments(source, (comment: string) => `<${String(comment.length)}>`),
    }
})
const patterns = inputs.patterns.map(([value, pattern]: [string, string]) =>
    expression.evaluate(expression.parse('d2:validatePattern(#{value}, #{pattern})'), scopeOf({ value, pattern }, '')).value,
)
const itemNames = inputs.itemNames.map(([kind, body]: [string, string]) => checking.itemName(kind, fill(body)) ?? null)

// --- The backend's analytics -------------------------------------------------------------------------
const analytics = {
    server: backend.SERVER,
    me: backend.ME,
    types: backend.INDICATOR_TYPES,
    operators: backend.OPERATORS,
    strategies: backend.STRATEGIES,
    indicators: indicatorSeed,
    rules: ruleSeed,
    values: indicatorSeed.map((one) => {
        const top = expression.parse(one.numerator)
        const bottom = expression.parse(one.denominator)
        return UNITS.map((unit: string) => PERIODS_ALL.map((period: string) => backend.indicatorValue(top, bottom, one.type, unit, period)))
    }),
    runs: ruleSeed.map((one) => backend.runRule(one, expression.parse(one.left), expression.parse(one.right), metadata.sitesUnder(metadata.ROOT).map((site: any) => site.id), metadata.PERIODS)),
    variants: inputs.variants.flatMap(([code, units]: [string, string[]]) => {
        const one = ruleSeed.find((rule) => rule.code === code)
        const left = expression.parse(one.left)
        const right = expression.parse(one.right)
        return Object.keys(backend.OPERATORS).flatMap((operator) =>
            Object.keys(backend.STRATEGIES).flatMap((leftStrategy) =>
                Object.keys(backend.STRATEGIES).map((rightStrategy) => {
                    const run = backend.runRule({ ...one, operator, leftStrategy, rightStrategy }, left, right, units.map(unitOf), PERIODS_ALL)
                    return { tried: run.tried, skipped: run.skipped, count: run.violations.length, head: run.violations.slice(0, 3) }
                }),
            ),
        )
    }),
    unitNames: [...UNITS, 'nothing'].map(backend.unitName),
}

// --- The program ---------------------------------------------------------------------------------------
const runOf = (run: any) => ({ hidden: [...run.hidden], assigned: run.assigned, effects: run.effects, broken: run.broken })
const programTable = {
    program: program.PROGRAM,
    fields: program.FIELDS,
    variables: program.VARIABLES,
    evaluated: program.EVALUATED_VARIABLES,
    actions: program.ACTIONS,
    seed: programSeed,
    runs: inputs.forms.flatMap(([form, date]: [Record<string, unknown>, string]) =>
        [...versions.VERSIONS].map((version: string) => runOf(program.runRules(programSeed, form, date, version))),
    ),
    edited: inputs.ruleEdits.map(([at, condition, data]: [number, string, string | null]) => {
        const rules = programSeed.map((one, index) =>
            index === at ? { ...one, condition: fill(condition), actions: data === null ? one.actions : one.actions.map((action: any) => ({ ...action, data: fill(data) })) } : one,
        )
        return inputs.forms.map(([form, date]: [Record<string, unknown>, string]) => runOf(program.runRules(rules, form, date, versions.LATEST)))
    }),
}

// --- The pages' shared parts ----------------------------------------------------------------------------
const R = await import(new URL('react.ts', import.meta.url).pathname)
const React = await import(`${MODULES}react/index.js`)
;(React.default ?? React).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE.H = R.dispatcher

const ui = await import(`${ROOT}ui.tsx`)
const checkedOf = (source: string, language: string, version: string) => checking.check(source, language, contextOf(language, version))
const uiTable = {
    figures: inputs.figures.map(([value, unit]: [number | null, string | null]) => ui.figure(value, unit ?? undefined)),
    counts: inputs.counts.map(([value, one, many]: [number, string, string | null]) => ui.count(value, one, many ?? undefined)),
    issues: inputs.expressions.map((raw: string) => ui.issuesOf(fill(raw), checkedOf(fill(raw), 'expression', versions.LATEST))),
    verdicts: inputs.expressions.map((raw: string) => ui.verdict(checkedOf(fill(raw), 'expression', versions.LATEST), checkedOf(fill(raw), 'rule', versions.LATEST))),
    describedItems: inputs.expressions.map((raw: string) => ui.describeItems(fill(raw))),
    statuses: ui.STATUSES,
}

/** An expression field drawn on its own: what it lists, says under itself and underlines. */
function fieldLook(mounted: any) {
    const field = mounted.props
    const editor = field.children.props
    return {
        id: field.id,
        label: field.label,
        value: editor.value,
        language: editor.language,
        issues: field.issues,
        help: field.help ?? null,
        marks: editor.marks,
        invalid: editor.invalid,
    }
}

/** The words an expression field offers as it is typed and names under the pointer, for each language and version. */
function editorWords() {
    return ['expression', 'rule'].flatMap((language) =>
        [...versions.VERSIONS].map((one: string) => {
            ui.VersionContext._currentValue = one
            const context = language === 'rule' ? { variables: program.VARIABLES, evaluatedVariables: program.EVALUATED_VARIABLES } : undefined
            const field = R.mount(() =>
                ui.ExpressionField({ id: 'words', label: 'Words', value: '', onChange: () => {}, language, checked: { node: null, findings: [] }, context }),
            ).current()
            const editor = field.props.children.props
            return {
                completions: inputs.completions.map((before: string) => editor.completions('', { before: fill(before) })),
                hover: inputs.hover.map((word: string) => editor.hover(fill(word)) ?? null),
            }
        }),
    )
}

// --- The app, mounted on the stand-in ----------------------------------------------------------------
const UI = `${MODULES}@winterop-com/sokkel-ui/dist/components/`
const { Shell } = await import(`${UI}Shell.js`)
const { Rail } = await import(`${UI}Rail.js`)
const { TopBar } = await import(`${UI}TopBar.js`)
const { ServerMenu } = await import(`${UI}ServerMenu.js`)
const { StatusItem } = await import(`${UI}StatusLine.js`)
const { PageHeader } = await import(`${UI}PageHeader.js`)
const { PageState } = await import(`${UI}PageState.js`)
const { DataTable } = await import(`${UI}DataTable.js`)
const { SearchField } = await import(`${UI}SearchField.js`)
const { FilterChip } = await import(`${UI}FilterChip.js`)
const { StatusChip } = await import(`${UI}StatusChip.js`)
const { Stat } = await import(`${UI}Stat.js`)
const { SaveBar } = await import(`${UI}SaveBar.js`)
const { ConfirmDialog } = await import(`${UI}DialogShell.js`)
const { TextField } = await import(`${UI}TextField.js`)
const { SelectField } = await import(`${UI}SelectField.js`)
const { TreeSelect } = await import(`${UI}TreeSelect.js`)
const { PeriodPicker } = await import(`${UI}PeriodPicker.js`)
const { Section } = await import(`${UI}Section.js`)
const { Tree } = await import(`${UI}Tree.js`)
const { ComputedField } = await import(`${UI}ComputedField.js`)
const { YesNoField } = await import(`${UI}YesNoField.js`)
const { NumberField } = await import(`${UI}NumberField.js`)
const { DateField } = await import(`${UI}DateField.js`)
const { RepeatedGroup } = await import(`${UI}RepeatedGroup.js`)
const { DetailSheet } = await import(`${UI}DetailSheet.js`)
const { DescriptionList } = await import(`${UI}DescriptionList.js`)
const { TextLink } = await import(`${UI}TextLink.js`)
const { Badge } = await import(`${UI}ui/badge.js`)
const { Button } = await import(`${UI}ui/button.js`)
const { DropdownMenuRadioGroup } = await import(`${UI}ui/dropdown-menu.js`)
const { toast } = await import(`${UI}ui/toast.js`)
const { ChartFrame } = await import(`${MODULES}@winterop-com/sokkel-ui-charts/dist/ChartFrame.js`)
const { NotFound } = await import(`${UI}NotFound.js`)
const { App } = await import(`${ROOT}App.tsx`)

let toasts: unknown[] = []
toast.add = (one: any) => {
    toasts.push({ type: one.type ?? null, title: one.title, description: one.description ?? null })
    return ''
}
let foot: string | null = null
ui.FootContext._currentValue = (next: { note?: unknown }) => {
    foot = typeof next.note === 'string' ? next.note : null
}

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
const byId = (tree: any, id: string) => all(tree).find((node) => node.props?.id === id)
function text(node: any): string {
    if (node === null || node === undefined || typeof node === 'boolean') return ''
    if (typeof node === 'string' || typeof node === 'number') return String(node)
    if (Array.isArray(node)) return node.map(text).join('')
    if (typeof node === 'object') return text(node.props?.children)
    return ''
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

async function flush() {
    for (let round = 0; round < 100; round += 1) {
        for (let i = 0; i < 30; i += 1) await Promise.resolve()
        const any = R.renderStale()
        ui.VersionContext._currentValue = version()
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

const words = editorWords()
ui.VersionContext._currentValue = versions.LATEST
const app = R.mount(() => App())
const shell = () => first(app.current(), Shell)
const version = (): string => first(shell().props.top, ServerMenu).props.version
const pageElement = () => shell().props.children
let page: { current: () => any; render: () => void } | undefined
let pageKey = ''
const keyOf = (element: any) => `${typeof element.type === 'function' ? element.type.name : String(element.type)}:${String(element.key)}`
function mountPage() {
    const element = pageElement()
    const key = keyOf(element)
    if (key === pageKey && page !== undefined) return
    pageKey = key
    subs.clear()
    foot = null
    page = R.mount(() => {
        const current = pageElement()
        return typeof current.type === 'function' ? current.type(current.props) : current
    })
}
const tree = () => page!.current()
async function go(path: string) {
    location.hash = path
    await flush()
    mountPage()
    await flush()
}

// --- Reading ---------------------------------------------------------------------------------------
const headerOf = (node: any) => {
    const p = first(node, PageHeader)?.props
    return p === undefined ? null : { title: p.title, code: p.code ?? null, description: p.description ?? null }
}
const frameLook = () => {
    const node = app.current()
    const sh = first(node, Shell).props
    return {
        rail: first(sh.rail, Rail).props.groups.map((group: any) => ({
            id: group.id,
            title: group.title ?? null,
            entries: group.entries.map((one: any) => ({ id: one.id, label: one.label, description: one.description, icon: one.icon.displayName ?? one.icon.name ?? null, href: one.href, active: one.active, keywords: one.keywords })),
        })),
        trail: first(sh.top, TopBar).props.trail.map((one: any) => ({ label: one.label, href: one.href ?? null, code: one.code ?? false })),
        server: { version: version(), facts: first(sh.top, ServerMenu).props.facts, status: text(first(sh.foot, StatusItem)) },
        palette: sh.palette
            .filter((group: any) => ['Pages', 'Server version', 'Actions'].includes(group.title))
            .map((group: any) => ({
                title: group.title,
                verb: group.verb ?? null,
                commands: group.commands.map((one: any) => ({ id: one.id, label: one.label, hint: one.hint ?? null, code: one.code ?? null, verb: one.verb ?? null, keywords: one.keywords ?? [] })),
            })),
        search: inputs.searches.map((query: string) => sh.paletteSearch.search(query).map((group: any) => ({ kind: group.kind, title: group.title, results: group.results.map((one: any) => ({ id: one.id, title: one.title, meta: one.meta })) }))),
        searchMin: sh.paletteSearch.minQuery,
    }
}
const fieldsOf = (node: any) => named(node, 'ExpressionField').map((one: any) => fieldLook(sub(one)))
const statsOf = (node: any) => find(node, Stat).map((one: any) => ({ label: one.props.label, value: one.props.value, note: one.props.note ?? null, emphasis: one.props.emphasis ?? false }))
const chartRows = (node: any) => {
    const frame = first(node, ChartFrame)
    if (frame === undefined) return null
    return { empty: frame.props.empty ?? false, subtitle: frame.props.subtitle ?? null, rows: frame.props.children({}).props.rows, label: frame.props.children({}).props.label }
}
const valueOfField = (node: any, id: string) => byId(node, id)?.props.value
const readers: Record<string, () => unknown> = {
    Indicators() {
        const node = tree()
        const table = first(node, DataTable).props
        return {
            header: headerOf(node),
            rows: table.rows.map((one: any) => ({
                id: one.id,
                name: one.name,
                code: one.code,
                type: one.type,
                state: one.state,
                value: one.value,
                findings: one.findings,
                formula: text(table.columns[2].cell(one)),
                check: table.columns[3].cell(one).props.label,
                shown: text(table.columns[4].cell(one)),
            })),
            valueHeader: table.columns[4].header,
            tableFoot: table.foot ?? null,
            empty: table.words?.empty ?? null,
            filters: find(node, FilterChip).map((one: any) => ({ facet: one.props.facet, options: one.props.options, selected: one.props.selected })),
        }
    },
    IndicatorPage() {
        const node = tree()
        const editor = named(node, 'Editor')[0]
        const state = first(node, PageState).props
        if (editor === undefined) return { loading: state.loading, empty: state.empty, message: state.emptyMessage }
        const inner = sub(editor)
        const below = first(inner, DataTable)?.props
        return {
            header: headerOf(inner),
            draft: {
                name: valueOfField(inner, 'indicator-name'),
                type: valueOfField(inner, 'indicator-type'),
                description: valueOfField(inner, 'indicator-description'),
            },
            typeOptions: byId(inner, 'indicator-type').props.options,
            fields: fieldsOf(inner),
            unit: first(inner, TreeSelect).props.value,
            period: first(inner, PeriodPicker).props.value,
            stats: statsOf(inner),
            chart: chartRows(inner),
            below: below === undefined ? null : { header: below.columns[0].header, rows: below.rows, cells: below.rows.map((one: any) => below.columns.map((column: any) => text(column.cell(one)))) },
            changed: first(inner, SaveBar)?.props.count ?? 0,
            busy: first(inner, SaveBar)?.props.busy ?? false,
            confirm: { open: first(inner, ConfirmDialog).props.open, question: first(inner, ConfirmDialog).props.question, detail: first(inner, ConfirmDialog).props.detail },
        }
    },
    ValidationRules() {
        const node = tree()
        const table = first(node, DataTable).props
        return {
            header: headerOf(node),
            rows: table.rows.map((one: any) => ({
                id: one.id,
                name: one.name,
                code: one.code,
                importance: one.importance,
                state: one.state,
                violations: one.violations,
                rule: text(table.columns[2].cell(one)),
            })),
            empty: table.words?.empty ?? null,
        }
    },
    ValidationRulePage() {
        const node = tree()
        const editor = named(node, 'Editor')[0]
        const state = first(node, PageState).props
        if (editor === undefined) return { loading: state.loading, empty: state.empty, message: state.emptyMessage }
        const inner = sub(editor)
        const table = first(inner, DataTable)?.props
        const select = (id: string) => byId(inner, id).props
        return {
            header: headerOf(inner),
            draft: {
                name: valueOfField(inner, 'rule-name'),
                importance: valueOfField(inner, 'rule-importance'),
                instruction: valueOfField(inner, 'rule-instruction'),
                operator: valueOfField(inner, 'rule-operator'),
                leftStrategy: valueOfField(inner, 'rule-left-strategy'),
                rightStrategy: valueOfField(inner, 'rule-right-strategy'),
                from: valueOfField(inner, 'rule-from'),
                to: valueOfField(inner, 'rule-to'),
            },
            helps: [select('rule-left-strategy').help, select('rule-right-strategy').help],
            options: { operators: select('rule-operator').options, strategies: select('rule-left-strategy').options, periods: select('rule-from').options },
            fields: fieldsOf(inner),
            unit: first(inner, TreeSelect).props.value,
            runnable: !find(inner, Button).find((one: any) => text(one.props.children).trim() === 'Run').props.disabled,
            stats: statsOf(inner),
            rows:
                table === undefined
                    ? null
                    : table.rows.map((one: any) => {
                          const site = table.columns[0].cell(one).props.children
                          return {
                              id: one.id,
                              site: text(site[0]),
                              path: text(site[1]),
                              month: text(table.columns[1].cell(one)),
                              left: text(table.columns[2].cell(one)),
                              sign: text(table.columns[3].cell(one)),
                              right: text(table.columns[4].cell(one)),
                          }
                      }),
            empty: table?.words?.empty ?? null,
            changed: first(inner, SaveBar)?.props.count ?? 0,
            busy: first(inner, SaveBar)?.props.busy ?? false,
            confirm: { open: first(inner, ConfirmDialog).props.open, question: first(inner, ConfirmDialog).props.question, detail: first(inner, ConfirmDialog).props.detail },
        }
    },
    ProgramRules() {
        const node = tree()
        const list = first(node, Tree).props
        const editor = named(node, 'Editor')[0]
        const form = named(node, 'TryForm')[0]
        const state = first(node, PageState).props
        const out: Record<string, unknown> = { nodes: list.nodes, selected: list.selected, empty: state.empty, message: state.emptyMessage }
        if (editor !== undefined) {
            const inner = sub(editor)
            const group = first(inner, RepeatedGroup).props
            out.editor = {
                header: headerOf(inner),
                draft: {
                    name: valueOfField(inner, 'program-rule-name'),
                    priority: valueOfField(inner, 'program-rule-priority'),
                    description: valueOfField(inner, 'program-rule-description'),
                },
                fields: fieldsOf(inner),
                actions: group.rows.map((row: any) => {
                    const action = sub(group.children(row.key))
                    return {
                        key: row.key,
                        type: byId(action, `${row.key}-type`).props.value,
                        field: byId(action, `${row.key}-field`)?.props.value ?? null,
                        content: byId(action, `${row.key}-content`)?.props.value ?? null,
                        data: named(action, 'ExpressionField').map((one: any) => fieldLook(sub(one)))[0] ?? null,
                    }
                }),
                changed: first(inner, SaveBar)?.props.count ?? 0,
                busy: first(inner, SaveBar)?.props.busy ?? false,
                confirm: { open: first(inner, ConfirmDialog).props.open, question: first(inner, ConfirmDialog).props.question, detail: first(inner, ConfirmDialog).props.detail },
            }
        }
        if (form !== undefined) {
            const inner = sub(form)
            const WIDGETS = new Map<unknown, string>([
                [ComputedField, 'computed'],
                [SelectField, 'option'],
                [YesNoField, 'boolean'],
                [NumberField, 'number'],
                [DateField, 'date'],
                [TextField, 'text'],
            ])
            const tables = find(inner, DataTable).map((one: any) => one.props)
            const [effects, broken] = tables
            const sections = find(inner, Section).map((one: any) => ({ title: one.props.title, meta: one.props.meta ?? null }))
            out.form = {
                eventDate: byId(inner, 'try-event-date').props.value,
                fields: all(inner)
                    .filter((one: any) => typeof one.props?.id === 'string' && one.props.id.startsWith('try-') && one.props.id !== 'try-event-date' && WIDGETS.has(one.type))
                    .map((one: any) => ({ variable: one.key, id: one.props.id, label: one.props.label, widget: WIDGETS.get(one.type), value: one.props.value ?? null, issues: one.props.issues ?? [] })),
                effects: effects.rows.map((row: any) => {
                    const cell = effects.columns[1].cell(row).props.children
                    return {
                        id: row.id,
                        rule: effects.columns[0].cell(row),
                        action: text(cell[0]),
                        field: cell[1] === false ? null : (cell[1].props.children[1] ?? null),
                        text: cell[2] === false ? null : text(cell[2]),
                    }
                }),
                broken: broken === undefined ? null : broken.rows.map((row: any) => ({ id: row.id, rule: broken.columns[0].cell(row), why: broken.columns[1].cell(row) })),
                sections,
                words: effects.words.empty,
            }
        }
        return out
    },
    DataElements() {
        const node = tree()
        const table = first(node, DataTable).props
        const sheet = first(node, DetailSheet).props
        const lists = find(node, DescriptionList)
        const sections = find(node, Section)
        const read = sections.find((one: any) => String(one.props.title).startsWith('Read by'))
        return {
            header: headerOf(node),
            rows: table.rows.map((one: any) => {
                const combo = table.columns[2].cell(one)
                const used = table.columns[4].cell(one)
                return {
                    id: one.id,
                    name: one.name,
                    code: one.code,
                    disaggregation: combo.type === Badge ? text(combo) : null,
                    valueType: one.valueType,
                    readBy: typeof used === 'string' ? used : null,
                    value: text(table.columns[5].cell(one)),
                }
            }),
            valueHeader: table.columns[5].header,
            empty: table.words.empty,
            sheet: sheet.open
                ? {
                      title: sheet.title,
                      code: sheet.code,
                      description: sheet.description,
                      facts: lists[0].props.items.map((one: any) => ({ term: one.term, value: one.value })),
                      combos: lists[1].props.items.map((one: any) => ({ term: one.term, value: one.value })),
                      readTitle: read.props.title,
                      readers: find(read, TextLink).map((one: any) => ({ href: one.props.href, name: one.props.children[0], kind: text(one.props.children[2]) })),
                      none: all(read).find((one: any) => one.type === 'p') === undefined ? null : text(all(read).find((one: any) => one.type === 'p')),
                  }
                : null,
        }
    },
    OrgUnits() {
        const node = tree()
        const table = first(node, DataTable).props
        const facts = first(node, DescriptionList).props.items
        const valuesSection = find(node, Section).find((one: any) => one.props.title === 'Data values').props
        return {
            header: headerOf(node),
            facts: facts.map((one: any) => ({ term: one.term, value: typeof one.value === 'string' ? one.value : text(one.value) })),
            chart: chartRows(node),
            values: table.rows.map((one: any) => ({ ...one, shown: text(table.columns[2].cell(one)) })),
            valuesDescription: valuesSection.description,
            period: valuesSection.actions.props.value,
            expanded: first(node, Tree).props.defaultExpanded,
            selected: first(node, Tree).props.selected,
        }
    },
    Functions() {
        const node = tree()
        const table = first(node, DataTable).props
        return {
            header: headerOf(node),
            rows: table.rows,
            knownHeader: table.columns[4].header,
            facets: find(node, FilterChip).map((one: any) => one.props.facet),
            empty: table.words.empty,
        }
    },
}
const pageLook = () => {
    const element = pageElement()
    const name = element.type === NotFound ? 'NotFound' : typeof element.type === 'function' ? element.type.name : 'Page'
    const read = readers[name]
    return { page: name, ...(read === undefined ? {} : (read() as object)), foot }
}

// --- Acting ----------------------------------------------------------------------------------------
const button = (node: any, label: string) => find(node, Button).find((one: any) => text(one.props.children).trim() === label)
/** The node the page's own editor draws, or the page's when it has none. */
const editorNode = () => {
    const editor = named(tree(), 'Editor')[0]
    return editor === undefined ? tree() : sub(editor)
}
const formNode = () => sub(named(tree(), 'TryForm')[0])
function change(node: any, id: string, value: unknown) {
    const found = byId(node, id)
    if (found === undefined) {
        // An expression field, by the id it hands its editor.
        const field = named(node, 'ExpressionField').find((one: any) => one.props.id === id)
        field.props.onChange(value)
        return
    }
    const p = found.props
    if (p.onChange !== undefined) p.onChange(value)
    else p.onValueChange(value)
}
async function act(step: any): Promise<void> {
    switch (step.op) {
        case 'go':
            await go(`/${refs(step.route).join('/')}`)
            return
        case 'version':
            first(shell().props.top, DropdownMenuRadioGroup).props.onValueChange(step.value)
            return
        case 'search':
            first(tree(), SearchField).props.onChange(step.value)
            return
        case 'filter':
            find(tree(), FilterChip).find((one: any) => one.props.facet.startsWith(step.facet)).props.onSelectedChange(step.values)
            return
        case 'button':
            button(tree(), step.label).props.onClick()
            return
        case 'edit':
            change(step.form === true ? formNode() : editorNode(), step.id, typeof step.value === 'string' ? fill(step.value) : step.value)
            return
        case 'unit':
            first(editorNode(), TreeSelect).props.onValueChange(step.code === null ? [] : [unitOf(step.code)])
            return
        case 'period':
            first(step.page === true ? tree() : editorNode(), PeriodPicker).props.onValueChange(step.value)
            return
        case 'open': {
            const table = first(editorNode(), DataTable).props
            table.onOpen(table.rows.find((one: any) => one.id === unitOf(step.code)))
            return
        }
        case 'save':
            first(editorNode(), SaveBar).props.onSave()
            return
        case 'discard':
            first(editorNode(), SaveBar).props.onDiscard()
            return
        case 'delete': {
            button(editorNode(), 'Delete').props.onClick()
            await flush()
            const confirm = first(editorNode(), ConfirmDialog).props
            if (step.confirm !== false) confirm.confirm.onConfirm()
            return
        }
        case 'run': {
            const run = button(editorNode(), 'Run').props
            if (!run.disabled) run.onClick()
            return
        }
        case 'select':
            first(tree(), Tree).props.onSelect([idOf(step.ref)])
            return
        case 'move':
            first(tree(), Tree).props.onMove(idOf(step.ref), null, step.index)
            return
        case 'addAction':
            first(editorNode(), RepeatedGroup).props.onAdd()
            return
        case 'removeAction': {
            const group = first(editorNode(), RepeatedGroup).props
            group.onRemove(group.rows[step.index].key)
            return
        }
        case 'moveAction':
            first(editorNode(), RepeatedGroup).props.onMove(step.from, step.to)
            return
        case 'action': {
            const group = first(editorNode(), RepeatedGroup).props
            const key = group.rows[step.index].key
            change(sub(group.children(key)), `${key}-${step.field}`, typeof step.value === 'string' ? fill(step.value) : step.value)
            return
        }
        case 'clear':
            button(formNode(), 'Clear').props.onClick()
            return
        case 'tree': {
            const id = step.ref === null ? undefined : idOf(step.ref)
            first(tree(), Tree).props.onSelect(id === undefined ? [] : [id])
            return
        }
        case 'wait':
            return
        default:
            throw new Error(`no step ${String(step.op)}`)
    }
}

const backendRows = (name: string, seed: () => unknown[]) => JSON.parse(kept.get(`tidevann-demo:dhis2:${name}`) ?? 'null') ?? seed()
async function look(): Promise<Record<string, unknown>> {
    const out = {
        route: hash,
        indicators: backendRows('indicators', () => indicatorSeed),
        rules: backendRows('validation-rules', () => ruleSeed),
        programRules: backendRows('program-rules', () => programSeed),
        saving: savingNote(savingNow()).note,
        toasts,
        went,
        frame: frameLook(),
        view: pageLook(),
    }
    toasts = []
    went = []
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
await advance(inputs.now + 900)
mountPage()
await advance(inputs.now + 1800)
const steps: unknown[] = [changed(await look())]
for (const step of inputs.steps) {
    await advance(inputs.now + step.at)
    try {
        await act(step)
    } catch (error) {
        throw new Error(`step ${JSON.stringify(step)} on ${keyOf(pageElement())}: ${String(error)}\n${(error as Error).stack}`)
    }
    await flush()
    mountPage()
    await advance(inputs.now + step.at + (step.wait ?? 900))
    mountPage()
    await flush()
    steps.push(changed(await look()))
}

const core = {
    meta: json(meta),
    values: json(valueTable),
    versions: json(versionTable),
    expressions: json(expressions),
    patterns,
    itemNames,
    analytics: json(analytics),
    program: json(programTable),
    ui: json(uiTable),
    words: json(words),
    themes: CARRIED,
}
process.stdout.write(JSON.stringify({ core, steps }))
