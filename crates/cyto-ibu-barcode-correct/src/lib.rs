mod stats;
mod whitelist;

pub use stats::{CorrectStats, FormattedStats};
pub use whitelist::Whitelist;

use std::path::Path;

use anyhow::Result;
use cyto_cli::ibu::ArgsBarcode;
use cyto_io::{match_input, match_output, match_output_stderr};
use ibu::{Reader, Record, Writer};
use log::trace;

use crate::whitelist::Correction;

fn write_statistics<P: AsRef<Path>>(path: Option<P>, stats: CorrectStats) -> Result<()> {
    let mut writer = match_output_stderr(path)?;
    let format_stats = FormattedStats::new(stats);
    serde_json::to_writer_pretty(&mut writer, &format_stats)?;
    writer.flush()?;
    Ok(())
}

/// Prebuild whitelist so multiple threads deduplicate work in building mismatch table.
pub fn run_with_prebuilt_whitelist(args: &ArgsBarcode, mut whitelist: Whitelist) -> Result<()> {
    // Build IO handles
    let input = match_input(args.input.input.as_ref())?;

    // Initialize the reader and header
    let reader = Reader::new(input)?;
    let header = reader.header();

    // Write the header to the output file
    let output = match_output(args.options.output.as_ref())?;
    let mut writer = Writer::new(output, header)?;

    // Process the records sequentially
    let mut stats = CorrectStats::default();
    let mut second_pass = Vec::new();

    trace!(
        "Starting first pass [file: {}, exact_match: {}]",
        args.input.input.as_deref().unwrap_or("stdin"),
        args.options.exact
    );
    for record in reader {
        let record = record?;
        let barcode = record.barcode;
        stats.total += 1;

        // Case where barcode is in the whitelist without error
        match whitelist.correct_to(barcode, args.options.exact) {
            Correction::Ambiguous => {
                if args.options.skip_second_pass {
                    stats.ambiguous += 1;
                    if args.options.include {
                        writer.write_record(&record)?;
                    }
                } else {
                    second_pass.push(record); // Record is ambiguous - will try to resolve in second pass
                }
            }
            Correction::Unchanged => {
                stats.matched += 1;
                stats.unchanged += 1;
                whitelist.increment(barcode);
                writer.write_record(&record)?;
            }
            Correction::Corrected(corrected) => {
                stats.matched += 1;
                stats.corrected += 1;
                whitelist.increment(corrected);
                let new_record = Record::new(corrected, record.umi, record.index);
                writer.write_record(&new_record)?;
            }
        }
    }

    if !second_pass.is_empty() && !args.options.exact {
        trace!(
            "Starting second pass (ambiguous subset) [file: {}]...",
            args.input.input.as_deref().unwrap_or("stdin")
        );
        for record in second_pass {
            match whitelist.ambiguously_correct_to_(record.barcode) {
                Correction::Ambiguous => {
                    stats.ambiguous += 1;
                    // Write ambiguous unless user wants to remove
                    if args.options.include {
                        writer.write_record(&record)?;
                    }
                }
                Correction::Unchanged => {
                    stats.matched += 1;
                    stats.unchanged += 1;
                    writer.write_record(&record)?;
                }
                Correction::Corrected(corrected) => {
                    stats.matched += 1;
                    stats.corrected += 1;
                    stats.corrected_via_counts += 1;
                    let new_record = Record::new(corrected, record.umi, record.index);
                    writer.write_record(&new_record)?;
                }
            }
        }
    }

    // Flush the output
    writer.finish()?;

    // Write the statistics to stderr
    write_statistics(args.options.log.as_ref(), stats)?;
    Ok(())
}

pub fn run(args: &ArgsBarcode) -> Result<()> {
    let whitelist = Whitelist::from_path(&args.options.whitelist)?;
    run_with_prebuilt_whitelist(args, whitelist)
}
