# Design

A header flex row contains project name and compact identity. It can truncate either independently. The checkout remains a separate short line when reported. A state group below it presents the native state badge alongside a current activity and reported tool. The activity is omitted when no observation is available; the state remains visible.

The metrics area has a context cell and two equally weighted data blocks. Only context uses a circular arc. Session input and output share one value block with equal rows; cached input share occupies a neighbouring block with a large percentage. Response-only values retain explicit scope. Unknowns remain dashes. Both React and fallback DOM use the same structure and accessible details.

Eight cards must fit at 1280×720 and 1920×1080, and the layout must remain readable at 16:10 and narrow tile widths. Review actual browser rectangles for overlap and scroll.
