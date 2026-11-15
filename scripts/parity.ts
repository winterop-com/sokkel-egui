/**
 * Parity with a sokkel-ui example: keep the egui port as close to the web example as can be seen.
 *
 *   bun scripts/parity.ts check <example>   what changed in the example since it was ported, the
 *                                           library files it imports (and the tokens and styles)
 *                                           changed since its web screens were shot, and every
 *                                           library part it uses with no egui counterpart
 *   bun scripts/parity.ts web <example>     build the example in .parity/sokkel-ui at SOKKEL_UI_REF
 *                                           and shoot its screens into parity/<example>/web/,
 *                                           keeping a picture that differs only by antialiasing
 *                                           (PARITY_SCALE=2 shoots at a Retina screen's scale into
 *                                           parity/<example>/web@2x/, for comparing how text is
 *                                           drawn; PARITY_ONLY=a,b shoots only those screens)
 *   bun scripts/parity.ts board <example>   a page of each screen, web beside egui, in .parity/
 *
 * The manifest, parity/<example>.json, records the sokkel-ui commit each file of the example was
 * ported from, every library module it imports with the egui part that stands for it (null while
 * none does), and the screens to compare: a route and steps, at 1280 by 800 on a fixed day.
 * sokkel-ui is read from .parity/sokkel-ui, fetched from GitHub on every run (scripts/sokkel.ts).
 */
import { execFileSync, spawn } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'

import { clone, git, root, sync } from './sokkel.ts'

interface Screen {
    name: string
    route: string
    scheme?: 'light' | 'dark'
    /** The window, 1280 by 800 unless given: a phone is 390 by 844. */
    viewport?: { width: number; height: number }
    /**
     * In order: clicks by accessible name (`role:name`), keys (`key:Meta+K`), and double clicks on
     * visible text (`dblclick:text`) or on what a selector finds (`dblclick-css:selector`).
     */
    steps?: string[]
}

interface Manifest {
    example: string
    ported: { commit: string; files: Record<string, string> }
    /** The sokkel-ui commit the web screens were last shot at. */
    shot?: string
    parts: Record<string, string | null>
    screens: Screen[]
    /** The day the screens are shot on, so relative due dates read the same on both sides. */
    today: string
    /**
     * A live example (one whose page ticks on timers): its clock is paused, and the shot is taken
     * after the same span of its time, so every shot catches it at the same tick.
     */
    live?: boolean
}

const [command, name = 'tasks'] = process.argv.slice(2)
const manifestPath = join(root, 'parity', `${name}.json`)
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8')) as Manifest

/** Every file of the example at HEAD, by path inside the example, with its blob. */
function files(): Record<string, string> {
    const out: Record<string, string> = {}
    for (const line of git('ls-tree', '-r', 'HEAD', '--', manifest.example).trim().split('\n')) {
        const [meta, path] = line.split('\t')
        out[path.slice(manifest.example.length + 1)] = meta.split(' ')[2]
    }
    return out
}

