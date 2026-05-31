# ui/panel_list

- `mod.rs`: left panel layout, folder/thread row dispatch and width hints.
- `folder_row.rs`: folder/group row rendering.
- `thread_row.rs`: thread row rendering and jump badges.
- `viewport.rs`: only build rows around the visible sidebar selection to keep redraws cheap.
- `style.rs`: sidebar colors and shared style helpers.
- `animation.rs`: busy/waiting badge animation.
- `metrics.rs`: display width and truncation helpers.
