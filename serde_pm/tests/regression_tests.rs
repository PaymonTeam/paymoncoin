extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_pm as serde_pm_other_name;    // Tests `serde_pm_derive`
#[macro_use]
extern crate serde_pm_derive;
#[macro_use]
extern crate log;
extern crate env_logger;

use serde::ser::{self};
use serde::de::{self, Deserializer, DeserializeSeed, Error as DeError};
use serde_pm_other_name::{Boxed, PMSized, from_stream, to_buffer};
use serde_pm_other_name::serializable::*;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, PMIdentifiable, PMSized)]
enum Algebraic {
//    #[pm_identifiable(id = "0xaaaaaaaa")]
    #[pm_identifiable(id = "0xbbbbbbbb")]
    A,
    #[pm_identifiable(id = "0xbbbbbbbb")]
    B(u32),
    #[pm_identifiable(id = "0xcccccccc")]
    C(String)
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, PMIdentifiable, PMSized)]
#[pm_identifiable(id = "0xacacacac")]
struct Pack {
    v: i32,
    s: String,
    variant: Boxed<Algebraic>,
}

impl Pack {
    pub fn new() -> Self {
        Pack {
            v: i32::default(),
            s: String::default(),
            variant: Boxed::new(Algebraic::A)
        }
    }
}

fn init_log() {
    use env_logger::LogBuilder;
    use log::{LogRecord, LogLevelFilter};
    use std::env;

    let format = |record: &LogRecord| {
        format!("[{}]: {}", record.level(), record.args())
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
}

#[test]
fn it_works() {
    init_log();

    let enum_ids = [ "", ];

    let mut pack = Pack::new();
    pack.v = 3;
    pack.s = "hello".into();
    pack.variant = Boxed::new(Algebraic::B(4));

    let b0 = to_buffer(&pack).expect("failed to serialize data");
    debug!("b0={:?}", b0.as_ref());
    let mut b1 = SerializedBuffer::from_slice(&[3, 0, 0, 0, 5, 104, 101, 108, 108, 111, 0, 0, 187, 187, 187, 187, 4, 0, 0, 0]);
//        let mut b1 = SerializedBuffer::from_slice(&[228u8, 0, 0, 0, 3, 0, 0, 0]);

    assert_eq!(b0.as_ref(), b1.as_ref());
    let pack2 = from_stream::<Pack>(&mut b1, &enum_ids).expect("failed to deserealize data");
    assert_eq!(pack, pack2);
    debug!("{:?}", pack2);
}
