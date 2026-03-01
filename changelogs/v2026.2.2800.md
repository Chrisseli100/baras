# v2026.2.2800

## UI upgrades

The user interface has been significantly changed to be more compact and intuitive. The positioning of controls has been shifted, but everything should still be only a few clicks away.

## Live Query Mode

The data explorer now supports querying data for the live encounter! It updates every 2.5 seconds, and you can enable it by clicking the **Live** button in the encounter sidebar.

Live Query mode is disabled by default, enabling it will moderately increase the application's CPU usage while viewing the data explorer.

## Data Explorer

- Eliminated much of the flickering, pop-in, shifting, and stutter visual artifacts caused by page loads when navigating the UI.
- Charts tab has been reworked so effect/ability boxes are easier to navigate
- Effect and ability uptime calculations are now more accurate
- Effects can now be filtered based on the source type

## Other

- Added hotkey for starting/stopping the operations timer

## Bugfixes

- Fixed issue where certain icons were not being rendered in the overlay
- Fixed issue causing timer based phase transition logic to be ignored when parsing historical files.
