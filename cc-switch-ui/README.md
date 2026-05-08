# cc-switch-ui

React frontend for **CC Switch Web**.

## Stack

- React 19
- TypeScript
- Vite
- Tailwind CSS v3
- Radix UI primitives
- lucide-react icons

## Development

```bash
npm install
npm run dev
```

The Vite dev server proxies `/api` to `http://localhost:5007`, so run the backend from the repository root in another terminal:

```bash
cargo run --bin cc-switch-server
```

## Build

```bash
npm run lint
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
