/**
 * Resolve sokkel-ui's tokens (read from a sokkel-ui checkout, never written there) into
 * crates/core/tokens/tokens.resolved.json, the plain values egui paints with.
 *
 *   bun scripts/resolve.ts            write the resolved tokens
 *   bun scripts/resolve.ts --check    fail when they are not what sokkel-ui's tokens make
 *
 * SOKKEL_UI names the checkout, ../sokkel-ui by default. Either way it fails when a token is left
 * unresolved that is not in CSS_ONLY: a value written in a form the evaluator does not know yet,
 * which would otherwise only show as a widget asking for a token that is not there.
 */
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { resolveTokens, unresolved } from './resolve-tokens.ts'

const root = join(import.meta.dirname, '..')
const source = join(resolve(root, process.env.SOKKEL_UI ?? '../sokkel-ui'), 'packages', 'core', 'tokens', 'tokens.json')
const target = join(root, 'crates', 'core', 'tokens', 'tokens.resolved.json')

if (!existsSync(source)) {
    console.error(`${source}: no sokkel-ui tokens there; set SOKKEL_UI to a sokkel-ui checkout`)
    process.exit(1)
}
/** Tokens that only mean something to CSS: a scroll bar's two colors, an inset against its container. */
const CSS_ONLY = ['scrollbar', 'density.page-inset']

const tokens = JSON.parse(readFileSync(source, 'utf8'))
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
    console.log('the resolved tokens match sokkel-ui')
} else {
    writeFileSync(target, resolved)
    console.log(`wrote ${target}`)
}
