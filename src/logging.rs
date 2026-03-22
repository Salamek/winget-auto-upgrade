use crate::config::Config;
use log::LevelFilter;
use log4rs::{
    append::{
        console::ConsoleAppender,
        rolling_file::{
            policy::compound::{
                roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
            },
            RollingFileAppender,
        },
    },
    config::{Appender, Config as LogConfig, Root},
    encode::pattern::PatternEncoder,
};

pub fn init(config: &Config) -> anyhow::Result<()> {
    let pattern = "{d(%H:%M:%S)} - {l} - {m}{n}";

    let console = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build();

    // Rotated files: winget-update.log.1, winget-update.log.2, ...
    let roller_pattern = format!("{}.{{}}", config.log_path);
    let roller = FixedWindowRoller::builder()
        .build(&roller_pattern, config.max_log_files)?;
    let trigger = SizeTrigger::new(config.max_log_size);
    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    let file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build(&config.log_path, Box::new(policy))?;

    let log_config = LogConfig::builder()
        .appender(Appender::builder().build("console", Box::new(console)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .build(Root::builder().appender("console").appender("file").build(LevelFilter::Info))?;

    log4rs::init_config(log_config)?;

    Ok(())
}
