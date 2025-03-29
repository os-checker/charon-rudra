use charon_lib::ast::*;
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

/// Type param and analysis basis on a adt.
type Args = IndexMap<TypeVarId, ParamInfo>;

/// Generic param on adt.
pub struct AdtGenericParam {
    /// Type id of adt.
    pub tid: TypeDeclId,
    /// The map order corresponds to the position of adt's generic params.
    ///
    /// TypeVarId is the generic type id on a specific adt.
    pub args: Args,
}

impl AdtGenericParam {
    pub fn new(krate: &TranslatedCrate, tid: TypeDeclId) -> Self {
        let adt = &krate.type_decls[tid];

        let generic_types = adt.generics.types.iter();
        let mut args: Args = generic_types
            .map(|t| (t.index, ParamInfo::default()))
            .collect();

        adt_trait_bounds(adt, &mut args);

        // type params ownership behaviors from adt decl
        for field in fields(&adt.kind) {
            ownership_behavior(field.ty.kind(), &mut args, &mut false);
        }

        AdtGenericParam { tid, args }
    }
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

fn ownership_behavior(ty_kind: &TyKind, args: &mut Args, behind_a_pointer: &mut bool) {
    match ty_kind {
        TyKind::Adt(_, generics) => {
            for ty in generics.types.iter() {
                ownership_behavior(ty.kind(), args, behind_a_pointer);
            }
        }
        TyKind::TypeVar(type_var_id) => {
            let ownership = &mut args.get_mut(type_var_id).unwrap().ownership_behavior;
            let flag = if *behind_a_pointer {
                OwnershipFlag::POINTED
            } else {
                OwnershipFlag::OWNED
            };
            *ownership |= flag;
        }
        TyKind::Ref(_, ty, _) | TyKind::RawPtr(ty, _) => {
            *behind_a_pointer = true;
            ownership_behavior(ty.kind(), args, behind_a_pointer);
            *behind_a_pointer = false;
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
