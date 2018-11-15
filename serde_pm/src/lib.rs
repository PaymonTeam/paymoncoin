#[macro_use]
extern crate serde_derive;
extern crate serde;
#[macro_use]
extern crate log;
extern crate env_logger;

pub use self::serializable::*;
pub use self::error::{Error, Result};

trait SVUID {
    fn svuid() -> i32;
}

//pub fn serialize<T: Serializable>(obj: &T) -> Result<SerializedBuffer> {
//    get_serialized_object(obj, true)
//}

//pub fn deserialize<T: Serializable>(stream: &mut SerializedBuffer) -> Result<T> {
//    let mut obj = T::default();
//    obj.read_params(stream)?;
//    Ok(obj)
//}

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

//    #[serde(bound(serialize = "T: SVUID"))]
    #[derive(Serialize)]
    struct Pack {
        svuid: i32,
        v: i32
    }

    impl Pack {
        pub fn new() -> Self {
            Pack {
                svuid: <Self as SVUID>::svuid(),
                v: i32::default()
            }
        }
    }

    impl SVUID for Pack {
        fn svuid() -> i32 {
            228
        }
    }

    #[derive(Serialize, SVUID(228))]
    struct AutoPack {
        v: i32
    }

    #[test]
    fn it_works() {
        use env_logger::LogBuilder;
        use log::{LogRecord, LogLevelFilter};
        use std::env;

        let format = |record: &LogRecord| {
            format!("[{}]: {}", record.level(), record.args())
//        format!("[{} {:?}]: {}", record.level(), thread::current().id(), record.args())
        };

        let mut builder = LogBuilder::new();
        builder.format(format)
            .filter(None, LogLevelFilter::Info)
            .filter(Some("futures"), LogLevelFilter::Error)
            .filter(Some("tokio"), LogLevelFilter::Error)
            .filter(Some("tokio-io"), LogLevelFilter::Error)
            .filter(Some("hyper"), LogLevelFilter::Error)
            .filter(Some("iron"), LogLevelFilter::Error);

        if env::var("RUST_LOG").is_ok() {
            builder.parse(&env::var("RUST_LOG").unwrap());
        }

        builder.init().unwrap();

        let mut pack = Pack::new();
        pack.v = 3;
        let b0 = ser::to_buffer(&pack).expect("failed to serialize data");
        let b1 = SerializedBuffer::from_slice(&[228u8, 0, 0, 0, 3, 0, 0, 0]);
        assert_eq!(b0.as_ref(), b1.as_ref());
    }
}
