# Desktop UI Style Guide

This project currently uses a compact native Windows desktop shell with a dark blue LAN-console visual style.

## Required Style Rules

- Keep the main palette in deep navy, blue-gray, and cyan accents. Avoid introducing white system control backgrounds, white scroll tracks, or unrelated bright panels.
- Prefer custom-painted or theme-colored controls when WinForms defaults would create white borders, white focus boxes, or system-colored scrollbars.
- Keep the animated background subtle. Do not continuously repaint the full window for decorative particles unless flicker is eliminated by a dedicated buffered surface.
- Keep dense tool actions grouped on the right side below room details. The left side should remain focused on editable room/network fields.
- When a section needs scrolling, use an in-theme dark track and cyan thumb, or another custom treatment that matches the existing title bar and input frames.
- Window chrome should remain custom-painted: dark title bar, compact language switcher, and line-based minimize/maximize/close icons.

## Current Layout Notes

- Left column: labels and room/network input fields.
- Center column: primary editable fields and command output.
- Right column: room details at the top, action buttons below.
- The desktop shell is borderless, but edge and corner resize hit testing must continue to work without exposing a white system frame.
