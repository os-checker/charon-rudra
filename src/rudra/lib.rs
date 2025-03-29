use crate::progress_info;
use crate::rudra::analysis::{SendSyncChecker, UnsafeDataflowChecker, UnsafeDestructorChecker};
use crate::rudra::context::CtxOwner;
use charon_lib::ast::TranslatedCrate;

use crate::rudra::report::ReportLevel;
use log::LevelFilter;

// // Insert rustc arguments at the beginning of the argument list that Rudra wants to be
// // set per default, for maximal validation power.
// pub static RUDRA_DEFAULT_ARGS: &[&str] =
//     &["-Zalways-encode-mir", "-Zmir-opt-level=0", "--cfg=rudra"];

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct RudraConfig {
    pub verbosity: LevelFilter,
    pub report_level: ReportLevel,
    pub unsafe_destructor_enabled: bool,
    pub send_sync_variance_enabled: bool,
    pub unsafe_dataflow_enabled: bool,
}

impl Default for RudraConfig {
    fn default() -> Self {
        RudraConfig {
            verbosity: LevelFilter::Info,
            //verbosity: Verbosity::Trace,
            report_level: ReportLevel::Info,
            unsafe_destructor_enabled: true,
            send_sync_variance_enabled: true,
            unsafe_dataflow_enabled: true,
        }
    }
}

// /// Returns the "default sysroot" that Rudra will use if no `--sysroot` flag is set.
// /// Should be a compile-time constant.
// #[allow(clippy::option_env_unwrap)]
// pub fn compile_time_sysroot() -> Option<String> {
//     // option_env! is replaced to a constant at compile time
//     if option_env!("RUSTC_STAGE").is_some() {
//         // This is being built as part of rustc, and gets shipped with rustup.
//         // We can rely on the sysroot computation in librustc.
//         return None;
//     }
//
//     // For builds outside rustc, we need to ensure that we got a sysroot
//     // that gets used as a default. The sysroot computation in librustc would
//     // end up somewhere in the build dir.
//     // Taken from PR <https://github.com/Manishearth/rust-clippy/pull/911>.
//     let home = option_env!("RUSTUP_HOME").or(option_env!("MULTIRUST_HOME"));
//     let toolchain = option_env!("RUSTUP_TOOLCHAIN").or(option_env!("MULTIRUST_TOOLCHAIN"));
//     Some(match (home, toolchain) {
//         (Some(home), Some(toolchain)) => format!("{}/toolchains/{}", home, toolchain),
//         _ => option_env!("RUST_SYSROOT")
//             .expect("To build Rudra without rustup, set the `RUST_SYSROOT` env var at build time")
//             .to_owned(),
//     })
// }

fn run_analysis<F, R>(name: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    progress_info!("{} analysis started", name);
    let result = f();
    progress_info!("{} analysis finished", name);
    result
}

pub fn analyze(crate_data: TranslatedCrate, config: RudraConfig) {
    // workaround to mimic arena lifetime
    let rcx_owner = CtxOwner::new(crate_data, config.report_level);
    let rcx = &*Box::leak(Box::new(rcx_owner));

    // Send/Sync variance analysis
    if config.send_sync_variance_enabled {
        run_analysis("SendSyncVariance", || {
            SendSyncChecker::new(rcx).analyze();
        })
    }

    // Unsafe dataflow analysis
    if config.unsafe_dataflow_enabled {
        run_analysis("UnsafeDataflow", || {
            UnsafeDataflowChecker::new(rcx).analyze();
        })
    }

    // Unsafe destructor analysis
    if config.unsafe_destructor_enabled {
        run_analysis("UnsafeDestructor", || {
            UnsafeDestructorChecker::new(rcx).analyze();
        })
    }
}
