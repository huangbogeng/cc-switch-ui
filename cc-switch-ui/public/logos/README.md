# Provider brand assets

The provider list, preset picker and selected-preset header share `ProviderLogo` and the local allowlist in `src/lib/providerLogo.ts`. Assets ship with the UI; displaying a logo does not contact a provider or an icon CDN.

All assets except OrcaRouter are copied from the cc-switch extracted icon library at commit `b4a079430ce85a604e10d97d4b7530774e00e112`:
https://github.com/farion1231/cc-switch/tree/b4a079430ce85a604e10d97d4b7530774e00e112/src/icons/extracted

The upstream MIT license is included as `LICENSE.cc-switch`. Existing brand shapes, raster pixels and multicolor fills are retained. SVG `currentColor` is set to `#E2E8F0` for dark-theme contrast when displayed as an image. `aihubmix` maps to upstream `aihubmix-color.svg`.

`orcarouter.png` is the official icon linked from https://www.orcarouter.ai/:
https://www.orcarouter.ai/orca-icon-192.png

Brand marks identify their respective providers. New preset records persist their icon identifier. Existing records resolve known identifiers, preset IDs or provider domains without changing stored user data. Unknown brands and failed image loads use a neutral server icon.
