extern crate serde;

pub use self::serializable::*;
pub use self::error::{Error, Result};

pub fn serialize<T: Serializable>(obj: &T) -> Result<SerializedBuffer> {
    get_serialized_object(obj, true)
}

pub fn deserialize<T: Serializable>(stream: &mut SerializedBuffer) -> Result<T> {
    let mut obj = T::default();
    obj.read_params(stream)?;
    Ok(obj)
}

//#[proc_macro_derive(PMSerializable)]
//pub fn pm_serializable_macro_derive(input: TokenStream) -> TokenStream {
//    let ast = syn::parse(input).unwrap();
//    impl_serialize(&ast)
//}
//
//fn impl_serialize(ast: &syn::DeriveInput) -> TokenStream {
//    let name = &ast.ident;
//    let gen = quote! {
//        impl Serializable for #name {
//            fn ser() {
//                println!("Hello, Macro! My name is {}", stringify!(#name));
//            }
//        }
//    };
//    gen.into()
//}

pub mod ser;
pub mod de;
pub mod error;
pub mod serializable;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(PMSerializable)]
    struct Pack {
        v: i32
    }

    #[test]
    fn it_works() {
        let mut pack = Pack { v: 3 };
        assert_eq!(serialize(pack), SerializedBuffer::from_slice(&[1, 2, 3]));
    }
}
