# Frontend design conventions

Dashboard and Providers define the visual baseline for the admin UI. Keep the existing dark palette and blue primary action; use the bundled font stack from `src/index.css`.

## Pages and cards

Use `PageHeader` for page titles and actions. Maintain a 24px section gap. Two-column pages use a flexible main column and a 320px secondary column at the `xl` breakpoint; both columns start at the same top edge. Do not give individual pages an extra centered width constraint.

Use `Card`, `CardHeader`, `CardTitle`, `CardDescription` and `CardContent`. These share the border, surface, typography, and 20px mobile / 24px desktop horizontal padding. Avoid adding dark header fills, glow, heavy shadows or decorative nested cards. A selected provider uses a subtle blue background. Status colors indicate actual state.

Lists use separated rows within a card. Keep edit/delete controls visible and give icon controls accessible names. Allow action groups to wrap on narrow screens; retain horizontal scrolling for wide data tables. Long names and paths must truncate with an available full value or wrap without forcing the page wider.

## Forms and dialogs

Inputs, selects and textareas share the same surface and focus treatment. Use `dialog-panel`, `dialog-body` and `dialog-actions` for form and confirmation surfaces. Dialog content must fit the viewport and scroll, including on short screens. Use `useDialog` for focus trapping, Escape handling, scroll locking and focus restoration, with existing unsaved-change guards. Keep a single blue primary action; destructive confirmations use the destructive variant.

## Feedback

Use `ToastNotice` for short operation results. Notices render in `NotificationViewport` outside page layout, expire after 5 seconds for success or 8 seconds for errors/warnings, pause during hover or keyboard focus, and support manual dismissal. Successful bulk operations may expose item details inside the notice.

Keep form validation next to its input or form. Retain inline recovery controls and failed bulk-operation details when the user needs to act; completion should remove the recovery panel. Loading/empty states and ongoing route state are page content, not transient notifications.

Keep page imports lazy and preserve reduced-motion support. Typography must work using assets shipped with the frontend; do not require server fonts or external font services.

Provider brands use the shared `ProviderLogo` component and the bundled assets documented in `public/logos/README.md`. Use the same icon identifier in preset selection and saved providers. Preserve original brand shapes, use a neutral icon for unknown providers, and keep logo display independent from third-party network services.

The app brand uses `BrandMark` and `public/brand-mark.svg`: two concentric open C arcs form a compact monogram. The sidebar, login screen and About panel share this mark. The mark has a transparent background and uses the existing blue accent with a pale outer arc. `public/favicon.svg` is the same asset at browser-tab scale; version its URL when replacing the mark to avoid stale favicon caches.

Managed Skills and MCP rows use the shared accessible `Switch` control with `role="switch"`, controlled enabled state, disabled write states, keyboard focus and reduced-motion support. Import and Restore open a selection dialog with no default selections. Use **Remove** for remembered managed removal and explain retained sources and explicit restoration.
