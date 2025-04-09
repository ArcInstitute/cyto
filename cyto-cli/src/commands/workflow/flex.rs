use anyhow::Result;

use crate::cli::workflow::FlexMappingCommand;

pub fn run(args: &FlexMappingCommand) -> Result<()> {
    let flex_args = &args.flex_args;

    eprintln!("Flex Args: {:?}", flex_args);

    Ok(())
}
