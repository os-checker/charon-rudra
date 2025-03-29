use super::{super::Tag, generics_mapping::ImplToAdtTypeVar, trait_bounds_on_a_trait_impl};
use charon_lib::{ast::*, formatter::FmtCtx};
use indexmap::IndexMap;

/// Generic param infomation, related to Send/Sync analysis.
#[derive(Debug, Default)]
pub struct ParamInfo {
    // Insertion order, i.e. the position on adt's declaration.
    // pub id: usize,
    /// Trait bounds on send impl.
    pub send_impl_trait_bounds: Vec<TraitDeclId>,
    /// Trait bounds on sync impl.
    pub sync_impl_trait_bounds: Vec<TraitDeclId>,
    /// Trait bounds on adt's declaration.
    pub adt_trait_bounds: Vec<TraitDeclId>,
    /// Owned or pointed.
    pub ownership_behavior: OwnershipFlag,
    // Does the generic param (only?) appear in PhantomData?
    // pub is_in_phantomdata: bool,
}

impl ParamInfo {
    fn update_ownership_behavior(&mut self, flag: OwnershipFlag) {
        self.ownership_behavior.insert(flag);
    }

    pub fn tag(&self, mut tag: Tag) -> Tag {
        if self.ownership_behavior.contains(OwnershipFlag::OWNED) {
            tag.insert(Tag::API_SEND_FOR_SYNC);
        }
        if self.ownership_behavior.contains(OwnershipFlag::POINTED) {
            tag.insert(Tag::API_SYNC_FOR_SYNC);
        }
        tag
    }
}

/// Type param and analysis basis on a adt.
pub type Args = IndexMap<TypeVarId, ParamInfo>;

/// Generic param on adt.
#[derive(Debug)]
pub struct AdtGenericParams {
    /// Type id of adt.
    pub tid: TypeDeclId,
    /// The map order corresponds to the position of adt's generic params.
    ///
    /// TypeVarId is the generic type id on a specific adt.
    pub args: Args,
}

impl AdtGenericParams {
    pub fn new(krate: &TranslatedCrate, tid: TypeDeclId) -> Self {
        let adt = &krate.type_decls[tid];

        let generic_types = adt.generics.types.iter();
        let mut args: Args = generic_types
            .map(|t| (t.index, ParamInfo::default()))
            .collect();

        adt_trait_bounds(adt, &mut args);

        // type params ownership behaviors from adt decl
        for field in fields(&adt.kind) {
            let mut v = vec![];
            ownership_behavior(field.ty.kind(), &mut v, &mut false);
            for (type_var_id, flag) in v {
                let info = &mut args.get_mut(&type_var_id).unwrap();
                info.update_ownership_behavior(flag);
            }
        }

        AdtGenericParams { tid, args }
    }

    pub fn add_trait_bounds_on_impl(&mut self, imp: &TraitImpl, ctx: &FmtCtx, send: bool) {
        for (adt_type_var, trait_id) in trait_bounds_on_a_trait_impl(imp, ctx) {
            let type_var = self.args.get_mut(&adt_type_var).unwrap();
            if send {
                &mut type_var.send_impl_trait_bounds
            } else {
                &mut type_var.sync_impl_trait_bounds
            }
            .push(trait_id);
        }
    }

    /// Update ownership state on adt's type var.
    pub fn ownership_of_type_var_on_api(&mut self, f: &FunDecl) {
        // only look at Regular and TraitImpl functions (TraitDecl is excluded)
        // https://os-checker.github.io/charon-rudra/charon/charon_lib/ast/gast/enum.ItemKind.html
        if matches!(&f.kind, ItemKind::TraitDecl { .. }) {
            return;
        }

        let mut mapping = ImplToAdtTypeVar::default();
        let mut impl_type_var_id_flags = vec![];
        let sig = &f.signature;

        for kind in sig.inputs.iter().chain([&sig.output]).map(|t| t.kind()) {
            // Skip types irrelevant to this adt. We use naive for loop, because
            // manual sync impls are rare, so such adts are rare, if types are
            // cached beforehand, a lot of unused adts fns will be computed.
            // We also omit the adt used as a field or variant in another adt.
            // FIXME: handle Self type via PathElem in Name of ItemMeta
            let Some((adt_generics, mut behind_a_pointer)) = adt(kind, self.tid, false) else {
                continue;
            };
            println!("Analyze func {}", f.def_id);

            // TypeVarId is defined on impl block or function sig,
            // so need a mapping to focus on adt's TypeVarId.
            mapping.fill(adt_generics);

            ownership_behavior(kind, &mut impl_type_var_id_flags, &mut behind_a_pointer);
            for (impl_type_var_id, flag) in &impl_type_var_id_flags {
                let adt_type_var_id = mapping.get_adt_type_var_id(impl_type_var_id);
                let info = &mut self.args.get_mut(&adt_type_var_id).unwrap();
                info.update_ownership_behavior(*flag);
            }
        }
    }

