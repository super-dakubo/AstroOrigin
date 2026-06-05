# Contributing to AstroOrigin

## Setup

- Prerequisites: Rust toolchain, Node.js 18+, pnpm
- `pnpm install && pnpm tauri dev`

## Code Quality

- TypeScript: `npx tsc --noEmit`
- Rust: `cd src-tauri && cargo check`
- Format: `pnpm format` (Prettier)
- Tests: `pnpm test` (Vitest) + `cd src-tauri && cargo test`

## Commit Convention

Conventional Commits: `feat:`, `fix:`, `refactor:`, `chore:`, `docs:`
- Subject ≤50 chars, body wrapped at 72 chars

## PR Process

1. Branch from main
2. Make focused commits
3. Run checks before opening PR
4. Keep PRs small and focused