function check(): number {
    const head = sync()
    let problems = 0
    const now = files()
    for (const [path, blob] of Object.entries(now)) {
        const was = manifest.ported.files[path]
        if (was === undefined) {
            console.log(`new file since ${manifest.ported.commit}: ${path}`)
            problems++
        } else if (was !== blob) {
            console.log(`changed since ${manifest.ported.commit}: ${path}`)
            console.log(`    git -C .parity/sokkel-ui diff ${manifest.ported.commit} HEAD -- ${manifest.example}/${path}`)
            problems++
        }
    }
    for (const path of Object.keys(manifest.ported.files)) {
        if (!(path in now)) {
            console.log(`removed since ${manifest.ported.commit}: ${path}`)
            problems++
        }
    }
    const imported = new Set<string>()
    for (const path of Object.keys(now).filter((p) => /\.tsx?$/.test(p))) {
        const source = git('show', `HEAD:${manifest.example}/${path}`)
        for (const match of source.matchAll(/from '(@winterop-com\/[^']+)'/g)) imported.add(match[1])
    }
    const missing = [...imported].filter((module) => manifest.parts[module] === null)
    const unknown = [...imported].filter((module) => !(module in manifest.parts))
    for (const module of unknown) {
        console.log(`imported, not in the manifest: ${module}`)
        problems++
    }
    // The library moves under an example that does not: the files it imports, and the tokens and
    // styles every part wears, changed since the web screens were shot.
    if (manifest.shot !== undefined) {
        const exports = (JSON.parse(git('show', 'HEAD:packages/core/package.json')) as { exports: Record<string, { import?: string }> }).exports
        const sources = new Set<string>()
        for (const module of imported) {
            const target = exports[`.${module.slice('@winterop-com/sokkel-ui'.length)}`]?.import
            if (target !== undefined) sources.add(target.replace(/^\.\/dist\//, 'packages/core/src/').replace(/\.js$/, ''))
        }
        const changed = git('diff', '--name-only', manifest.shot, 'HEAD', '--', 'packages/core/src', 'packages/core/tokens')
            .trim()
            .split('\n')
            .filter((path) => path !== '' && !/\.(test|stories)\.tsx?$/.test(path))
            .filter((path) => sources.has(path.replace(/\.tsx?$/, '')) || /\/(tokens|styles)\//.test(path))
        if (changed.length > 0) {
            console.log(`library changed since the web was shot at ${manifest.shot.slice(0, 8)} (make parity-web, then look):`)
            for (const path of changed) console.log(`    ${path}`)
        }
    }
    console.log(`sokkel-ui at ${head}; ported from ${manifest.ported.commit}`)
    console.log(`${imported.size - missing.length - unknown.length} of ${imported.size} parts have an egui counterpart`)
    if (missing.length > 0) console.log(`still without one: ${missing.map((m) => m.replace('@winterop-com/sokkel-ui/', '')).join(', ')}`)
    return problems
}

async function web() {
    sync()
    execFileSync('bun', ['install'], { cwd: clone, stdio: 'ignore' })
    const app = join(clone, 'examples', name, 'web')
    // the library's packages the example depends on, the core first, built as an app installs them
    const needs = Object.values(
        (JSON.parse(readFileSync(join(app, 'package.json'), 'utf8')) as { dependencies: Record<string, string> }).dependencies,
    )
        .map((spec) => /^file:\.\.\/\.\.\/\.\.\/packages\/(.+)$/.exec(spec)?.[1])
        .filter((one): one is string => one !== undefined)
        .sort((a, b) => (a === 'core' ? -1 : b === 'core' ? 1 : a.localeCompare(b)))
    for (const one of needs) execFileSync('bun', ['run', 'build'], { cwd: join(clone, 'packages', one), stdio: 'ignore' })
    execFileSync('bun', ['install'], { cwd: app, stdio: 'ignore' })
    execFileSync('bun', ['--bun', 'vite', 'build'], { cwd: app, stdio: 'ignore' })
    const port = 6290
    const server = spawn('python3', ['-m', 'http.server', String(port), '--bind', '127.0.0.1', '--directory', join(app, 'dist')], { stdio: 'ignore' })
    await new Promise((done) => setTimeout(done, 600))
    const { chromium } = createRequire(join(clone, 'package.json'))('@playwright/test') as typeof import('@playwright/test')
    const browser = await chromium.launch()
    const scale = Number(process.env.PARITY_SCALE ?? '1')
    const only = process.env.PARITY_ONLY?.split(',')
    const out = join(root, 'parity', name, scale === 1 ? 'web' : `web@${String(scale)}x`)
    mkdirSync(out, { recursive: true })
    try {
        for (const screen of manifest.screens.filter((one) => only === undefined || only.includes(one.name))) {
            const context = await browser.newContext({
                viewport: screen.viewport ?? { width: 1280, height: 800 },
                deviceScaleFactor: scale,
                colorScheme: screen.scheme ?? 'light',
                // egui's clock is UTC: a page printing times in the viewer's zone prints them so too
                timezoneId: 'UTC',
            })
            const page = await context.newPage()
            await page.clock.install({ time: new Date(`${manifest.today}T09:00:00Z`) })
            // a live example draws from Math.random: seeded, it draws the same on every shot
            await page.addInitScript(() => {
                let seed = 20261116
                Math.random = () => {
                    seed = (seed + 0x6d2b79f5) | 0
                    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed)
                    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
                    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
                }
            })
            if (manifest.live === true) await page.clock.pauseAt(new Date(`${manifest.today}T09:00:00Z`))
            await page.goto(`http://127.0.0.1:${port}/#/${screen.route}`)
            await page.waitForTimeout(800)
            // a live page runs the same 5 s of its own time on every shot, then settles
            if (manifest.live === true) {
                await page.clock.runFor(5000)
                await page.waitForTimeout(300)
            }
            for (const step of screen.steps ?? []) {
                const [kind, ...rest] = step.split(':')
                const what = rest.join(':')
                if (kind === 'key') await page.keyboard.press(what)
                else if (kind === 'dblclick-css') await page.locator(what).locator('visible=true').first().dblclick()
                else if (kind === 'dblclick') await page.getByText(what, { exact: true }).locator('visible=true').first().dblclick()
                else await page.getByRole(kind as 'button', { name: what, exact: true }).first().click()
                await page.waitForTimeout(400)
            }
            const shot = await page.screenshot()
            const path = join(out, `${screen.name}.png`)
            const kept = existsSync(path) && (await same(page, readFileSync(path), shot))
            if (!kept) writeFileSync(path, shot)
            await context.close()
            console.log(`${kept ? 'same' : 'shot'} ${screen.name}`)
        }
        // the screens the board compares are the 1x ones, so only they move the shot's commit
        if (scale === 1 && only === undefined) {
            manifest.shot = git('rev-parse', 'HEAD').trim()
            writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 4)}\n`)
        }
    } finally {
        await browser.close()
        server.kill()
    }
}

/** Whether two pictures differ by no more than antialiasing: no channel of any pixel by more than 2. */
async function same(page: import('@playwright/test').Page, a: Buffer, b: Buffer): Promise<boolean> {
    return page.evaluate(
        async ([one, two]) => {
            const pixels = async (data: string) => {
                const image = await createImageBitmap(await (await fetch(`data:image/png;base64,${data}`)).blob())
                const canvas = new OffscreenCanvas(image.width, image.height)
                const context = canvas.getContext('2d')
                if (context === null) return null
                context.drawImage(image, 0, 0)
                return context.getImageData(0, 0, image.width, image.height)
            }
            const [x, y] = [await pixels(one), await pixels(two)]
            if (x === null || y === null || x.width !== y.width || x.height !== y.height) return false
            for (let i = 0; i < x.data.length; i++) if (Math.abs(x.data[i] - y.data[i]) > 2) return false
            return true
        },
        [a.toString('base64'), b.toString('base64')],
    )
}

function board() {
    const egui = join(root, 'examples', name, 'tests', 'snapshots')
    const web = join(root, 'parity', name, 'web')
    const rows = manifest.screens
        .map((screen) => {
            const side = (dir: string, label: string) =>
                existsSync(join(dir, `${screen.name}.png`)) ? `<figure><img src="file://${join(dir, `${screen.name}.png`)}"><figcaption>${label}</figcaption></figure>` : `<figure class="none">${label}: not shot</figure>`
            return `<h2>${screen.name}</h2><div class="pair">${side(web, 'web')}${side(egui, 'egui')}</div>`
        })
        .join('\n')
    const page = `<!doctype html><meta charset="utf-8"><title>${name}: web and egui</title><style>body{font:14px system-ui;margin:24px;background:#f4f5f7}.pair{display:grid;grid-template-columns:1fr 1fr;gap:12px}img{width:100%;border:1px solid #ccd}figure{margin:0}.none{padding:40px;color:#888;border:1px dashed #ccd}</style><h1>${name}: web and egui, on ${manifest.today}</h1>${rows}`
    const path = join(root, '.parity', `${name}.html`)
    writeFileSync(path, page)
    console.log(path)
}

if (command === 'check') process.exit(check() > 0 ? 1 : 0)
else if (command === 'web') await web()
else if (command === 'board') board()
else {
    console.error('bun scripts/parity.ts check|web|board <example>')
    process.exit(2)
}
