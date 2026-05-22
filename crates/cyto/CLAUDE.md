# cyto

## Purpose

Main binary crate and entry point for the entire application. Parses top-level CLI arguments, sets up logging, and routes commands to the appropriate crate (`cyto-map`, `cyto-workflow`, or `cyto-ibu-*`).

## Key Source Files

- `src/main.rs` — CLI parsing (`Cli` struct via Clap), command routing via `match` on `Commands` enum, output directory validation
- `src/logging.rs` — Logging infrastructure with two modes:
  - `setup_workflow_logging()` — Dual-output: stderr (with ANSI colors) + log file (ANSI-stripped), used for `map` and `workflow` commands
  - `setup_default_logging()` — Standard stderr logging, used for `detect` and `ibu` subcommands

## Key Types

- `Cli` — Top-level Clap parser struct
- `MultiWriter` — `io::Write` implementation that tees output to both stderr and a file, stripping ANSI escape codes for the file output

## Design Notes

- Log level is controlled via `CYTO_LOG` environment variable (defaults to `Info`)
- `ext_sort` module logs are filtered to `Warn` level to reduce noise
- Clap v3-style colored help via custom `STYLES` constant

## Dependencies (within workspace)

Depends on all other workspace crates — this is the final binary that ties everything together. `cyto-cli` provides the command definitions, and all other crates provide the `run()` functions that get dispatched to.

## Testing

```bash
cargo test -p cyto
```

No unit tests in this crate — it's a thin routing layer. Integration testing is done via `justfile` targets (see root CLAUDE.md).
