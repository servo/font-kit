// font-kit/c/src/lib.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use font_kit::handle::Handle;
use std::mem;
use std::slice;
use std::sync::Arc;

pub type FKDataRef = *const Vec<u8>;
pub type FKHandleRef = *mut Handle;

#[no_mangle]
pub unsafe extern "C" fn FKDataCreate(bytes: *const u8, len: usize) -> FKDataRef {
    Arc::into_raw(Arc::new(slice::from_raw_parts(bytes, len).to_vec()))
}

#[no_mangle]
pub unsafe extern "C" fn FKDataDestroy(data: FKDataRef) {
    drop(Arc::from_raw(data))
}

/// Does not take ownership of `bytes`.
#[no_mangle]
pub unsafe extern "C" fn FKHandleCreateWithMemory(bytes: FKDataRef, font_index: u32)
                                                  -> FKHandleRef {
    let bytes = Arc::from_raw(bytes);
    mem::forget(bytes.clone());
    Box::into_raw(Box::new(Handle::from_memory(bytes, font_index)))
}

#[no_mangle]
pub unsafe extern "C" fn FKHandleDestroy(handle: FKHandleRef) {
    drop(Box::from_raw(handle))
}
