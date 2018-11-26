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

pub mod ser;
pub mod de;
pub mod error;
pub mod serializable;

#[cfg(test)]
mod tests {
    use super::*;

//    #[serde(bound(serialize = "T: SVUID"))]
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
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

    #[derive(Serialize, Deserialize)]
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
        let mut b1 = SerializedBuffer::from_slice(&[228u8, 0, 0, 0, 3, 0, 0, 0]);
        assert_eq!(b0.as_ref(), b1.as_ref());
        let pack2 = de::from_stream::<Pack>(&mut b1).expect("failed to deserealize data");
        assert_eq!(pack, pack2);
    }
}
