use super::utils::{self_type, AdtGenericParams, Krate, TraitDid};
use crate::rudra::context::RudraCtxt;
use charon_lib::{
    ast::{TraitImpl, TranslatedCrate},
    formatter::{FmtCtx, IntoFormatter},
    pretty::FmtWithCtx,
};

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
            let mut adt_type_params = generic_params(imp, krate, ctx);
            adt_type_params.add_trait_bounds_on_send_impl(imp, ctx);
            dbg!(&adt_type_params);
        }
    }
}

fn generic_params(imp: &TraitImpl, krate: &TranslatedCrate, ctx: &FmtCtx) -> AdtGenericParams {
    let this = self_type(imp, ctx);
    let Some(adt) = this.as_adt().and_then(|t| t.0.as_adt()) else {
        panic!(
            "Display:{}\nDebug:{0:?}\nin send impl should be an adt.",
            this.fmt_with_ctx(ctx),
        );
    };

    AdtGenericParams::new(krate, *adt)
}
