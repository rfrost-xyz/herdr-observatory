# Design

Rebuild the visual system around compact terminal chrome, full-card circuit borders, scanning planes, moving grid textures and console text. Retain deterministic entity phase so redraws cannot restart effects. Only working agents and machines with working agents receive active circuit FX; blocked cards get static attention treatment. Connection loss removes all active effects.

The bounded snapshot console derives records from actual source captures and sampled transitions, escaping every source string. It does not simulate shell commands, tool execution or token throughput. Display a persistent derived-data label. Console records preserve their capture timestamps; animation does not fabricate observations.

Retain six agent/three machine pagination, keyboard controls, Omarchy colours and fixed 720p/1080p layout. Reduced-motion disables every animation. Test rendered geometry, computed animations, stale states and malicious strings; inspect synthetic screenshots before deployment.
