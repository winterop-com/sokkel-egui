/**
 * Check the resolved colors against a browser: draw sokkel-ui's own generated CSS (tokens.css and
 * every theme file) in Chromium, read back what each color token paints into an sRGB canvas, and
 * compare it with crates/core/tokens/tokens.resolved.json, every theme in light and dark.
 *
 *   bun scripts/verify-colors.ts      (make verify-colors)
 *
 * Playwright comes from the sokkel-ui checkout's own install (SOKKEL_UI, ../sokkel-ui by default),
 * read and never written. A token only the utility classes name (`good-ink`, an alias of a status
 * ink) has no variable of its own on the page, so only the tokens a theme sets are compared.
 */
import { execFileSync } from 'node:child_process'
import { readdirSync, readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

const root = join(import.meta.dirname, '..')
const sokkel = resolve(root, process.env.SOKKEL_UI ?? '../sokkel-ui')
const core = join(sokkel, 'packages', 'core')
const { chromium } = createRequire(join(sokkel, 'package.json'))('@playwright/test') as typeof import('@playwright/test')

// sokkel-ui's files at SOKKEL_UI_REF when one is given, as scripts/resolve.ts reads them
const ref = process.env.SOKKEL_UI_REF
const read = (path: string): string =>
    ref ? execFileSync('git', ['-C', sokkel, 'show', `${ref}:packages/core/${path}`], { encoding: 'utf8' }) : readFileSync(join(core, path), 'utf8')
const themeFiles = (): string[] =>
    ref
        ? execFileSync('git', ['-C', sokkel, 'ls-tree', '--name-only', `${ref}:packages/core/src/styles/themes`], { encoding: 'utf8' }).split('\n').filter(Boolean)
        : readdirSync(join(core, 'src', 'styles', 'themes'))
const tokens = JSON.parse(read('tokens/tokens.json'))
const resolved = JSON.parse(readFileSync(join(root, 'crates', 'core', 'tokens', 'tokens.resolved.json'), 'utf8'))
let css = read('src/styles/tokens.css')
for (const file of themeFiles()) {
    css += read(`src/styles/themes/${file}`)
}

/** The most a channel may differ, out of 255: rounding on either side. */
const TOLERANCE = 2

const browser = await chromium.launch()
const page = await browser.newPage()
await page.setContent(`<style>${css}</style><div id="probe"></div><canvas id="canvas" width="1" height="1"></canvas>`)
let compared = 0
const wrong: string[] = []
for (const [name, theme] of Object.entries<Record<string, { colors: Record<string, string> }>>(resolved.themes)) {
    for (const scheme of ['light', 'dark']) {
        const colors = theme[scheme].colors
        const keys = Object.keys(colors).filter((key) => key in tokens.themes.tidevann.light)
        const painted: Record<string, number[]> = await page.evaluate(
            ({ name, scheme, keys }) => {
                const html = document.documentElement
                html.classList.toggle('dark', scheme === 'dark')
                if (name === 'tidevann') html.removeAttribute('data-theme')
                else html.setAttribute('data-theme', name)
                const probe = document.getElementById('probe') as HTMLElement
                const canvas = document.getElementById('canvas') as HTMLCanvasElement
                const context = canvas.getContext('2d', { willReadFrequently: true }) as CanvasRenderingContext2D
                const out: Record<string, number[]> = {}
                for (const key of keys) {
                    probe.style.backgroundColor = `var(--${key})`
                    context.clearRect(0, 0, 1, 1)
                    context.fillStyle = getComputedStyle(probe).backgroundColor
                    context.fillRect(0, 0, 1, 1)
                    out[key] = [...context.getImageData(0, 0, 1, 1).data]
                }
                return out
            },
            { name, scheme, keys },
        )
        for (const key of keys) {
            const hex = colors[key]
            const ours = [1, 3, 5, 7].map((i) => (i < hex.length ? parseInt(hex.slice(i, i + 2), 16) : 255))
            const theirs = painted[key]
            // a nearly transparent color's channels mean little; compare its alpha alone
            const channels = ours[3] < 40 ? [3] : [0, 1, 2, 3]
            const off = Math.max(...channels.map((i) => Math.abs(ours[i] - theirs[i])))
            compared++
            if (off > TOLERANCE) wrong.push(`${name} ${scheme} ${key}: resolved ${hex}, browser rgba(${theirs.join(', ')})`)
        }
    }
}
await browser.close()
if (wrong.length > 0) {
    console.error(`${wrong.length} of ${compared} colors differ from the browser:\n${wrong.join('\n')}`)
    process.exit(1)
}
console.log(`all ${compared} colors match the browser within ${TOLERANCE}/255`)
