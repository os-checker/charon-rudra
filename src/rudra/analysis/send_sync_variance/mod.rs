//! Unsafe Send/Sync impl detector

use crate::rudra::analysis::IntoReportLevel;
use crate::rudra::report::ReportLevel;
use bitflags::bitflags;

mod analyze;
mod utils;

bitflags! {
    #[derive(Default, Debug, Clone, Copy)]
    pub struct BehaviorFlag: u8 {
        // T: Send for impl Sync (with api check & phantom check)
        const API_SEND_FOR_SYNC = 0b00000001;
        // T: Sync for impl Sync (with api check & phantom check)
        const API_SYNC_FOR_SYNC = 0b00000100;
        // T: Send for impl Send (with phantom check)
        // const PHANTOM_SEND_FOR_SEND = 0b00000010;
        // T: Send for impl Send (no api check, no phantom check)
        const NAIVE_SEND_FOR_SEND = 0b00001000;
        // T: Sync for impl Sync (no api check, no phantom check)
        const NAIVE_SYNC_FOR_SYNC = 0b00010000;
        // Relaxed Send for impl Send (with phantom check)
        const RELAX_SEND = 0b00100000;
        // Relaxed Sync for impl Sync (with phantom check)
        const RELAX_SYNC = 0b01000000;
    }
}

impl IntoReportLevel for BehaviorFlag {
    fn report_level(&self) -> ReportLevel {
        let high = BehaviorFlag::API_SEND_FOR_SYNC | BehaviorFlag::RELAX_SEND;
        let med = BehaviorFlag::API_SYNC_FOR_SYNC
            // | BehaviorFlag::PHANTOM_SEND_FOR_SEND
            | BehaviorFlag::RELAX_SYNC;

        if !(*self & high).is_empty() {
            ReportLevel::Error
        } else if !(*self & med).is_empty() {
            ReportLevel::Warning
        } else {
            ReportLevel::Info
        }
    }
}
