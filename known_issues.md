# Known Issues

Issues are tracked externally. For the current list of known bugs, limitations, and
planned improvements, see [GitHub Issues](https://github.com/GnosticEchos/VibeEye/issues).

## Notable limitations

- **Authentication:** No built-in support for login forms or session-based auth.
  Only publicly accessible content is supported.
- **Servo alpha quality:** Servo 0.1.0 is pre-1.0. Some sites may render differently
  than Chromium. Mitigated with `libc::_exit(0)` to avoid SpiderMonkey segfault on
  process exit.
- **SPA edge cases:** Some client-side rendering patterns (Astro partial hydration,
  heavy WASM) may not settle fully. The SPA settle loop handles most cases.
- **Anti-bot blocking:** Sites like GitHub may return JS-gated or rate-limited content.
  This is expected from the target site, not a bug in VibeEye.
- **Memory:** Each browser session loads a full Servo instance. The engine drops the
  WebView between pages to prevent OOM on long crawls.
