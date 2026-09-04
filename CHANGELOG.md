# v2026.9.3

## Streaming Display Source

In the application settings menu, there is now an option the mirror the visible overlays into a locally hosted web page. This webpage can be used as a browser source in OBS and other streaming
software programs in order to avoid the need for desktop capture to display the overlays in a stream or recorded video.

## Boss HP Bar

The entries in the overlay can now be set to a consistent size using the **Bosses Sized to Fit** option. This will reserve space for the selected number of
elements in the preview mode and will render them exactly as set. No compression or shifting will occur when entries are added or removed until the selected value is
exceeded.

- Icon position can be set to bottom/top/left/right
- Toggling off _Show HP Value_ now causes the boss name to increase in size
- Icon placeholders can now be seen in the preview

## Challenges

The challenges overlay has a similar option. It can be sized to fit a selected number of challenges. This allows for reserving space to display multiple challenges without a single challenge stretching across the entire overlay window.

There is also an option to start the display from the left/right or top/bottom of the overlay window (depending on the direction selected).

## Timers and Effects

- Renaming and cleanup of phase names and HP markers in Gods from the Machine
- Nahut combat start shield will now appear after he uncloaks
- Izax deflection shield will no longer get stuck on overlay after it should have expired
- Scyva Ignite Core challenge added
- Minor changes to SnV encounter definitions
- Affliction/Weaken Mind DOTs added for Lightning/TK
- Retractable Blade/Gut DOTs added for AP/Tactics

## Other

- Latency has been renamed to "Bias Adjustment" and now allows for negative values. This more clearly describes the field's in-app function
- Discipline icons no longer overlap with metrics bars

# v2026.8.29

## Effects Tracking

- Added Effects C and Cooldowns B overlays
- New option **Show Inactive** shows a greyed-out version of the icon/bar in a stable location on the screen that will fill in when the effect is Inactive. (pair with a 0 duration to track persistent effects such as Guard)
- Show countdown/stacks and other display options can now be toggled individually per effect instead of per overlay
- Kolto Shells, Kotlo Probe and Mirror abilities now show stack count at 1
- Raid frames now has an option to show a colored border that remains as the duration counts down
- Effects max icon/bar size increased

## Boss HP Bar

- HP bar formatting updated to be more readable.
- Additional scaling options and toggles have been added

## General

- Countdown bars for timers and effects now end at the rightmost edge of the icon instead of the icon obscuring visibility or the bar. Entries without icons are given a place-holder diamond glyph.

## Bugfixes

- Fixed improper data filtering when double clicking the timeline element to select a phase
- File selector area badges should now display properly for DE/FR localizations
- The metrics overlay "max entries" option now selects the top N entries across both teams in PvP zones, instead of the top N on your team
- Opening files where the final fight ended in a logout/disconnect will no longer trigger live parsing in historical mode
- DOT tracker bar mode will now use the display text field, if it is present
