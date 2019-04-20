pub mod defines;
pub mod config;
pub use self::defines::{AM, AWM};

#[macro_export]
macro_rules! make_am {
    ($a:expr) => {
        Arc::new(Mutex::new($a))
    };
}

#[macro_export]
macro_rules! make_awm {
    ($a:expr) => {
        Weak::new(Mutex::new($a))
    };
}

#[macro_export]
macro_rules! lock_awm {
    ($v:expr, $locked:ident, $op:block) => {
        $v.upgrade().and_then(|arc| match arc.lock().and_then(|mut $locked| {
            $op
            Ok(())
        }).or_else(|e| {
            error!("lock poison error: {:?}", e);
            Err(e)
        }) {
            Ok(_) => Some(()),
            Err(_) => None
        });
    };
}
#[macro_export]
macro_rules! try_lock_awm {
    ($v:ident, $locked:ident, $op:block) => {
        $v.upgrade().and_then(|arc| match arc.try_lock().and_then(|mut $locked| {
            $op
            Ok(())
        }).or_else(|e| {
            error!("lock poison error: {:?}", e);
            Err(e)
        }) {
            Ok(_) => Some(()),
            Err(_) => None
        });
    };
}

//#[macro_export]
//macro_rules! lock_am {
//    ($v:ident, $locked:ident, $op:block) => {
//        $v.upgrade().and_then(|arc| match arc.lock().and_then(|mut $locked| {
//            $op
//            Ok(())
//        }).or_else(|e| {
//            error!("lock poison error: {:?}", e);
//            Err(e)
//        }) {
//            Ok(_) => Some(()),
//            Err(_) => None
//        });
//    };
//}