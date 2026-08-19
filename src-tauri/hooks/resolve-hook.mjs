import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const prefix = process.env.DSH_DESKTOP_PREFIX
const parentURL = prefix ? pathToFileURL(join(prefix, 'package.json')).href : undefined

/**
 * Resolve bare @scope/pkg specifiers from the bundled npm prefix, not from
 * $DSH_HOME/profiles/<name>. Official dsh heals profiles/node_modules with
 * per-package junctions; those can miss on a first Windows launch or when the
 * profile was created by another desktop (shared AppData identifier).
 */
export async function resolve(specifier, context, nextResolve) {
  if (parentURL !== undefined && !specifier.startsWith('.') && !specifier.startsWith('node:') && !specifier.includes(':')) {
    try {
      return await nextResolve(specifier, { ...context, parentURL })
    } catch {
      // Fall through to the importer's own parent walk.
    }
  }
  return nextResolve(specifier, context)
}
