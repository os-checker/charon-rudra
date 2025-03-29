use super::utils::{AdtGenericParams, Krate, TraitDid};
use crate::rudra::context::RudraCtxt;
use charon_lib::{
    ast::{TraitImpl, TranslatedCrate},
    formatter::FmtCtx,
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

    pub fn analyze(self) {}
}

fn generic_params(imp: &TraitImpl, krate: &TranslatedCrate, ctx: &FmtCtx) -> AdtGenericParams {
    let Some(this) = imp.impl_trait.generics.types.iter().next() else {
        panic!(
            "Display:{}\nDebug:{0:?}\nNo Self type in this trait impl.",
            imp.fmt_with_ctx(ctx)
        );
    };
    let Some(adt) = this.as_adt().and_then(|t| t.0.as_adt()) else {
        panic!(
            "Display:{}\nDebug:{0:?}\nin send impl should be an adt.",
            this.fmt_with_ctx(ctx),
        );
    };

    AdtGenericParams::new(krate, *adt)
}
