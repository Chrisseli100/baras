# v2026.3.3

## Role-Based Profile Switching

You can now select a default profile for Tank/Healer/DPS roles. Upon role changes being recorded in the game, your active profile will automatically update to the default. You can still manually swap to non default profiles (e.g. 16 man healer).

## Role-Based Timer Visibility

The visibility of timers can now be toggled on/off for specific roles in the encounter editor. There are small buttons for toggling a timer for each role. This will disable the timer's appearance, alert text, and audio, however the timer will still run and fire any chains or timer logic it's based on.

## General

- Boss HP overlay now has option to continue displaying it after encounter end
- Users are now assigned a "Default" profile
- Streamlined header UI formatting, moved customization menu to controls section
- Global metrics settings now auto-collapse when selecting a new overlay in settings

## Bug fixes

- Fixed issue with effect window in charts tab not counting effects applied before the start of the selected time range
- Corrected DPS/EHPS values in character selector sidebar of ability usage and rotation view
- Overlays will now re-render data on profile switch
- Fixed several visual errors in the data explorer

# v2026.3.1

## General

- Timers/Phases/Counters and Effects have improved UI handling to discriminate between built-in, modified, and user created elements
- New option to hide disabled elements in editor tabs
- Notification now appears if overlays have been moved without being saved

## Data Explorer

- Removed donut charts from the data explorer
- Reformatted NPC health table to a more compact design
- Charts now properly resize when sidebars are collapsed/expanded or tab is set to fullscreen
- Removed flashing visual artifact from ability usage tab
- Effects on the charts tab are now consolidated into a single table

## Bugfixes

- Overlays positioned at the edge of the screen are now properly assigned to the active monitor when saving
- Text alerts and audio queues now properly sync with the timer's end time
- Fixed Starparse timer import setting display target to non-existent overlay
- The `Show at` field for encounter timers is now properly evaluated
- Fixed issue where effects were not scoped to source, causing parallel applications of the same effect from multiple sources to affect tracking of each other
- Fixed issue with Huntmaster success/wipe classification
- Prevent dead NPCs from being registered in the next encounter in specific edge cases
- Timer/Phase/Counter trigger chains are now evaluated recursively on each event
