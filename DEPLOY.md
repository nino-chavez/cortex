# Deploy — cortex

## Host
- **Platform**: Tauri desktop app (NOT a web deploy) — also SvelteKit static via adapter-static
- **Distribution**: TODO — Tauri produces native installers (dmg/msi/AppImage)

## Deploy trigger
- **Not a web deploy.** Build with `npm run tauri build` (TODO confirm script)
- Static SvelteKit output can be hosted anywhere if web mode is wanted separately

## Database
- TODO — Tauri apps usually have local SQLite via `@tauri-apps/api`

## Environment variables
- TODO

## Preflight checks
- `npm run check` passes
- Tauri build succeeds on target platform

## Verify after deploy
- Install + run the produced bundle on a target OS

## Authority limits
- Cannot codesign without Apple/MS certs
- Cannot notarize on macOS without Apple ID + app-specific password

## Notes
- Tauri 2 desktop app
- adapter-static for the embedded UI
- Status: WIP — not yet shipping
