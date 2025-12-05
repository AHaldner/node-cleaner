# Node Cleaner

Terminal UI to find and remove `node_modules` folders in a directory tree. It scans asynchronously, shows sizes and last modified times, and lets you trigger deletion from the list.

## Run it

- `cargo run`

## Keys

- Up/Down: navigate
- Enter: confirm removal of the selected `node_modules` (with a popup)
- Esc/n: cancel popup
- q: quit

## Notes

- Deletions run in a background thread so the UI stays responsive.
- A deleting item is marked with `(deleting...)` while it is in progress.
- Safety checks ensure only `node_modules` folders under the chosen root are removed.
