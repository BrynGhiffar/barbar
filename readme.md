# Barbar

A Wayland bar, kinda like waybar

Latest Issues:
- Crashes on monitor restart, due state being unsychronized
    (before thread sleep & after thread sleep).
    Solution: Integrate timerfd then use timer to trigger rerender
- Only renders datetime.
- Need to integrate:
    - Disk Usage
    - CPU Usage
    - Memory Usage
    - Show Running Apps (Show App Icon & Names)
    - Show workspaces (Want to create a fancy way to show this)
    - Need to integrate automatic theme color generation based on wallpaper.
