import { describe, expect, it } from 'vitest'
import { luminanceOf } from '../src/chrome.ts'

describe('luminanceOf', () => {
  it('reads hex and rgb and treats lime as bright', () => {
    expect(luminanceOf('#000000')).toBeCloseTo(0, 5)
    expect(luminanceOf('#ffffff')).toBeCloseTo(1, 5)
    expect(luminanceOf('rgb(11, 16, 13)')).toBeLessThan(0.1)
    expect(luminanceOf('#f4f1ea')).toBeGreaterThan(0.7)
    expect(luminanceOf('nope')).toBeNull()
  })
})
