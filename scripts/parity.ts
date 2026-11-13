/**
 * Parity with a sokkel-ui example: keep the egui port as close to the web example as can be seen.
 *
 *   bun scripts/parity.ts check <example>   what changed in the example since it was ported, and
 *                                           every library part it uses with no egui counterpart
 *   bun scripts/parity.ts web <example>     build the example in .parity/sokkel-ui at SOKKEL_UI_REF
 *                                           and shoot its screens into parity/<example>/web/
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
    /** Clicks by accessible name (`role:name`) or keys (`key:Meta+K`), in order. */
    steps?: string[]
}

interface Manifest {
    example: string
    ported: { commit: string; files: Record<string, string> }
    parts: Record<string, string | null>
    screens: Screen[]
    /** The day the screens are shot on, so relative due dates read the same on both sides. */
    today: string
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
    console.log(`sokkel-ui at ${head}; ported from ${manifest.ported.commit}`)
    console.log(`${imported.size - missing.length - unknown.length} of ${imported.size} parts have an egui counterpart`)
    if (missing.length > 0) console.log(`still without one: ${missing.map((m) => m.replace('@winterop-com/sokkel-ui/', '')).join(', ')}`)
    return problems
}

async function web() {
    sync()
    execFileSync('bun', ['install'], { cwd: clone, stdio: 'ignore' })
    execFileSync('bun', ['run', 'build'], { cwd: join(clone, 'packages', 'core'), stdio: 'ignore' })
    const app = join(clone, 'examples', name, 'web')
    execFileSync('bun', ['install'], { cwd: app, stdio: 'ignore' })
    execFileSync('bun', ['--bun', 'vite', 'build'], { cwd: app, stdio: 'ignore' })
    const port = 6290
    const server = spawn('python3', ['-m', 'http.server', String(port), '--bind', '127.0.0.1', '--directory', join(app, 'dist')], { stdio: 'ignore' })
    await new Promise((done) => setTimeout(done, 600))
    const { chromium } = createRequire(join(clone, 'package.json'))('@playwright/test') as typeof import('@playwright/test')
    const browser = await chromium.launch()
    const out = join(root, 'parity', name, 'web')
    mkdirSync(out, { recursive: true })
    try {
        for (const screen of manifest.screens) {
            const context = await browser.newContext({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1, colorScheme: screen.scheme ?? 'light' })
            const page = await context.newPage()
            await page.clock.install({ time: new Date(`${manifest.today}T09:00:00`) })
            await page.goto(`http://127.0.0.1:${port}/#/${screen.route}`)
            await page.waitForTimeout(800)
            for (const step of screen.steps ?? []) {
                const [kind, ...rest] = step.split(':')
                const what = rest.join(':')
                if (kind === 'key') await page.keyboard.press(what)
                else await page.getByRole(kind as 'button', { name: what, exact: true }).first().click()
                await page.waitForTimeout(400)
            }
            await page.screenshot({ path: join(out, `${screen.name}.png`) })
            await context.close()
            console.log(`shot ${screen.name}`)
        }
    } finally {
        await browser.close()
        server.kill()
    }
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
