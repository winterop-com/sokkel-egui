/**
 * What the web's studio modules give for the inputs in `inputs.json`, as JSON on stdout: run by
 * `tests/web.rs` with bun from the repository's root, the zone UTC, against `.parity/sokkel-ui` at
 * the commit `parity/studio.json` names. The backend's timers and clock are held here, so a run
 * goes as it would with every timer firing on time.
 */

const ROOT = new URL('../../../../.parity/sokkel-ui/examples/studio/web/src/', import.meta.url).pathname
const inputs = JSON.parse(await Bun.file(new URL('inputs.json', import.meta.url).pathname).text())

// The clock and the timers, moved by hand.
let now: number = inputs.now
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
Date.now = () => now
const kept = new Map<string, string>()
Object.assign(globalThis, {
    setTimeout: setTimer,
    clearTimeout: clearTimer,
    window: { setTimeout: setTimer, clearTimeout: clearTimer, addEventListener: () => {} },
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
        // A timer that throws is reported and the page goes on, as a browser does.
        try {
            next.fn()
        } catch {
            // The run whose step threw stalls.
        }
    }
    now = to
    await settle()
}

const { configureDemo } = await import(`${ROOT}../node_modules/tidevann-shared/backend/index.ts`)
configureDemo({ app: 'studio' })
const model = await import(`${ROOT}model.ts`)
const seed = await import(`${ROOT}seed.ts`)
const { tablesWith } = await import(`${ROOT}tables.ts`)
const { runQuery, SqlError } = await import(`${ROOT}sql.ts`)
const pipeline = await import(`${ROOT}pipeline.ts`)
const edit = await import(`${ROOT}pipeline-edit.ts`)
const { startRun, cancelRun } = await import(`${ROOT}simulate.ts`)
const YAML = await import(`${ROOT}../node_modules/yaml/dist/index.js`)

const query = (source: string, tables: unknown) => {
    try {
        const { columns, rows } = runQuery(source, tables)
        return { columns, rows }
    } catch (error) {
        if (error instanceof SqlError) return { error: error.message, from: error.from, to: error.to }
        return { error: String(error) }
    }
}

// --- YAML, as the pipelines lean on it ---------------------------------------------------------
function node(n: any): any {
    if (n === null || n === undefined) return null
    const out: any = {}
    if (YAML.isScalar(n)) {
        out.v = n.value
        out.type = n.type ?? null
    } else if (YAML.isMap(n)) out.map = n.items.map((pair: any) => [node(pair.key), node(pair.value)])
    else if (YAML.isSeq(n)) out.seq = n.items.map(node)
    out.range = n.range ?? null
    if (n.flow) out.flow = true
    if (n.commentBefore) out.cb = n.commentBefore
    if (n.comment) out.c = n.comment
    if (n.spaceBefore) out.sb = true
    return out
}
const WRITE = { flowCollectionPadding: false, lineWidth: 0 }
const yaml = inputs.yaml.map((text: string) => {
    const doc = YAML.parseDocument(text, { prettyErrors: false })
    if (doc.errors.length > 0) return { text, broken: true }
    return {
        text,
        broken: false,
        js: doc.toJS(),
        tree: { contents: node(doc.contents), cb: doc.commentBefore ?? null, c: doc.comment ?? null },
        written: doc.toString(WRITE),
    }
})

// --- The seeded workspace and the model's words ---------------------------------------------------
const files = seed.seedFiles()
const tables = tablesWith([])
const SQL = [...files.filter((file: any) => file.kind === 'query').map((file: any) => file.text), ...inputs.queries]

// --- Pipelines: read, and edited ------------------------------------------------------------------
const textOf = (one: { file?: string; text?: string }) =>
    one.text ?? files.find((file: any) => file.id === one.file)?.text ?? ''
