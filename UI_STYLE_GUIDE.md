# Desktop UI Style Guide

This project uses a dark desktop shell with a cyan accent language: a near-black charcoal content area, a deep teal-black navigation rail on the left with brand block and nav items, rounded dark cards, cyan primary actions, and a subtle cyan particle field animating in the background.

## Required Style Rules

- Keep the main palette in dark charcoal (#14161A), sampled WeChat-IME-like sidebar gradient (#0F454E -> #1E4C56 -> #284156), muted dark cards (#1E2228), and **cyan** (#00D4D8) primary actions. Avoid returning to the older blue LAN-console look, the WeChat-green primary, white system control backgrounds, white scroll tracks, or unrelated bright panels.
- Prefer custom-painted or theme-colored controls when WinForms defaults would create white borders, white focus boxes, or system-colored scrollbars.
- Keep the animated background subtle and cheap. Use a small number of slow cyan particles without connection-line meshes or expensive glow brushes; avoid high-frequency full-window redraws that make the desktop shell look blurry or raise CPU/GPU usage.
- The left navigation rail is brand-block + 5 nav items mapped to the product workflow (Home/Room, Diagnostics, Game profiles, More tools, About). The active item uses a cyan left bar and cyan text; inactive items stay muted. Nav must not be replaced by decorative gradients.
- The right content area is multi-page; switching nav items swaps the visible page (BringToFront). The shared output console is docked at the bottom of the content area and stays available on every page.
- Keep the default customer workflow simple and player-facing: the Home page shows only the main LAN path actions (host, join, start, check) plus the room details card. Diagnostics, game profiles, runtime/Wintun/coordination/relay tools live on their own pages, accessed from the nav rail.
- When a section needs scrolling, use an in-theme dark track and cyan thumb, or another custom treatment that matches the existing title bar and input frames.
- Window chrome should remain custom-painted: dark title bar, compact language switcher, and line-based minimize/maximize/close icons. The chrome should be quiet, not glossy or game-like.
- Use rounded corners throughout the desktop shell: the borderless window, input frames, action buttons, room detail panels, nav items, and custom scroll thumbs should all use soft rounded geometry while keeping the compact layout.
- Single-line inputs must stay flat and clean: borderless text over a dark rounded frame, vertically centered, with no native TextBox lower block, white focus box, or mismatched bottom strip.
- Customer-facing buttons should use the custom clear-text button renderer: rounded cyan/dark backgrounds painted by the app, with text drawn via `TextRenderer` so high-DPI screens do not show soft or doubled system Button text.
- Window resize hit testing should not use visible or pseudo-transparent overlay panels on the four corners; WinForms transparent panels still paint over content and create square corner blocks. Keep resize behavior in window hit testing so rounded corners stay clean.
- The app icon must be a multi-size `.ico` containing at least 16/24/32/48/64/128/256 px entries. Small title-bar and taskbar sizes should use simplified, high-contrast shapes instead of relying on a scaled-down single bitmap.

## Current Layout Notes

- Left rail (200 px): circular brand logo + app name + tagline at top, then 5 navigation buttons.
- Right content area: page title at top, a dark rounded card with fields/actions, and the shared output console docked at the bottom.
- Home page: quick-flow fields (room name, host, invite) + 4 main action buttons + room details card on the right.
- Diagnostics / Game profiles / More tools pages: dedicated field sets and their action groups, each in its own card.
- About page: app name, version and description.
- The desktop shell is borderless, but edge and corner resize hit testing must continue to work without exposing a white system frame.
