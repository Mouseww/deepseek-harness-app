import { describe, expect, it } from 'vitest'
import {
  DEFAULT_CONFIG,
  parseConfig,
  parseHost,
  parsePort,
  webUrl,
} from '../src/config.ts'

describe('parsePort', () => {
  it('accepts 0 only for local launch', () => {
    expect(parsePort(0, 'local')).toBe(0)
    expect(parsePort('3080', 'connect')).toBe(3080)
    expect(() => parsePort(0, 'connect')).toThrow(/nonzero port/)
    expect(() => parsePort(1.5, 'local')).toThrow(/integer/)
    expect(() => parsePort(70_000, 'local')).toThrow(/65535/)
  })
})

describe('parseHost', () => {
  it('rejects empty, spaced, schemed, and all-interfaces local binds', () => {
    expect(parseHost('127.0.0.1', 'local')).toBe('127.0.0.1')
    expect(parseHost(' 192.168.1.8 ', 'connect')).toBe('192.168.1.8')
    expect(() => parseHost('', 'local')).toThrow(/empty/)
    expect(() => parseHost('127.0.0.1:3080', 'local')).toThrow(/without a scheme or port/)
    expect(() => parseHost('http://127.0.0.1', 'local')).toThrow(/without a scheme or port/)
    expect(() => parseHost('0.0.0.0', 'local')).toThrow(/0\.0\.0\.0/)
    expect(parseHost('0.0.0.0', 'connect')).toBe('0.0.0.0')
  })
})

describe('parseConfig', () => {
  it('normalizes a complete form', () => {
    expect(parseConfig({
      host: '127.0.0.1',
      port: '3080',
      autoStart: false,
      launchMode: 'local',
    })).toEqual({
      host: '127.0.0.1',
      port: 3080,
      autoStart: false,
      launchMode: 'local',
    })
    expect(() => parseConfig({
      ...DEFAULT_CONFIG,
      launchMode: 'other',
    })).toThrow(/launchMode/)
  })
})

describe('webUrl', () => {
  it('builds an http URL and refuses port 0', () => {
    expect(webUrl({ host: '127.0.0.1', port: 3080 })).toBe('http://127.0.0.1:3080/')
    expect(() => webUrl({ host: '127.0.0.1', port: 0 })).toThrow(/assigned port/)
  })
})
