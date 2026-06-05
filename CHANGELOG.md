# Changelog

## [0.1.0] - 2026-06-05

### Added
- Gacha screenshot OCR import (PP-OCRv4)
- Gacha records CRUD with pagination
- Per-banner stats and filtering
- LuckChart (欧非曲线)
- Game switch (Genshin / StarRail)
- Panel/List view toggle on Gacha page
- Playtime tracking
- Screenshot tagging

### Fixed
- useECharts chart flicker on every render
- OCR engine Mutex poisoning recovery
- Database pool connection timeout
- WCAG contrast on import buttons
- RecordTable semantics for screen readers
- Shell security permission removed
- README and project docs

### Changed
- echarts tree-shaken (saves ~400KB)
- OCR decode once instead of twice
