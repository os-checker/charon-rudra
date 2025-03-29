use super::{
    utils::{self_type, AdtGenericParams, Krate, TraitDid},
    BehaviorFlag,
};
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
            analyze_send(imp, &self.trait_dids, krate, ctx);
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

fn analyze_send(imp: &TraitImpl, traits: &TraitDid, krate: &TranslatedCrate, ctx: &FmtCtx) {
    let mut adt_type_params = generic_params(imp, krate, ctx);
    adt_type_params.add_trait_bounds_on_send_impl(imp, ctx);
    dbg!(&adt_type_params);

    // There must be Send trait for Send impls.
    let trait_send = traits.send.unwrap();
    let trait_copy = traits.copy;

    let mut tag_all_args = BehaviorFlag::NAIVE_SEND_FOR_SEND;

    for (arg, info) in &adt_type_params.args {
        let mut tag_arg = BehaviorFlag::NAIVE_SEND_FOR_SEND;

        let iter = info.adt_trait_bounds.iter();
        let trait_bounds = iter.chain(&info.send_impl_trait_bounds);
        for &trait_ in trait_bounds {
            if trait_ == trait_send || trait_copy.map(|copy| trait_ == copy).unwrap_or(false) {
                tag_arg.remove(BehaviorFlag::NAIVE_SEND_FOR_SEND);
                tag_all_args.remove(BehaviorFlag::NAIVE_SEND_FOR_SEND);
            }
        }

        if tag_arg.contains(BehaviorFlag::NAIVE_SEND_FOR_SEND) {
            let adt = &krate.type_decls[adt_type_params.tid];
            let type_var = &adt.generics.types[*arg];
            println!(
                "{}\n\x1b[30;41mGeneric Type Param `{}` is not Send\x1b[0m",
                imp.fmt_with_ctx(ctx),
                type_var.name
            );
        }
    }
}
