use crate::rudra::{
    context::RudraCtxt,
    report::{rudra_report, Report, ReportLevel},
    utils::ColorSpan,
};
use charon_lib::{
    ast::{Body, FnOperand, FunDeclId, FunIdOrTraitMethodRef, TranslatedCrate},
    formatter::{Formatter, IntoFormatter},
    ullbc_ast::Statement,
};
use termcolor::Color;

#[derive(Clone, Copy)]
pub struct UnsafeDestructorChecker<'tcx> {
    rcx: RudraCtxt<'tcx>,
}

impl<'tcx> UnsafeDestructorChecker<'tcx> {
    pub fn new(rcx: RudraCtxt<'tcx>) -> Self {
        Self { rcx }
    }

    pub fn analyze(self) {
        let krate = &self.rcx.crate_data;
        // let fmt = krate.into_fmt();

        let trait_impls = krate
            .trait_impls
            .iter()
            .map(|t| (&t.impl_trait, &t.required_methods));
        for (impl_trait, methods) in trait_impls {
            let trait_did = impl_trait.trait_id;
            // println!("{}", fmt.format_object(trait_did));
            let trait_decl = &krate.trait_decls[trait_did];
            // println!("{}", fmt.format_object(trait_decl));
            let trait_name = &trait_decl.item_meta.name;
            let is_drop_trait = trait_name.equals_ref_name(&["core", "ops", "drop", "Drop"]);
            if !is_drop_trait {
                continue;
            }
            // drop fn must be in a Drop trait and as the first method
            let (fn_name, fn_did) = methods
                .first()
                .expect("drop should be the first method in Drop");
            // println!("{}", fmt.format_object(*fn_did));
            assert_eq!(fn_name.0, "drop");
            let fn_decl = &krate.fun_decls[*fn_did];
            let body_id = fn_decl.body.expect("Drop trait must have a body.");
            let body = match &krate.bodies[body_id] {
                Body::Unstructured(body) => body,
                _ => unreachable!(),
            };

            for bb in body.body.iter() {
                for stmt in &bb.statements {
                    if let Some(call) = stmt.content.as_call() {
                        if let FnOperand::Regular(fn_ptr) = &call.func {
                            match &fn_ptr.func {
                                FunIdOrTraitMethodRef::Fun(f) => {
                                    if let Some(f) = f.as_regular() {
                                        if fn_is_unsafe(krate, *f) {
                                            report(krate, *fn_did, stmt);
                                        }
                                    }
                                }
                                FunIdOrTraitMethodRef::Trait(_, _, f) => {
                                    if fn_is_unsafe(krate, *f) {
                                        report(krate, *fn_did, stmt);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Is the function safe?
fn fn_is_unsafe(krate: &TranslatedCrate, f: FunDeclId) -> bool {
    krate.fun_decls[f].signature.is_unsafe
}

/// Report unsafe drop fn.
fn report(krate: &TranslatedCrate, f: FunDeclId, stmt: &Statement) {
    let fun = &krate.fun_decls[f];

    // drop fn span
    let mut color_span = ColorSpan::new(krate, fun.item_meta.span)
        .expect("Failed to construct colored span {span:?}");
    // unsafe call span
    color_span.add_sub_span(Color::Red, stmt.span);
    rudra_report(Report::with_color_span(
        ReportLevel::Error,
        "UnsafeDestructor",
        format!(
            "Potential unsafe destructor issue in `{}`",
            krate.into_fmt().format_object(f)
        ),
        &color_span,
    ));
}
