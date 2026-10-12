# Frontend design conventions

Dashboard and Providers define the visual baseline for the admin UI. Keep the existing dark palette and blue primary action; use the bundled font stack from `src/index.css`.

## Pages and cards

Use `PageHeader` for page titles and actions. Maintain a 24px section gap. Two-column pages use a flexible main column and a 320px secondary column at the `xl` breakpoint; both columns start at the same top edge. Do not give individual pages an extra centered width constraint.

Use `Card`, `CardHeader`, `CardTitle`, `CardDescription` and `CardContent`. These share the border, surface, typography, and 20px mobile / 24px desktop horizontal padding. Avoid adding dark header fills, glow, heavy shadows or decorative nested cards. A selected provider uses a subtle blue background. Status colors indicate actual state.

Lists use separated rows within a card. Keep edit/delete controls visible and give icon controls accessible names. Allow action groups to wrap on narrow screens; retain horizontal scrolling for wide data tables. Long names and paths must truncate with an available full value or wrap without forcing the page wider.

## Forms and dialogs

Inputs, selects and textareas share the same surface and focus treatment. Use `dialog-panel`, `dialog-body` and `dialog-actions` for form and confirmation surfaces. Dialog content must fit the viewport and scroll, including on short screens. Use `useDialog` for focus trapping, Escape handling, scroll locking and focus restoration, with existing unsaved-change guards. Keep a single blue primary action; destructive confirmations use the destructive variant.

Native selects must request the dark browser color scheme. Give their `option`
and `optgroup` elements an opaque `popover` background and `popover-foreground`
text; use `muted-foreground` for disabled options. Define these rules centrally
in `src/index.css`, including browser-rendered menus rather than only the closed
select. Custom menus use the same tokens. Verify readability in a real browser.

## Provider editor

Model discovery, selection, diagnostics and saving are separate actions. Loading
a catalog must not replace configured mappings or save a provider. Preserve
manual IDs and unrelated protocol/account metadata. The [editor guide](provider-editor.md)
defines user-visible save and account behavior.

| Component | Responsibility |
|-----------|----------------|
| `ProviderFormDialog` | Owns the draft and discovery catalog, account/endpoint context, pending-request invalidation and explicit save/preview actions. |
| `ModelPickerDialog` | Searches the fetched catalog, refreshes it on request and applies one model to one selected role. |
| `ModelMappingInput` | Provides manual input and an anchored menu for one role, using the editor's shared catalog. |
| `CodexDiagnostics` | Owns an independent account-scoped catalog check and derives mapping status without changing the draft. |

All four mapping inputs share the fetched catalog. Opening a role's menu resets
its search and shows all candidates, even when the input already contains a
different ID. Normalize and deduplicate candidates; retain an unlisted current
value. Search the full catalog before rendering batches of 100. Closing the
large picker retains the catalog for this editor session. Reopening the editor
or changing discovery context requires a new fetch; mapping edits reuse it.
Ignore responses from superseded contexts. A failed refresh retains the last
successful discovery catalog for the same context.

Anchor a mapping menu to the input's left edge and width. Render it through a
portal so card containment and dialog scrolling cannot clip it. Use a 4px gap,
keep at least 8px from viewport edges, choose above/below according to available
space, and cap height at 320px or that side's available height. Reposition on
scroll and resize. Long IDs truncate with the complete value available in a
tooltip. Do not apply the shared modal scale animation to anchored menus: it
moves their visible edges away from the input.

The arrow has an accessible role-specific name and is disabled until choices
are available. Arrow Down from the input opens the menu; search and options
support arrow navigation, with Home/End on options and Enter to select. Use
`useDialog` for nested focus containment, Escape, Tab and focus restoration;
outside pointer interaction dismisses the menu. Saving, previewing or opening
the large picker disables the mapping controls and closes their menus.

For Codex OAuth, show the fixed Responses protocol and **Check OAuth connection**
instead of general endpoint detection. Disable the check with an inline account
connection instruction when no connected account can be resolved. During a
check, show progress and prevent duplicate submissions. Success reports catalog
reachability and each mapping's presence; failure shows sanitized guidance.
Account changes clear results and invalidate pending checks; mapping changes
recompute statuses from the checked catalog. Diagnostics and discovery have
independent snapshots. Explain that catalog presence does not establish
generation access; verify that separately through the local route.

Review provider changes on wide and narrow screens, with long/unlisted IDs,
more than 100 candidates, no connected account, account changes during pending
requests, a failed refresh, and keyboard-only nested-menu interaction. Use
synthetic fixtures; do not place credentials or personal account data in images,
examples or test artifacts intended for publication.

## Feedback

Use `ToastNotice` for short operation results. Notices render in `NotificationViewport` outside page layout, expire after 5 seconds for success or 8 seconds for errors/warnings, pause during hover or keyboard focus, and support manual dismissal. Bulk Import/Sync outcomes, including partial or full failures, expose item details inside a scrollable notice.

Keep form validation next to its input or form. Retain inline recovery controls when the user needs to act; completion should remove the recovery panel. Loading/empty states and ongoing route state are page content, not transient notifications.

Keep page imports lazy and preserve reduced-motion support. Typography must work using assets shipped with the frontend; do not require server fonts or external font services.

Provider brands use the shared `ProviderLogo` component and the bundled assets documented in `public/logos/README.md`. Use the same icon identifier in preset selection and saved providers. Preserve original brand shapes, use a neutral icon for unknown providers, and keep logo display independent from third-party network services.

The app brand uses `BrandMark` and `public/brand-mark.svg`: two concentric open C arcs form a compact monogram. The sidebar, login screen and About panel share this mark. The mark has a transparent background and uses the existing blue accent with a pale outer arc. `public/favicon.svg` is the same asset at browser-tab scale; version its URL when replacing the mark to avoid stale favicon caches.

Managed Skills and MCP rows use the shared accessible `Switch` control with `role="switch"`, controlled enabled state, disabled write states, keyboard focus and reduced-motion support. Import and Restore open a selection dialog with no default selections. Use **Remove** for remembered managed removal and explain retained sources and explicit restoration.
