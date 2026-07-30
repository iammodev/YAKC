/**
 * Tauri API shim for the OBS browser source.
 *
 * overlay.js and devices.js talk to the Rust side through `window.__TAURI__`
 * (config via core.invoke, live input via event.listen). In a plain browser
 * that object doesn't exist, so this shim provides it, backed by the YAKC OBS
 * server: config over fetch(/config), live events over an SSE stream (/events).
 *
 * Loaded as a NON-deferred script before the deferred overlay.js / devices.js,
 * so `window.__TAURI__` is ready before they run.
 */

(() => {
  const bus = new EventTarget();

  // One shared SSE connection; the Rust hub sends {event, payload} frames.
  const source = new EventSource("/events");
  source.onmessage = (e) => {
    try {
      const msg = JSON.parse(e.data);
      bus.dispatchEvent(new CustomEvent(msg.event, { detail: msg.payload }));
    } catch {
      // Ignore malformed frames.
    }
  };

  window.__TAURI__ = {
    core: {
      invoke: async (cmd) => {
        switch (cmd) {
          case "get_config":
            return (await fetch("/config")).json();
          case "get_key_labels":
            return (await fetch("/key_labels")).json();
          case "get_pending_errors":
            return [];
          case "get_config_path":
            return "config.json";
          default:
            return null;
        }
      },
    },
    event: {
      listen: async (name, cb) => {
        const handler = (e) => cb({ payload: e.detail });
        bus.addEventListener(name, handler);
        return () => bus.removeEventListener(name, handler);
      },
    },
  };
})();
