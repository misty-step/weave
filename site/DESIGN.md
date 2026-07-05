# Weave DESIGN.md

This file is the product's public-site brand contract. Keep it short and exact:
agents and humans should be able to update `site/` from this file without
inventing a second design system.

## Brand Voice

- Plain-spoken, concrete, and operator-facing.
- Lead with the user outcome, then the proof.
- Avoid marketing fog, mascot language, and decorative claims.
- Weave is the aggregate: say what it actually is (contracts, doctrine, a
  composition layer) and what it actually links to (the family), not what it
  aspires to become.

## Pitch One-Liner

`Weave is the Misty Step factory's composition layer and hub: the contracts, event schema, and cross-tool doctrine that let independently-built products cohere into one fleet.`

## Lucide Mark

- Icon: `layers`
- Reason: selected 2026-07-02 through the fleet-wide icon-logo playground
  (`aesthetic/prototypes/icon-logo-playground.html`) and ratified in
  `backlog.d/016-adopt-lucide-layers-as-the-weave-wordmark-icon.md` — the
  fabric, the composition of agent-first dev tools, the loop, the stitching.
- Rule: the mark is an inline Lucide SVG inside `.ae-app-mark`. No bespoke
  marks, logo images, emoji marks, or colored wordmarks.

## Palette Hooks

Weave keeps the Aesthetic default palette — it is the contracts/doctrine
layer, not a product with its own runtime UI, so it does not need a
differentiated accent the way a live-viewer product (Glass) or board product
(Powder) does.

```css
:root {
  --ae-accent: #2643d0;
  --ae-accent-dark: #8c9eff;
}
```

## Screenshot Inventory

Weave has no runtime UI of its own — its product surface is contracts,
schemas, doctrine docs, and the fleet coverage map. The gallery below is real
terminal/GitHub output, not mockups:

| File                                             | Surface                          | State                                        | Caption                                                                             |
| ------------------------------------------------ | --------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------ |
| `site/assets/screenshots/01-verify-gate.png`      | `./scripts/verify.sh` terminal    | Real run against current `master` (2026-07-04) | The composition-contract gate: schema/fixture conformance plus the Rust workspace, all green. |
| `site/assets/screenshots/02-stacked-diff-log.png` | `git log --oneline -12` terminal  | Real weave `master` history                    | The stacked-diff discipline (weave-018) landing as a real bottom-up-merged 2-PR stack. |
| `site/assets/screenshots/03-five-faces-matrix.png` | Live GitHub render of `docs/the-five-faces.md` | Real merged doc, 2026-07-04 | The fleet coverage matrix that backs every capability claim on this page — no cell is asserted without it. |

## Footer Links

- Misty Step: `https://mistystep.io`
- GitHub: `https://github.com/misty-step/weave`
- Weave: omitted. Weave is the weave-family root — a self-link back to the
  page you're already on is noise, not navigation. The site-kit's "always
  present for weave-family products" rule is written for the family's
  *members*, not for Weave itself.

## Release Notes Rule

`site/changelog.html` is user-facing. Write entries as product outcomes, not
commit logs. Each entry needs a date, a version or release label, and one or two
plain-language bullets.

## The Family Grid (Weave-specific — not part of the generic site-kit contract)

Because Weave is the operator-ratified aggregate/hub product, `site/index.html`
carries one additional section beyond the generic kit: a card per product in
the weave family, each with its own Lucide mark, an honest one-line
description sourced from that product's own `VISION.md`/README (never
invented here), and a link to its live site where one exists. Every claim on
every card is backed by `docs/the-five-faces.md` (the pinned, merged coverage
matrix) or, for products newer than that assessment, is scoped to a plain
description with no face-coverage claim at all rather than a guessed one.

Family card sources (`repo`: visibility, mark, live site, evidence source):

| Product        | Repo visibility | Mark               | Live site                                  | Evidence                                  |
| --------------- | ---------------- | ------------------- | -------------------------------------------- | -------------------------------------------- |
| Bitterblossom   | public            | `flower` (ratified, `bitterblossom` backlog 111) | none yet | `docs/the-five-faces.md` row |
| Roster          | public            | `users` (provisional — no ratified mark yet) | none yet | `roster/VISION.md` (too new for the five-faces pass) |
| Powder          | public            | `snowflake` (matches Powder's own live site header) | https://misty-step.github.io/powder/ | `docs/the-five-faces.md` row + live site |
| Glass           | private           | `mirror-rectangular` (ratified, `glass/DESIGN.md`) | none yet (site built, repo private — Pages needs public visibility, not flipped by this card) | `glass/DESIGN.md` + `glass/VISION.md` |
| Glance          | private (`misty-step/glance`) | `scan-search` (provisional) | none yet | `glance-next/VISION.md` (too new for the five-faces pass; distinct from the archived `phrazzld/glance` Go tool) |
| Canary          | public            | `bird` (ratified) | none yet | `docs/the-five-faces.md` row |
| Crucible        | private           | `flask-conical` (ratified) | none yet | `docs/the-five-faces.md` row |
| Exocortex       | public            | `brain-circuit` (provisional) | none yet | `exocortex/VISION.md` (too new for the five-faces pass) |
| Landmark        | public            | `milestone` (ratified) | none yet | `docs/the-five-faces.md` row |

Provisional marks (Roster, Glance, Exocortex) are Weave's own placeholder
choice for this grid only — each product repo owns its real mark contract
when it adopts the site-kit itself; this table does not ratify anything on
their behalf.
