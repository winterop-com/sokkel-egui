/**
 * Resolve sokkel-ui's tokens (read from a sokkel-ui checkout, never written there) into
 * crates/core/tokens/tokens.resolved.json, the plain values egui paints with.
 *
 *   bun scripts/resolve.ts            write the resolved tokens
 *   bun scripts/resolve.ts --check    fail when they are not what sokkel-ui's tokens make
 *
 * SOKKEL_UI names the checkout, ../sokkel-ui by default, and SOKKEL_UI_REF the git ref its
 * tokens are read at (origin/main through make, so the tokens follow sokkel-ui's main and never the
 * checkout's work in progress); with no ref the working tree is read. Either way it fails when a token is left
 * unresolved that is not in CSS_ONLY: a value written in a form the evaluator does not know yet,
 * which would otherwise only show as a widget asking for a token that is not there.
 */
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { resolveTokens, unresolved } from './resolve-tokens.ts'

const root = join(import.meta.dirname, '..')
const checkout = resolve(root, process.env.SOKKEL_UI ?? '../sokkel-ui')
const ref = process.env.SOKKEL_UI_REF
const source = join(checkout, 'packages', 'core', 'tokens', 'tokens.json')
const read = (): string =>
    ref ? execFileSync('git', ['-C', checkout, 'show', `${ref}:packages/core/tokens/tokens.json`], { encoding: 'utf8' }) : readFileSync(source, 'utf8')
const target = join(root, 'crates', 'core', 'tokens', 'tokens.resolved.json')

if (!existsSync(source)) {
    console.error(`${source}: no sokkel-ui tokens there; set SOKKEL_UI to a sokkel-ui checkout`)
    process.exit(1)
}
/** Tokens that only mean something to CSS: a scroll bar's two colors, an inset against its container. */
const CSS_ONLY = ['scrollbar', 'density.page-inset']

const tokens = JSON.parse(read())
const result = resolveTokens(tokens)
const unknown = unresolved(tokens, result).filter((key) => !CSS_ONLY.includes(key))
if (unknown.length > 0) {
    console.error(`tokens the resolver cannot settle: ${unknown.join(', ')}; teach scripts/resolve-tokens.ts their form, or list them in CSS_ONLY`)
    process.exit(1)
}
const resolved = `${JSON.stringify(result, null, 4)}\n`

if (process.argv.includes('--check')) {
    if (!existsSync(target) || readFileSync(target, 'utf8') !== resolved) {
        console.error(`${target} is not what ${source} makes: run make tokens`)
        process.exit(1)
    }
    console.log(`the resolved tokens match sokkel-ui${ref ? ` at ${ref}` : ''}`)
} else {
    writeFileSync(target, resolved)
    console.log(`wrote ${target}`)
}
