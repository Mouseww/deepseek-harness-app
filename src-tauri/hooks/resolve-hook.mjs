import { existsSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const prefix = process.env.DSH_DESKTOP_PREFIX
const home = process.env.DSH_HOME
const prefixParent = prefix ? pathToFileURL(join(prefix, 'package.json')).href : undefined
const profileParents = home
  ? ['web', 'desktop']
      .map((profile) => join(home, 'profiles', profile, 'package.json'))
      .filter((manifest) => existsSync(manifest))
      .map((manifest) => pathToFileURL(manifest).href)
  : []

function isBareSpecifier(specifier) {
  return !specifier.startsWith('.') && !specifier.startsWith('node:') && !specifier.includes(':')
}

/**
 * Resolve user-profile plugins from $DSH_HOME/profiles/<name> first, then fall
 * back to the bundled npm prefix. Official dsh heals profiles/node_modules with
 * per-package junctions; those can miss on a first Windows launch, so the
 * bundled prefix remains a fallback for @deepseek-ai/* packages.
 */
export async function resolve(specifier, context, nextResolve) {
  if (isBareSpecifier(specifier)) {
    for (const parentURL of profileParents) {
      try {
        return await nextResolve(specifier, { ...context, parentURL })
      } catch {
        // Try the next profile, then the bundled prefix.
      }
    }
    if (prefixParent !== undefined) {
      try {
        return await nextResolve(specifier, { ...context, parentURL: prefixParent })
      } catch {
        // Fall through to the importer's own parent walk.
      }
    }
  }
  return nextResolve(specifier, context)
}
