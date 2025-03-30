use super::{
    utils::{self_type, AdtGenericParams, TraitDid},
    Tag,
};
use charon_lib::{
    ast::{TraitImpl, TranslatedCrate, Ty, TypeVarId},
    formatter::FmtCtx,
    ids::Vector,
    pretty::FmtWithCtx,
};

pub fn analyze_send(imp: &TraitImpl, traits: &TraitDid, krate: &TranslatedCrate, fmt: &FmtCtx) {
    let mut adt_type_params = generic_params(imp, krate, fmt);
    adt_type_params.add_trait_bounds_on_impl(imp, fmt, true);

    // There must be Send trait for Send impls.
    let trait_send = traits.send.unwrap();
    let trait_copy = traits.copy;

    let mut ctx = TraitImplCxt::new(imp, fmt);
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
            ctx.report(arg, &tag_arg);
        }
    }
}

pub fn analyze_sync(imp: &TraitImpl, traits: &TraitDid, krate: &TranslatedCrate, fmt: &FmtCtx) {
    let mut adt_type_params = generic_params(imp, krate, fmt);
    adt_type_params.add_trait_bounds_on_impl(imp, fmt, false);

    for f in krate.fun_decls.iter() {
        adt_type_params.ownership_of_type_var_on_api(f);
    }

    // There must be Sync trait for Sync impls.
    let trait_sync = traits.sync.unwrap();
    let trait_send = traits.send;
    let trait_copy = traits.copy;

    let mut ctx = TraitImplCxt::new(imp, fmt);
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
            ctx.report(arg, &tag_arg);
        }
    }
}

struct TraitImplCxt<'a, 'b> {
    imp: &'a TraitImpl,
    fmt: &'b FmtCtx<'b>,
    adt_type_params_on_impl: Option<&'a Vector<TypeVarId, Ty>>,
    impl_content: Option<String>,
}

impl<'a, 'b> TraitImplCxt<'a, 'b> {
    fn new(imp: &'a TraitImpl, fmt: &'b FmtCtx) -> Self {
        let (adt_type_params_on_impl, impl_content) = Default::default();
        Self {
            imp,
            fmt,
            adt_type_params_on_impl,
            impl_content,
        }
    }

    /// Get type var name on the impl.
    fn get_type_var_name(&mut self, arg: &TypeVarId) -> &'a str {
        let type_vars = self.adt_type_params_on_impl.get_or_insert_with(|| {
            let this = self_type(self.imp, self.fmt);
            &this.as_adt().unwrap().1.types
        });
        let pos = arg.raw();
        let impl_type_var_id = *type_vars[pos].as_type_var().unwrap();
        &self.imp.generics.types[impl_type_var_id].name
    }

    fn report(&mut self, arg: &TypeVarId, tag_arg: &Tag) {
        let type_var_name = self.get_type_var_name(arg);
        let impl_str = self.get_or_init_impl_content();
        eprintln!(
            "\x1b[1m{impl_str}\x1b[0m\n╰───── \
            `\x1b[37;41m{type_var_name}\x1b[0m` doesn't meet \x1b[37;41m{tag_arg:?}\x1b[0m\n",
        );
    }

    fn get_or_init_impl_content(&mut self) -> &str {
        self.impl_content.get_or_insert_with(|| {
            let source_text = self.imp.item_meta.source_text.clone();
            source_text.unwrap_or_else(|| self.imp.fmt_with_ctx(self.fmt))
        })
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
