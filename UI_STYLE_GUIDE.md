# Desktop UI Style Guide

This project currently uses a compact native Windows desktop shell with a dark blue LAN-console visual style.

## Required Style Rules

- Keep the main palette in deep navy, blue-gray, and cyan accents. Avoid introducing white system control backgrounds, white scroll tracks, or unrelated bright panels.
- Prefer custom-painted or theme-colored controls when WinForms defaults would create white borders, white focus boxes, or system-colored scrollbars.
- Keep the animated background subtle. Do not continuously repaint the full window for decorative particles unless flicker is eliminated by a dedicated buffered surface.
- Keep dense tool actions grouped on the right side below room details. The left side should remain focused on editable room/network fields.
- Keep the default customer workflow simple and player-facing: show only the main LAN path actions first, keep labels plain, and hide diagnostics/developer tools behind a "More tools" disclosure. The first-screen goal is to help two players host, join, start LAN, and verify connection with as few choices as possible.
- Avoid exposing duplicate buttons in the default action area. If an action is already handled automatically by a quick flow, keep the manual version in "More tools".
- When a section needs scrolling, use an in-theme dark track and cyan thumb, or another custom treatment that matches the existing title bar and input frames.
- Window chrome should remain custom-painted: dark title bar, compact language switcher, and line-based minimize/maximize/close icons.
- Use rounded corners throughout the desktop shell: the borderless window, input frames, action buttons, room detail panels, and custom scroll thumbs should all use soft rounded geometry while keeping the compact layout.

## Current Layout Notes

- Left column: labels and room/network input fields.
- Center column: primary editable fields and command output.
- Right column: room details at the top, action buttons below.
- The desktop shell is borderless, but edge and corner resize hit testing must continue to work without exposing a white system frame.
