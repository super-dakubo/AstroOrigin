# Gacha 页面面板/列表布局 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development or executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Redesign Gacha page with segmented control switching between chart panel and data table list

**Architecture:** Pure frontend change — add `viewMode` state, segmented control UI, and conditional rendering. No backend changes. No new dependencies.

**Tech Stack:** React 18, TypeScript, Tailwind CSS (same as existing)

---

### Task 1: Update `generatePageNumbers` — show ±2 pages

**Files:**
- Modify: `frontend/components/RecordTable.tsx:4-16`

Current logic: at most 7 pages, shows immediately adjacent pages only (±1).

New logic: always show first + last page, show current ±2, use ellipsis.

- [ ] **Step 1: Rewrite generatePageNumbers**

```typescript
function generatePageNumbers(current: number, total: number): (number | '...')[] {
  if (total <= 7) {
    return Array.from({ length: total }, (_, i) => i + 1)
  }
  const pages: (number | '...')[] = [1]

  if (current - 2 > 2) pages.push('...')

  const start = Math.max(2, current - 2)
  const end = Math.min(total - 1, current + 2)
  for (let i = start; i <= end; i++) pages.push(i)

  if (current + 2 < total - 1) pages.push('...')

  if (total > 1) pages.push(total)
  return pages
}
```

### Task 2: Add viewMode state and segmented control to Gacha page

**Files:**
- Modify: `frontend/pages/Gacha.tsx`

- [ ] **Step 1: Add viewMode state after existing useState declarations**

Insert after `const [pageSize, setPageSize] = useState<number>(20)`:

```typescript
type ViewMode = 'panel' | 'list'
const [viewMode, setViewMode] = useState<ViewMode>('panel')
```

- [ ] **Step 2: Import `useMemo` if not already present**

Line 8 should include `useMemo`:
```typescript
import { useState, useEffect, useRef, useMemo } from 'react'
```

- [ ] **Step 3: Add segmented control after stat cards grid**

Insert between the stat cards grid (line ~370) and the import progress section:

```tsx
      {/* 面板/列表切换 */}
      <div className="flex justify-center">
        <div className="inline-flex bg-gray-100 rounded-lg p-0.5">
          <button
            onClick={() => setViewMode('panel')}
            className={`px-6 py-1.5 text-sm rounded-md transition-colors ${
              viewMode === 'panel'
                ? 'bg-white shadow-sm font-medium text-gray-900'
                : 'text-gray-500 hover:text-gray-700'
            }`}
          >
            📊 面板
          </button>
          <button
            onClick={() => setViewMode('list')}
            className={`px-6 py-1.5 text-sm rounded-md transition-colors ${
              viewMode === 'list'
                ? 'bg-white shadow-sm font-medium text-gray-900'
                : 'text-gray-500 hover:text-gray-700'
            }`}
          >
            📋 列表
          </button>
        </div>
      </div>
```

- [ ] **Step 4: Replace the existing `<LuckChart>` + filter bar + `<RecordTable>` with conditional rendering**

Find and replace this block (currently contains LuckChart, filter bar div, and RecordTable):

Old block (approximate):
```
      <LuckChart records={chartData} />
      <RecordTable ... />
```

Replace with:
```tsx
      {viewMode === 'panel' ? (
        <LuckChart records={chartData} />
      ) : (
        <>
          {/* 筛选栏（现有内容） */}
          <div className="flex gap-4 items-center flex-wrap">
            ...
          </div>
          <RecordTable ... />
        </>
      )}
```

Note: The filter bar `<div>` and `<RecordTable>` move into the `列表` branch. The `<LuckChart>` moves into the `面板` branch. Leave all existing import, state, progress, error toast code untouched.

### Task 3: Verify compilation

- [ ] **Step 1: TypeScript check**

```bash
npx tsc --noEmit
```
Expected: No errors.

- [ ] **Step 2: Visual smoke test**

```bash
pnpm tauri dev
```
Expected: Gacha page shows segmented control. Clicking 面板/列表 toggles between chart and data table view.
