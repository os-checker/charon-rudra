#![allow(dead_code)]
use charon_lib::ast::*;

mod adt_generic_params;

#[derive(Clone, Copy)]
pub struct Krate<'a> {
    krate: &'a TranslatedCrate,
}

impl<'a> Krate<'a> {
    pub fn new(data: &TranslatedCrate) -> Krate {
        Krate { krate: data }
    }

    /// Get def id of a trait.
    pub fn trait_def_id(self, name: &[&str]) -> Option<TraitDeclId> {
        self.krate
            .trait_decls
            .iter()
            .find(|t| t.item_meta.name.equals_ref_name(name))
            .map(|t| t.def_id)
    }

    /// Decl ids of Send, Sync, and Copy.
    pub fn send_sync_copy(self) -> TraitDid {
        TraitDid {
            send: self.trait_def_id(SEND),
            sync: self.trait_def_id(SYNC),
            copy: self.trait_def_id(COPY),
        }
    }

    /// All local impls of a trait.
    pub fn trait_impls(self, trait_did: Option<TraitDeclId>) -> Vec<&'a TraitImpl> {
        let Some(trait_did) = trait_did else {
            return vec![];
        };
        self.krate
            .trait_impls
            .iter()
            .filter(|i| i.impl_trait.trait_id == trait_did)
            .collect()
    }
}

const SEND: &[&str] = &["core", "marker", "Send"];
const SYNC: &[&str] = &["core", "marker", "Sync"];
const COPY: &[&str] = &["core", "marker", "Copy"];

/// Trait decl id in a crate. None if the trait doesn't appear in the crate.
#[derive(Debug, Clone)]
pub struct TraitDid {
    pub send: Option<TraitDeclId>,
    pub sync: Option<TraitDeclId>,
    pub copy: Option<TraitDeclId>,
}
