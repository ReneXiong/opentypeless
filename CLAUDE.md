# CLAUDE.md

## Search Preference

When searching for information, use MCP services (e.g., `mcp__tavily-remote-mcp__tavily_search`) instead of built-in `WebFetch`. MCP search provides better results and more comprehensive coverage.

## Project Overview

OpenTypeless is a Tauri 2 desktop app (Rust backend + React/TypeScript frontend) for voice-to-text input with AI polishing.

## Key Architecture

- **Backend**: Rust (src-tauri/src/)
- **Frontend**: React + TypeScript (src/)
- **State Management**: Zustand (src/stores/)
- **i18n**: react-i18next (src/i18n/)
- **UI**: Tailwind CSS + Framer Motion

## Common Commands

```bash
# Frontend dev
npm run dev

# Tauri dev (full app)
npx tauri dev

# Frontend build check
npm run build

# Rust check
cd src-tauri && cargo check

# Run tests
npm test
```
