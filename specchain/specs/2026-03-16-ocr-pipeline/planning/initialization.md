# OCR Pipeline — Initialization

**Date:** 2026-03-16
**Roadmap Item:** #2
**Status:** initialized

## Raw Description

OCR Pipeline — Process each screenshot through Apple Vision framework (via swift-rs bridge) to extract all visible text. Store extracted text in SQLite with FTS5 full-text indexing. Support keyword search across captured text with app and time filters.
