/**
 * The resolved tokens from sokkel-design, where they are made now (its `scripts/resolve.ts`, from
 * the sokkel-ui commit it pins): a clone of sokkel-design in .parity/sokkel-design, fetched from
 * GitHub on every run, and its `tokens/tokens.resolved.json` copied into the core crate.
 *
 *   bun scripts/design.ts            copy the resolved tokens in
 *   bun scripts/design.ts --check    fail when the copy is not sokkel-design's
 *
 *   SOKKEL_DESIGN_REF      the ref read: origin/main by default, or a commit to pin
 *   SOKKEL_DESIGN_REMOTE   where to fetch from: the sokkel-design checkout's own origin by default
 *   SOKKEL_DESIGN_OFFLINE  set to skip the fetch and read the clone as it stands
 */
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

const root = resolve(import.meta.dirname, '..')
const clone = join(root, '.parity', 'sokkel-design')
const ref = process.env.SOKKEL_DESIGN_REF ?? 'origin/main'
const target = join(root, 'crates', 'core', 'tokens', 'tokens.resolved.json')
const git = (...args: string[]): string => execFileSync('git', ['-C', clone, ...args], { encoding: 'utf8' })

function remote(): string {
    if (process.env.SOKKEL_DESIGN_REMOTE) return process.env.SOKKEL_DESIGN_REMOTE
    const checkout = resolve(root, process.env.SOKKEL_DESIGN ?? '../sokkel-design')
    return execFileSync('git', ['-C', checkout, 'remote', 'get-url', 'origin'], { encoding: 'utf8' }).trim()
}

if (!existsSync(clone)) {
    mkdirSync(join(root, '.parity'), { recursive: true })
    execFileSync('git', ['clone', '-q', remote(), clone], { stdio: 'inherit' })
}
// read only: nothing is ever pushed from here
git('remote', 'set-url', '--push', 'origin', 'no-push')
if (!process.env.SOKKEL_DESIGN_OFFLINE) {
    try {
        git('fetch', '-q', 'origin', 'main')
    } catch {
        console.error('warning: could not fetch sokkel-design; reading the clone as it was last fetched')
    }
}
git('-c', 'advice.detachedHead=false', 'checkout', '-q', '--force', ref)
const head = git('rev-parse', '--short', 'HEAD').trim()
const resolved = readFileSync(join(clone, 'tokens', 'tokens.resolved.json'), 'utf8')

if (process.argv.includes('--check')) {
    if (!existsSync(target) || readFileSync(target, 'utf8') !== resolved) {
        console.error(`${target} is not sokkel-design's at ${ref} (${head}): run make tokens`)
        process.exit(1)
    }
    console.log(`the resolved tokens are sokkel-design's at ${ref} (${head})`)
} else {
    writeFileSync(target, resolved)
    console.log(`copied the resolved tokens from sokkel-design at ${ref} (${head})`)
}
