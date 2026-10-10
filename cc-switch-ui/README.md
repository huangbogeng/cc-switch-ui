# cc-switch-ui

React frontend for **CC Switch Web**.

[User documentation](../docs/README.md) · [Development guide](../docs/development.md) · [Design conventions](../docs/frontend-design.md)

Use Node.js 24.15+ and the committed npm lockfile. The UI is English; user-defined provider names and content keep their original language.

## Stack

- React 19
- TypeScript
- Vite
- Tailwind CSS v3
- Radix UI primitives
- lucide-react icons

## Development

```bash
npm ci
npm run dev
```

The Vite dev server proxies `/api` to `http://localhost:5007`, so run the backend from the repository root in another terminal:

```bash
cargo run --bin cc-switch-server
```

## Build

```bash
npm run lint
npm test
npm run build
```

The production build is written to `dist/`. The Rust Web server serves this directory at `/ui`.

## Structure

```text
src/
├── api/                  # API client and shared frontend API types
├── components/
│   ├── dashboard/        # Dashboard display panels
│   ├── providers/        # Provider list, preset selector, provider form dialog
│   └── ui/               # Shared low-level UI primitives
├── config/               # Provider presets
├── lib/                  # Small reusable helpers
├── pages/                # Stateful page orchestration
├── App.tsx               # Auth gate and app shell
└── main.tsx              # React entrypoint
```

## Tailwind

This project uses Tailwind CSS v3 via PostCSS:

- `tailwind.config.js`
- `postcss.config.cjs`
- `src/index.css`

Do not switch back to Tailwind v4 casually; the current layout was tuned against Tailwind v3 output.

## Branding and feedback

Use `BrandMark` for the app logo and `ProviderLogo` for provider brands. Assets are shipped locally; keep logo mappings consistent between presets and saved providers, and retain [asset attribution](public/logos/README.md).

Use `ToastNotice` and the shared notification viewport for transient outcomes. Keep field validation and actionable recovery controls in place. Shared cards, page headers and dialog classes define the layout; see the [design guide](../docs/frontend-design.md) before adding page-specific styles.
