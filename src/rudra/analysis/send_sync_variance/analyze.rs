use super::{
    utils::{self_type, AdtGenericParams, Krate, TraitDid},
    Tag,
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

        for imp in &self.sync_impls {
            analyze_sync(imp, &self.trait_dids, krate, ctx);
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
    adt_type_params.add_trait_bounds_on_impl(imp, ctx, true);
    // dbg!(&adt_type_params);

    // There must be Send trait for Send impls.
    let trait_send = traits.send.unwrap();
    let trait_copy = traits.copy;

    let mut impl_content = None::<String>;
    let mut tag_all_args = Tag::NAIVE_SEND_FOR_SEND;

    for (arg, info) in &adt_type_params.args {
        let mut tag_arg = Tag::NAIVE_SEND_FOR_SEND;

        let iter = info.adt_trait_bounds.iter();
        let trait_bounds = iter.chain(&info.send_impl_trait_bounds);
        for &trait_ in trait_bounds {
            if trait_ == trait_send || trait_copy.map(|copy| trait_ == copy).unwrap_or(false) {
                let tag = Tag::NAIVE_SEND_FOR_SEND;
                tag_arg.remove(tag);
                tag_all_args.remove(tag);
            }
        }

        if tag_arg.contains(Tag::NAIVE_SEND_FOR_SEND) {
            let adt = &krate.type_decls[adt_type_params.tid];
            let type_var_name = &adt.generics.types[*arg].name;
            report(type_var_name, &tag_arg, &mut impl_content, imp, ctx);
        }
    }
}

fn analyze_sync(imp: &TraitImpl, traits: &TraitDid, krate: &TranslatedCrate, ctx: &FmtCtx) {
    let mut adt_type_params = generic_params(imp, krate, ctx);
    adt_type_params.add_trait_bounds_on_impl(imp, ctx, false);

    for f in krate.fun_decls.iter() {
        adt_type_params.ownership_of_type_var_on_api(f);
    }
    // dbg!(&adt_type_params);

    // There must be Sync trait for Sync impls.
    let trait_sync = traits.sync.unwrap();
    let trait_send = traits.send;
    let trait_copy = traits.copy;

    let mut impl_content = None::<String>;
    let mut tag_all_args = adt_type_params.default_tag_for_all_args();

    for (arg, info) in &adt_type_params.args {
        let mut tag_arg = info.tag(Tag::empty());

        let iter = info.adt_trait_bounds.iter();
        let trait_bounds = iter.chain(&info.sync_impl_trait_bounds);
        for &trait_ in trait_bounds {
            if trait_ == trait_sync {
                let tag = Tag::API_SYNC_FOR_SYNC;
                tag_arg.remove(tag);
                tag_all_args.remove(tag);
            }
            if trait_send.map(|send| trait_ == send).unwrap_or(false)
                || trait_copy.map(|copy| trait_ == copy).unwrap_or(false)
            {
                let tag = Tag::API_SEND_FOR_SYNC;
                tag_arg.remove(tag);
                tag_all_args.remove(tag);
            }
        }

        if !tag_arg.is_empty() {
            let adt = &krate.type_decls[adt_type_params.tid];
            let type_var_name = &adt.generics.types[*arg].name;
            report(type_var_name, &tag_arg, &mut impl_content, imp, ctx);
        }
    }
}

fn report(
    type_var_name: &str,
    tag_arg: &Tag,
    impl_content: &mut Option<String>,
    imp: &TraitImpl,
    ctx: &FmtCtx,
) {
    let impl_str = &**impl_content.get_or_insert_with(|| {
        imp.item_meta
            .source_text
            .clone()
            .unwrap_or_else(|| imp.fmt_with_ctx(ctx))
    });
    println!(
        "\x1b[1m{impl_str}\x1b[0m\n╰───── \
        `\x1b[37;41m{type_var_name}\x1b[0m` doesn't meet \x1b[37;41m{tag_arg:?}\x1b[0m\n",
    );
}
