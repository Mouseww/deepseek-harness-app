import { describe, expect, it } from 'vitest'
import { parseReadyLine, webLaunchArgs } from '../src/ready.ts'

describe('parseReadyLine', () => {
  it('reads the loopback URL and optional LAN URL', () => {
    expect(parseReadyLine('dsh web: http://127.0.0.1:4567')).toEqual({
      url: 'http://127.0.0.1:4567',
    })
    expect(parseReadyLine('dsh web: http://127.0.0.1:4567 (LAN: http://192.168.1.5:4567)')).toEqual({
      url: 'http://127.0.0.1:4567',
      lanUrl: 'http://192.168.1.5:4567',
    })
    expect(parseReadyLine('waiting for bind')).toBeUndefined()
    expect(parseReadyLine('dsh web: not-a-url')).toBeUndefined()
  })
})

describe('webLaunchArgs', () => {
  it('passes host and port as dsh web flags', () => {
    expect(webLaunchArgs('127.0.0.1', 0)).toEqual([
      '--host',
      '127.0.0.1',
      '--port',
      '0',
    ])
  })
})
