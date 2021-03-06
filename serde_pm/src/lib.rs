#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate num_traits;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_pm_derive;
#[macro_use]
extern crate log;

pub mod ser;
pub mod de;
pub mod error;
pub mod serializable;
pub mod identifiable;
pub mod sized;
pub mod utils;
pub mod wrappers;

pub use ser::{
    Serializer,
    to_buffer,
    to_boxed_buffer,
    to_buffer_with_padding,
};

pub use de::{
    Deserializer,
    from_stream,
};

// Error types and typedefs
pub use error::{Error, SerializationError, Result};

// Other items generally useful for PM [de]serialization
//pub use helpers::{UnsizedByteBuf, UnsizedByteBufSeed};
pub use identifiable::Identifiable;
pub use sized::{PMSized, size_hint_from_byte_seq_len};
pub use serializable::SerializedBuffer;
pub use wrappers::{Boxed, WithId};
pub use self::serializable::*;
