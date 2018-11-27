//! This crate provides Serde MTProto's two derive macros.
//!
//! ```
//! # #[macro_use] extern crate serde_pm_derive;
//! #[derive(PMIdentifiable, PMSized)]
//! # #[pm_identifiable(id = "0x00000000")]
//! # struct Stub;
//! # fn main() {}
//! ```
//!
//! # Examples
//!
//! ```
//! extern crate serde_pm;
//! #[macro_use]
//! extern crate serde_pm_derive;
//!
//! #[derive(PMIdentifiable, PMSized)]
//! #[pm_identifiable(id = "0xbeefdead")]
//! struct Message {
//!     message_id: u32,
//!     user_id: u32,
//!     text: String,
//!     attachment: Attachment,
//! }
//!
//! #[derive(PMIdentifiable, PMSized)]
//! enum Attachment {
//!     #[pm_identifiable(id = "0xdef19e00")]
//!     Nothing,
//!     #[pm_identifiable(id = "0xbadf00d0")]
//!     Link {
//!         url: String,
//!     },
//!     #[pm_identifiable(id = "0xdeafbeef")]
//!     Repost {
//!         message_id: u32,
//!     },
//! }
//!
//! # fn main() {}
//! ```

// For `quote!` used at the end of `impl_pm_identifiable`
#![recursion_limit = "87"]

extern crate proc_macro;

extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;


#[macro_use]
mod macros;

mod ast;
mod identifiable;
mod sized;


use proc_macro::TokenStream;

use identifiable::impl_pm_identifiable;
use sized::impl_pm_sized;


#[proc_macro_derive(PMIdentifiable, attributes(pm_identifiable))]
pub fn pm_identifiable(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let container = ast::Container::from_derive_input(ast)
        .expect("Cannot derive `pm::Identifiable` for unions.");

    impl_pm_identifiable(container).into()
}

#[proc_macro_derive(PMSized, attributes(pm_sized))]
pub fn pm_sized(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let container = ast::Container::from_derive_input(ast)
        .expect("Cannot derive `pm::PMSized` for unions.");

    impl_pm_sized(container).into()
}
