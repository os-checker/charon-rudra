use std::env;
use std::io;

use log::LevelFilter;

pub fn setup_logging() -> Result<(), fern::InitError> {
    // Default level as LOG=info, but LOG env var can be set to override
    let level = std::env::var("LOG")
        .ok()
        .and_then(|log| log.parse().ok())
        .unwrap_or(LevelFilter::Info);

    let mut base_config = fern::Dispatch::new();

    base_config = base_config.level(level).level_for(
        // log >= debug on debug build and >= info on release build
        "rudra-progress",
        if cfg!(debug_assertions) {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        },
    );

    if let Some(log_file_path) = env::var_os("RUDRA_LOG_PATH") {
        let file_config = fern::Dispatch::new()
            .filter(|metadata| metadata.target() == "rudra-progress")
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{} |PROGRESS-{:5}| {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.6f"),
                    record.level(),
                    message
                ))
            })
            .chain(fern::log_file(log_file_path)?);

        base_config = base_config.chain(file_config);
    }

    // stderr is captured and cached by Cargo, which leads to confusing output when used as `cargo rudra`
    let stdout_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} |{:5}| [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.6f"),
                record.level(),
                record.target(),
                message
            ))
        })
        .chain(io::stdout());

    base_config.chain(stdout_config).apply()?;

    Ok(())
}

#[macro_export]
macro_rules! progress_trace {
    ($($arg:tt)+) => (
        ::log::trace!(target: "rudra-progress", $($arg)+)
    )
}

#[macro_export]
macro_rules! progress_debug {
    ($($arg:tt)+) => (
        ::log::debug!(target: "rudra-progress", $($arg)+)
    )
}

#[macro_export]
macro_rules! progress_info {
    ($($arg:tt)+) => (
        ::log::info!(target: "rudra-progress", $($arg)+)
    )
}

#[macro_export]
macro_rules! progress_warn {
    ($($arg:tt)+) => (
        ::log::warn!(target: "rudra-progress", $($arg)+)
    )
}

#[macro_export]
macro_rules! progress_error {
    ($($arg:tt)+) => (
        ::log::error!(target: "rudra-progress", $($arg)+)
    )
}
