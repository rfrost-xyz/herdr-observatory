# Design

Server creates a bounded 120x36 character capture of the complete permitted terminal. The adapter receives the active theme colours and emits every frame until the library returns completion, followed by an explicit END marker. A 5000-frame/128MiB/15-second generation budget rejects the entire effect on exhaustion; no partial effect is played. HTTP compresses the validated ANSI-coloured frames. Only foreground RGB/reset sequences are allowed.

Browser plays one frame per tick, without replacing a running effect when observations refresh. Every frame, including the final one, is shown. A hold phase starts only after completion; default ten seconds, arrows adjust by one second (0..300). Hidden tabs and pause freeze progress. Reduced motion shows the live TUI. Source loss cancels an obsolete capture; ordinary fresh samples do not. Only complete successful effects update selection history, and next choice excludes the previous choice. Page Up/Down replaces old arrow paging.

All content participates in the effect, including header, sources, threads, stream and footer; floating controls hide during playback. The live TUI remains current between effects. Theme colours drive the library gradients/noise; internal neutral colours map to theme foreground. Branding is absent from display labels. Required source attribution and licences remain in developer files.
