use crate::config::Config;
use simplelog::*;
use std::fs::File;

pub fn init(config: &Config) -> anyhow::Result<()> {
    let log_file = File::create(&config.log_path)?;

    let log_config = ConfigBuilder::new()
        .set_time_level(LevelFilter::Info)     // log timestamp for info and above
        .build();

    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, log_config.clone(), TerminalMode::Mixed, ColorChoice::Auto),
        WriteLogger::new(LevelFilter::Info, log_config, log_file),
    ])?;

    Ok(())
}