    /// Default tag state based on ownership behaviors of all args.
    pub fn default_tag_for_all_args(&self) -> Tag {
        self.args
            .iter()
            .fold(Tag::empty(), |acc, (_, info)| info.tag(acc))
    }
}

fn adt(kind: &TyKind, tid: TypeDeclId, behind_a_pointer: bool) -> Option<(&GenericArgs, bool)> {
    match kind {
        TyKind::Adt(t, p) => {
            if let Some(&id) = t.as_adt() {
                if id == tid {
                    return Some((p, behind_a_pointer));
                }
            }
        }
        // FIXME: &Wrapper<T> is approximately treated as &T
        TyKind::Ref(_, t, _) | TyKind::RawPtr(t, _) => return adt(t.kind(), tid, true),
        _ => (),
    }
    None
}

/// Add adt definition trait bounds to each generic arg.
/// Usually `Size` appears becuase it's implicitly added by rustc.
fn adt_trait_bounds(adt: &TypeDecl, args: &mut Args) {
    let trait_clauses = adt.generics.trait_clauses.iter();
    for trait_clause in trait_clauses {
        let trait_bound = &trait_clause.trait_.skip_binder;
        // T              : Trait
        // |                |
        // type_var_id      trait_id
        if let Some(type_arg) = trait_bound.generics.types.iter().next() {
            if let Some(type_arg_id) = type_arg.as_type_var() {
                if let Some(info) = args.get_mut(type_arg_id) {
                    let trait_id = trait_bound.trait_id;
                    info.adt_trait_bounds.push(trait_id);
                } else {
                    error!(?adt, ?type_arg, "Invalid type arg id.");
                }
            } else {
                error!(?adt, ?type_arg, "No type arg in trait bound.");
            }
        } else {
            error!(?adt, ?trait_clause, "No type arg in trait bound.");
        }
    }
}

/// Fields in adt. For enums, all fields in each variant are collected.
fn fields(ty_decl: &TypeDeclKind) -> Vec<&Field> {
    match ty_decl {
        TypeDeclKind::Struct(v) | TypeDeclKind::Union(v) => v.iter().collect(),
        TypeDeclKind::Enum(e) => e.iter().flat_map(|e| e.fields.iter()).collect(),
        _ => vec![],
    }
}

fn ownership_behavior(
    ty_kind: &TyKind,
    v: &mut Vec<(TypeVarId, OwnershipFlag)>,
    behind_a_pointer: &mut bool,
) {
    match ty_kind {
        TyKind::Adt(_, generics) => {
            for ty in generics.types.iter() {
                ownership_behavior(ty.kind(), v, behind_a_pointer);
            }
        }
        TyKind::TypeVar(type_var_id) => {
            // let ownership = &mut args.get_mut(type_var_id).unwrap().ownership_behavior;
            let flag = if *behind_a_pointer {
                OwnershipFlag::POINTED
            } else {
                OwnershipFlag::OWNED
            };
            // *ownership |= flag;
            v.push((*type_var_id, flag));
        }
        TyKind::Ref(_, ty, _) | TyKind::RawPtr(ty, _) => {
            let old_state = std::mem::replace(behind_a_pointer, true);
            ownership_behavior(ty.kind(), v, behind_a_pointer);
            *behind_a_pointer = old_state;
        }
        _ => (),
    }
}

bitflags::bitflags! {
    /// Ownership on a type param across potential multiple APIs.
    /// This means the type param can be both owned or pointed in different usages.
    #[derive(Default, Debug, Clone, Copy)]
    pub struct OwnershipFlag: u8 {
        // Owned:
        const OWNED = 0b01;
        // Pointed: behind a borrow or raw pointer
        const POINTED = 0b10;
    }
}
