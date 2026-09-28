// The title bar is hidden, so the window moves by dragging any `.drag` area (sidebar top, page
// headers). WKWebView ignores `-webkit-app-region`, so the drag is started through Tauri instead.
import { getCurrentWindow } from "@tauri-apps/api/window";

const INTERACTIVE = "button, a, input, select, textarea, label, [contenteditable], .selectable, [class*='app-region:no-drag']";

function inDragArea(target: EventTarget | null) {
  const el = target instanceof Element ? target : null;
  return !!el?.closest(".drag") && !el.closest(INTERACTIVE);
}

export function installWindowDrag() {
  const win = getCurrentWindow();
  document.addEventListener("mousedown", (e) => {
    if (e.button !== 0 || !inDragArea(e.target)) return;
    e.preventDefault();
    // A double click zooms the window, like a native title bar.
    if (e.detail === 2) void win.toggleMaximize();
    else void win.startDragging();
  });
}
