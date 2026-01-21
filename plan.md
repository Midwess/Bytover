# Support Drag and Drop from Browser and Clipboard

## Overview
Enhance `shelf.tsx` in the desktop app to support multiple input methods for adding resources:
1. Drag and drop files from the operating system (existing Tauri solution)
2. Drag and drop images/content from web browsers (new HTML5 solution)
3. Paste content from clipboard (new clipboard plugin solution)

## Current State
- Tauri's `onDragDropEvent` handles OS file drops and provides file paths
- Does NOT support: browser images, URLs, text selections, or clipboard paste

## Implementation Plan

### 1. Add HTML5 Drag and Drop Events (Fallback)
Since Tauri's drag and drop only captures OS files with paths, we need HTML5 events for browser content.

**When to use HTML5 fallback:**
- User drags an image from Chrome/Firefox/Safari
- User drags a link from browser
- User drags selected text from a webpage
- Tauri event doesn't provide a file path

**Supported types (in priority order):**
| Type | MIME | Example | Output |
|------|------|---------|--------|
| URL | `text/uri-list` | `https://example.com/image.png` | Download to work folder |
| Plain Text | `text/plain` | `Hello world` | Create `.txt` file in work folder |
| HTML | `text/html` | `<b>Hello</b> world` | Create `.html` file in work folder |

### 2. Add Clipboard Paste Support
Use `tauri-plugin-clipboard` (community plugin) to read clipboard content when user presses Ctrl+V / Cmd+V.

**Why this plugin?**
- Official Tauri clipboard plugin does NOT support reading file paths
- Community plugin can read: file paths, images, text, and HTML

**Paste flow:**
1. User presses Ctrl+V / Cmd+V while shelf is focused
2. Listen for HTML5 `paste` event
3. Call Rust backend to read clipboard using `tauri-plugin-clipboard`
4. If clipboard contains file paths → use paths directly
5. If clipboard contains image data → write to work folder, then add resource
6. If clipboard contains text/URL → handle as fallback types

### 3. Rust Backend Functions (desktop/lib.rs)
Create separate handler functions for each content type:
```rust