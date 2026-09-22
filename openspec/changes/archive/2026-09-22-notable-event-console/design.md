# Design
## Context
The current CLI repeats every source capture and pane plus metadata revisions. Separate recent-transition and banner areas compete with its ten-line feed.
## Goals / Non-Goals
Keep current thread state visible while prioritising meaningful event text. Do not fabricate tool execution or introduce a new socket transport.
## Decisions
Use an eight-thread page above one 24-row CLI. Remove SAMPLE/PANE and revision/readiness log records; show them only in telemetry/table. Keep baseline, status changes, attach/detach and source loss/recovery as concise two-line records. Store at most 60 records; duplicates and quiet captures produce no new lines.
Use a 120-second full-screen hold, still adjustable 0–300 in one-second steps. Local CLI effects reveal actual event text and briefly highlight arrivals; pause/reduced motion render settled text. They never interrupt a whole-scene effect.
Build a static A–Z Delta Corps Priest 1 character atlas from the existing attributed Omarchy font. Render a bounded project label on fresh done/blocked transitions, with full event context below. Eight-second banner duration, sixty-second cooldown, thirty-second pending expiry and current-source/status checks keep it occasional and truthful. Keep it inside the CLI, reserve room for recent event records, and exclude temporary art from full-scene captures.
## Risks / Trade-offs
Polling can miss intermediate states → retain disclosure. Fewer visible thread rows → keep counts and explicit paging. Artwork cannot represent digits/punctuation → preserve full project/event in ordinary text and use state word if no letters. Long names → fit whole glyphs to available width, keep original context beneath.
## Migration Plan
Verify timing, quiet captures, transitions, artwork bounds/freshness, local motion and browser geometry. Deploy both containers, check privacy and health, retain one rollback and remove temporary artifacts.
