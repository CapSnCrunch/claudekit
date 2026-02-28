# ClaudeKit — Design Decisions

This document is the single source of truth for styling and design decisions.
Update it whenever a new decision is made.

---

## Color Palette

ClaudeKit uses an **always-dark** warm-stone theme inspired by the Claude brand.
There is no light mode — `html { color-scheme: dark; }` is hardcoded.

### CSS Custom Properties (`src/index.css`)

| Variable | HSL | Approx Hex | Purpose |
|---|---|---|---|
| `--background` | `24 9% 8%` | `#141210` | Main canvas |
| `--foreground` | `30 18% 88%` | `#e5ddd4` | Primary text — warm off-white |
| `--card` | `24 9% 11%` | `#1c1916` | Panel / card surfaces |
| `--popover` | `24 9% 13%` | — | Dropdowns, tooltips |
| `--primary` | `20 59% 59%` | `#d97757` | Claude orange — CTAs, links, icons, heatmap max |
| `--primary-foreground` | `24 9% 8%` | — | Text on primary backgrounds |
| `--secondary` | `24 9% 15%` | — | Secondary surfaces |
| `--muted` | `24 9% 14%` | — | Subtle backgrounds |
| `--muted-foreground` | `30 8% 52%` | `#8a8078` | Subdued labels, timestamps |
| `--accent` | `24 9% 16%` | — | Hover state for rows/buttons |
| `--border` | `24 9% 17%` | `#2a2521` | Subtle warm dividers |
| `--ring` | `20 59% 59%` | `#d97757` | Focus ring — matches primary |
| `--radius` | `0.45rem` | — | Border radius |

### Heatmap Intensity Scale

Warm orange tones from empty → full Claude orange.
Thresholds are **quartile-based** (25th / 50th / 75th percentile of non-zero days),
mirroring GitHub's contribution graph normalization.

| Level | Variable | HSL | Description |
|---|---|---|---|
| 0 | `--heat-0` | `24 9% 14%` | Empty cell (matches `--muted`) |
| 1 | `--heat-1` | `20 35% 28%` | Low activity |
| 2 | `--heat-2` | `20 45% 38%` | Medium-low |
| 3 | `--heat-3` | `20 55% 48%` | Medium |
| 4 | `--heat-4` | `20 59% 59%` | High — full Claude orange |

---

## Typography

- System font stack via Tailwind / browser default (no custom font loaded yet)
- `-webkit-font-smoothing: antialiased` on `body`
- `font-feature-settings: "rlig" 1, "calt" 1` for ligatures and contextual alternates
- Primary text: `text-sm` (14px) for most UI chrome
- Dense metadata: `text-xs` (12px), `text-[11px]`, `text-[10px]`, `text-[9px]`

---

## Animation Principles

**Use clean, fast animations that reinforce spatial relationships — never for decoration.**

### Rules
1. **Duration**: 150–200 ms for micro-interactions; 200–250 ms for spatial transitions.
   Anything slower feels sluggish; anything faster is imperceptible.
2. **Easing**: `ease-out` for elements entering the screen (fast start, gentle stop).
   `ease-in-out` for things expanding in place (accordion, grid rows).
3. **No layout thrash**: Prefer `transform`, `opacity`, and `grid-template-rows`
   over animating `height`, `max-height`, or `margin`.
4. **Respect motion preferences**: Future work — wrap animations in
   `@media (prefers-reduced-motion: no-preference)`.

### Implemented Animations

| Component | Trigger | Technique | Duration |
|---|---|---|---|
| Sidebar project folder chevron | Expand / collapse | CSS `transform: rotate(0→90deg)` | 200 ms `ease-in-out` |
| Sidebar session list | Expand / collapse | `grid-template-rows: 0fr → 1fr` (`.collapsible-grid`) | 200 ms `ease-in-out` |
| Day detail panel (Dashboard) | Day click opens panel | `slideDown` keyframe (+6px Y + opacity 0→1) | 180 ms `ease-out` |

### CSS Utilities (`src/index.css`)

```css
/* Enter animations */
.animate-slide-down  — slideDown keyframe (opacity + translateY)
.animate-fade-in     — fadeIn keyframe (opacity only)

/* Expand/collapse */
.collapsible-grid    — grid container; animate gridTemplateRows between "0fr" and "1fr"
```

---

## Component Layout Decisions

- **Sidebar**: Fixed 256 px (`w-64`), `bg-card`, right border. Never collapses.
- **Main area**: Flex-fill, `overflow-y-auto`, max-width `max-w-4xl` centered on Dashboard.
- **Stat cards**: 2-col on small screens, 4-col on `sm:` breakpoint.
- **Heatmap panel**: `bg-card` rounded card with `heatmap-root` class for tooltip positioning.
- **Day detail panel**: Appears directly below the heatmap in document flow (not a modal).
  Dismissed with an ✕ button or by clicking the same day again.

---

## Decisions Log

| Date | Decision |
|---|---|
| 2025-07 | Always-dark theme — no light mode toggle |
| 2025-07 | Claude orange `#d97757` as primary accent |
| 2025-07 | Warm stone backgrounds (hue 24, matching Claude brand) |
| 2025-07 | Quartile-based heatmap normalization (vs fixed thresholds) |
| 2025-07 | Count only `is_human_prompt = 1` messages in heatmap + session badge |
| 2026-02 | Use `grid-template-rows` trick for smooth sidebar accordion |
| 2026-02 | `slideDown` keyframe for day detail panel entrance |
| 2026-02 | Single rotating chevron (no icon swap) for project folders |
