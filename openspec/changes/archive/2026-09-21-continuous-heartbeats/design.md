# Design

Use a shared monotonic browser clock to set negative animation delays when cards are rendered. Replacement elements resume the current three-second sweep and 2.5-second core pulse phase instead of restarting. Thread sweeps occupy an absolute bottom track within existing cards, avoiding layout changes. Only fresh working threads animate; these are status indicators, not token or tool-call measurements. Existing reduced-motion rules disable them.
