use anyhow::Result;

use crate::cli::workflow::FlexMappingCommand;
use crate::commands::map as map_command;

pub fn run(args: &FlexMappingCommand) -> Result<()> {
    eprintln!("Running Flex Mapping Command");
    map_command::flex::run(&args.flex_args)?;
    Ok(())
}
