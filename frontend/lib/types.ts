export interface GachaRecord {
  id: number
  gameKind: string
  itemName: string
  itemType: string
  bannerType: string
  starRating: number
  recordDate: string
  isWon: boolean
}

export interface GameConfig {
  logDirs: string[]
  apiUrl: string
  extraParams: string
  gachaTypes: Record<string, string>
}

export interface GachaConfig {
  games: Record<string, GameConfig>
}
