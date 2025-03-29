#![allow(dead_code)]

mod trait_did;
pub use trait_did::{Krate, TraitDid};

mod adt_generic_params;
pub use adt_generic_params::AdtGenericParams;

mod generics_mapping;
pub use generics_mapping::self_type;