const pipelineTexts = [...files.filter((file: any) => file.kind === 'pipeline').map((file: any) => file.text), ...inputs.pipelines]
const pipelines = pipelineTexts.map((text: string) => {
    const read = pipeline.readPipeline(text)
    const order = pipeline.stepOrder(read.steps)
    return {
        text,
        read,
        order: order === null ? null : order.map((step: any) => step.id),
        words: pipeline.referenceWords(read),
        described: inputs.words.map((word: string) => pipeline.describeWord(read, word)),
    }
})
const edits = inputs.edits.map((one: any) => {
    const text = textOf(one)
    switch (one.op) {
        case 'addStep':
            return edit.addStep(text, one.kind, { needs: one.needs, before: one.before })
        case 'removeSteps':
            return edit.removeSteps(text, one.ids)
        case 'connect':
            return edit.connect(text, one.source, one.target)
        case 'disconnect':
            return edit.disconnect(text, one.source, one.target)
        case 'renameStep':
            return edit.renameStep(text, one.from, one.to)
        case 'stepFields':
            return edit.stepFields(text, one.id) ?? null
        case 'setStepFields':
            return edit.setStepFields(text, one.id, one.fields)
    }
    throw new Error(`No edit ${one.op}`)
})

// --- Runs, as they go ------------------------------------------------------------------------------
const stored = (): any[] => JSON.parse(localStorage.getItem('tidevann-demo:studio:runs') ?? '[]')
const looks: unknown[] = []
const refusals: unknown[] = []
for (const step of inputs.timeline) {
    await advance(inputs.now + step.at)
    if (step.do === 'start') {
        const file = files.find((one: any) => one.id === step.file)
        const text = step.text ?? file.text
        const number = Math.max(0, ...stored().map((run: any) => run.number)) + 1
        startRun(file, text, number, text !== file.text).catch((error: Error) => refusals.push({ at: step.at, message: error.message }))
        await settle()
    } else if (step.do === 'cancel') {
        const run = stored().find((one: any) => one.number === step.run)
        if (run !== undefined) cancelRun(run)
    } else looks.push({ at: step.at, runs: stored() })
}
const after = tablesWith(stored())

process.stdout.write(
    JSON.stringify({
        files,
        samples: seed.SAMPLES,
        templates: ['pipeline', 'schema', 'query'].map((kind) => seed.template(kind, 'stock-alerts', 'juniper')),
        prefs: model.DEFAULT_PREFS,
        kinds: model.KINDS.map(({ kind, label, plural, extension }: any) => ({ kind, label, plural, extension })),
        stems: files.map((file: any) => model.stem(file)),
        badNames: [['', []], ['  ', []], ['Nightly', []], ['9lives', []], ['a_b', []], ['nightly-export', ['nightly-export']], [' ok-2 ', ['ok']]].map(
            ([name, taken]) => model.badName(name, taken),
        ),
        durations: [0, 0.4, 840, 999.5, 1000, 1250, 2449, 59_949, 59_999, 60_000, 72_400, 119_600, 3_600_000].map(model.sayDuration),
        clocks: [inputs.now, inputs.now + 3_723_456, 0].map(model.clock),
        counted: [
            model.counted(1, 'row'),
            model.counted(0, 'row'),
            model.counted(1234567, 'run'),
            model.counted(2, 'query', 'queries'),
            model.counted(1.5, 'step'),
        ],
        order: [
            { name: 'b.yaml', rank: 1 },
            { name: 'a.yaml' },
            { name: 'C.yaml' },
            { name: 'z.yaml', rank: 0 },
            { name: 'c.yaml' },
            { name: 'm.yaml', rank: 1 },
        ]
            .sort(model.fileOrder)
            .map((file) => file.name),
        tables,
        queries: SQL.map((source: string) => ({ source, ...query(source, tables) })),
        yaml,
        schema: pipeline.PIPELINE_SCHEMA,
        pipelines,
        freeIds: inputs.freeIds.map(([taken, kind]: [string[], string]) => edit.freeId(taken, kind)),
        starting: pipeline.STEP_KINDS.map((kind: string) => edit.STARTING_SETTINGS[kind]),
        edits,
        looks,
        refusals,
        afterRuns: inputs.afterRuns.map((source: string) => ({ source, ...query(source, after) })),
    }),
)
