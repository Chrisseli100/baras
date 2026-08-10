# v2026.8.5

## Position Constraints

Certain timer triggers now have the option to add position constraints. These check coordinate data of the source or target in a log event to determine if the trigger should fire.

Character position data can be viewed by hovering over the source or target fields in the combat log.

## Death Recap

The death recap has been updated to be more informative at-a-glance. Precomputed damage and healing taken statistics will now pop-up in a dialog box.

## General

- Add dynamic background option for alerts overlay
- Enable column sorting in data explorer effects table
- Adjust opacity and text location of raid frames in rearrange mode for greater visibility of in-game character names
- Added a dialog box thanking some contributors to the project
- Starparse Import feature removed
- Add per-source/ per-target discipline/role filters to effects editor
- Operations timer should now timeout if the player has been outside of the instance for more than 10 minutes.

## Bugfixes

- Dynamic profile switching now works upon logging in
- Dxun Red timers for bull spawn now work on veteran difficulty
- Olok Wave timers are now accurate
- Timer TTS fallback enabled on all timers with no audio file, not just alert timers
- Alert countdown text will now disable
- Ready state is ignored by non-cooldown overlays when calculating effect duration
- Combat log tie-break row sorting should now be consistent
- Raid frames now respects the "disable icon" option for individual effects, when show icons is enabled
- Operations timer should now properly reset upon pulling a boss in a different operation instance
