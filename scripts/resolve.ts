/**
 * Resolve sokkel-ui's tokens into crates/core/tokens/tokens.resolved.json, the plain values egui
 * paints with, reading tokens.json from sokkel-ui as GitHub has it (see scripts/sokkel.ts).
 *
 *   bun scripts/resolve.ts            write the resolved tokens
 *   bun scripts/resolve.ts --check    fail when they are not what sokkel-ui's tokens make
 *
 * Either way it fails when a token is left unresolved that is not in CSS_ONLY: a value written in
 * a form the evaluator does not know yet, which would otherwise only show as a widget asking for a
 * token that is not there.
 */
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { resolveTokens, unresolved } from './resolve-tokens.ts'
import { ref, root, show, sync } from './sokkel.ts'

const target = join(root, 'crates', 'core', 'tokens', 'tokens.resolved.json')
const head = sync()
const read = (): string => show('packages/core/tokens/tokens.json')

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
        console.error(`${target} is not what sokkel-ui's tokens make at ${ref} (${head}): run make tokens`)
        process.exit(1)
    }
    console.log(`the resolved tokens match sokkel-ui at ${ref} (${head})`)
} else {
    writeFileSync(target, resolved)
    console.log(`wrote ${target} from sokkel-ui at ${ref} (${head})`)
}
