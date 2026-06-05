import { describe, it, expect } from 'vitest'
import { THEMES, ROUTES } from './constants'

describe('THEMES', () => {
  it('has both genshin and starrail themes', () => {
    expect(THEMES.genshin).toBeDefined()
    expect(THEMES.starrail).toBeDefined()
  })

  it('has required theme properties', () => {
    const required = ['primary', 'gold', 'bg', 'barGradient', 'appName']
    for (const key of required) {
      expect(THEMES.genshin).toHaveProperty(key)
      expect(THEMES.starrail).toHaveProperty(key)
    }
  })

  it('has non-empty app names', () => {
    expect(THEMES.genshin.appName).toBeTruthy()
    expect(THEMES.starrail.appName).toBeTruthy()
  })
})

describe('ROUTES', () => {
  it('defines all four page routes', () => {
    expect(ROUTES.OVERVIEW).toBe('/')
    expect(ROUTES.GACHA).toBe('/gacha')
    expect(ROUTES.PLAYTIME).toBe('/playtime')
    expect(ROUTES.SCREENSHOTS).toBe('/screenshots')
  })
})
