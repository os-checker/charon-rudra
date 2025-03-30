//! Unsafe Send/Sync impl detector

use crate::rudra::context::RudraCtxt;
use charon_lib::{ast::TraitImpl, formatter::IntoFormatter};
use utils::{Krate, TraitDid};

mod analyze;
mod utils;

bitflags::bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Tag: u8 {
        // T: Send for impl Sync (with api check & phantom check)
        const API_SEND_FOR_SYNC = 0b00000001;
        // T: Sync for impl Sync (with api check & phantom check)
        const API_SYNC_FOR_SYNC = 0b00000100;
        // T: Send for impl Send (with phantom check)
        // const PHANTOM_SEND_FOR_SEND = 0b00000010;
        // T: Send for impl Send (no api check, no phantom check)
        const NAIVE_SEND_FOR_SEND = 0b00001000;
        // T: Sync for impl Sync (no api check, no phantom check)
        // const NAIVE_SYNC_FOR_SYNC = 0b00010000;
        // Relaxed Send for impl Send (with phantom check)
        // const RELAX_SEND = 0b00100000;
        // Relaxed Sync for impl Sync (with phantom check)
        // const RELAX_SYNC = 0b01000000;
    }
}

pub struct SendSyncChecker<'tcx> {
    rcx: RudraCtxt<'tcx>,
    trait_dids: TraitDid,
    send_impls: Vec<&'tcx TraitImpl>,
    sync_impls: Vec<&'tcx TraitImpl>,
}

impl<'tcx> SendSyncChecker<'tcx> {
    pub fn new(rcx: RudraCtxt<'tcx>) -> Self {
        let krate = Krate::new(&rcx.crate_data);
        let trait_dids = krate.send_sync_copy();
        Self {
            rcx,
            send_impls: krate.trait_impls(trait_dids.send),
            sync_impls: krate.trait_impls(trait_dids.sync),
            trait_dids,
        }
    }

    pub fn analyze(self) {
        let krate = &self.rcx.crate_data;
        let ctx = &krate.into_fmt();

        for imp in &self.send_impls {
            analyze::analyze_send(imp, &self.trait_dids, krate, ctx);
        }

        for imp in &self.sync_impls {
            analyze::analyze_sync(imp, &self.trait_dids, krate, ctx);
        }
    }
}
