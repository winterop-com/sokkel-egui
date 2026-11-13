/**
 * sokkel-ui as the scripts read it: a clone of its own in .parity/sokkel-ui, fetched from GitHub on
 * every run, so a check never trusts a checkout someone last fetched days ago, and never touches
 * the sokkel-ui checkout another session works in.
 *
 *   SOKKEL_UI_REF      the ref read: origin/main by default, or a commit to pin
 *   SOKKEL_UI_REMOTE   where to fetch from: the sokkel-ui checkout's own origin by default
 *   SOKKEL_UI_OFFLINE  set to skip the fetch and read the clone as it stands
 */
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync } from 'node:fs'
import { join, resolve } from 'node:path'

export const root = resolve(import.meta.dirname, '..')
export const clone = join(root, '.parity', 'sokkel-ui')
export const ref = process.env.SOKKEL_UI_REF ?? 'origin/main'

export const git = (...args: string[]): string => execFileSync('git', ['-C', clone, ...args], { encoding: 'utf8' })

/** The remote: the sokkel-ui checkout's origin (its GitHub repository), unless told otherwise. */
function remote(): string {
    if (process.env.SOKKEL_UI_REMOTE) return process.env.SOKKEL_UI_REMOTE
    const checkout = resolve(root, process.env.SOKKEL_UI ?? '../sokkel-ui')
    return execFileSync('git', ['-C', checkout, 'remote', 'get-url', 'origin'], { encoding: 'utf8' }).trim()
}

/** Bring the clone up to date with GitHub and to `ref`; returns the short commit read. */
export function sync(): string {
    if (!existsSync(clone)) {
        mkdirSync(join(root, '.parity'), { recursive: true })
        execFileSync('git', ['clone', '-q', remote(), clone], { stdio: 'inherit' })
    } else {
        git('remote', 'set-url', 'origin', remote())
    }
    // read only: nothing is ever pushed from here
    git('remote', 'set-url', '--push', 'origin', 'no-push')
    if (!process.env.SOKKEL_UI_OFFLINE) {
        try {
            git('fetch', '-q', 'origin', 'main')
        } catch {
            console.error('warning: could not fetch sokkel-ui; reading the clone as it was last fetched')
        }
    }
    git('-c', 'advice.detachedHead=false', 'checkout', '-q', '--force', ref)
    return git('rev-parse', '--short', 'HEAD').trim()
}

/** A file of sokkel-ui at the synced commit. */
export const show = (path: string): string => git('show', `HEAD:${path}`)
