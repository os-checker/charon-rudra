//! The generics orders on impl blocks, functions and adt declaration may
//! vary since it's allowed to write generics in random order.
//!
//! Thus we need to map impl blocks and functions generics order to
//! adt declaration order.
//!
//! NOTE:
//! * only generic type params are considered
//! * for some special APIs, non-adt generic type param will be treated
//!   as adt generic type param, e.g.
//!   for `impl Adt<T> { fn foo<U: Into<T>>(..) {} }`, `U` will be used
//!   like owned `T`

use charon_lib::{ast::*, formatter::FmtCtx, pretty::FmtWithCtx};
use indexmap::IndexMap;

/// Self type being implemented with a trait.
pub fn self_type<'a>(imp: &'a TraitImpl, ctx: &FmtCtx) -> &'a Ty {
    let Some(this) = imp.impl_trait.generics.types.iter().next() else {
        panic!(
            "Display:{}\nDebug:{0:?}\nNo Self type in this trait impl.",
            imp.fmt_with_ctx(ctx)
        );
    };
    this
}

#[derive(Debug, Clone, Copy)]
struct AdtTypeVarId(usize);

impl AdtTypeVarId {
    fn into_type_var_id(self) -> TypeVarId {
        TypeVarId::from_usize(self.0)
    }
}

/// Generic params order
#[derive(Debug, Default)]
pub struct ImplToAdtTypeVar {
    inner: IndexMap<TypeVarId, AdtTypeVarId>,
}

impl ImplToAdtTypeVar {
    /// val is the appearance order from adt;
    /// suppose that order aggrees with adt's TypeVarId.
    fn insert(&mut self, key: TypeVarId, val: usize) {
        self.inner.insert(key, AdtTypeVarId(val));
    }

    /// Convert a impl/fn sig type var id into adt type var id.
    pub fn get_adt_type_var_id(&self, impl_type_var_id: &TypeVarId) -> TypeVarId {
        let id = self.inner.get(impl_type_var_id).unwrap();
        id.into_type_var_id()
    }

    /// Fill the mapping from impl/fn sig generic types to adt type generic types.
    /// adt_generics is from a impl/fn sig, not from type decl.
    pub fn fill(&mut self, adt_generics: &GenericArgs) {
        self.inner.clear();
        for (adt_type_var_id, impl_type_var) in adt_generics.types.iter().enumerate() {
            // Skip specific adt types on adt's generic param position.
            if let Some(&key) = impl_type_var.as_type_var() {
                self.insert(key, adt_type_var_id);
            }
        }
    }
}

/// TypeVarId is in adt decl's order.
pub type TypeVarTraitBound = Vec<(TypeVarId, TraitDeclId)>;

pub fn trait_bounds_on_a_trait_impl(
    imp: &TraitImpl,
    krate: &TranslatedCrate,
    ctx: &FmtCtx,
) -> TypeVarTraitBound {
    let this = self_type(imp, ctx);
    let adt_generics = this.as_adt().unwrap().1;
    let mut mapping = ImplToAdtTypeVar::default();
    mapping.fill(adt_generics);

    let trait_clauses = &imp.generics.trait_clauses;
    let mut v = Vec::with_capacity(imp.generics.trait_clauses.len());
    for trait_clause in trait_clauses.iter() {
        let trait_bound = &trait_clause.trait_.skip_binder;

        // Only simple cases supported: e.g. T: Trait or U: Trait<X> (X doesn't matter here),
        // single Type with single bound, not considering compound type param (i.e. (X, Y): Trait
        // or <X as Trait1>: Trait2, ...).
        // Charon will split multi bounds into multi simple trait clauses for us.
        let ty = trait_bound.generics.types.iter().next().unwrap();
        if let Some(impl_type_var_id) = ty.as_type_var() {
            let adt_type_var_id = mapping.get_adt_type_var_id(impl_type_var_id);
            // add direct trait
            v.push((adt_type_var_id, trait_bound.trait_id));

            // add supertraits
            let trait_ = &krate.trait_decls[trait_bound.trait_id];
            for parent in trait_.parent_clauses.iter() {
                let supertrait = parent.trait_.skip_binder.trait_id;
                // this may cause trait collection contains duplicated items
                v.push((adt_type_var_id, supertrait));
            }
        }
    }
    v
}
