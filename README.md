# Node Cleaner

Terminal UI to find and remove `node_modules` folders in a directory tree. It scans asynchronously, shows sizes and last modified times, and lets you trigger deletion from the list.

## Install

This repository uses Rust 1.100.0. Rustup selects it automatically when you run Cargo from the checkout.

```bash
cargo install --locked --path .
```

## Keys

- Up/Down: navigate
- Enter: confirm removal of the selected `node_modules` (with a popup)
- Esc/n: cancel popup
- q: quit

## Usage

- Build and install once using the command above.
- From any folder you want to clean, run `node-cleaner`.
- The scan starts in the current working directory.

## Notes

- Deletions run in a background thread so the UI stays responsive.
- A deleting item is marked with `(deleting...)` while it is in progress.
- Safety checks ensure only `node_modules` folders under the chosen root are removed.
