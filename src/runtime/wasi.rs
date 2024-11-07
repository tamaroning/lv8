use std::{
    cell::UnsafeCell,
    sync::{Mutex, OnceLock},
};
use tokio::runtime::Runtime as TokioRuntime;
use wasi_common::snapshots::preview_1::wasi_snapshot_preview1 as preview1;
use wasi_common::sync::WasiCtxBuilder;
use wasi_common::WasiCtx;
use wiggle::GuestMemory;

/// Global WASI context
static WASI_CTX: OnceLock<Mutex<WasiCtx>> = OnceLock::new();

fn get_wasi_ctx_mut() -> &'static Mutex<WasiCtx> {
    WASI_CTX.get_or_init(|| {
        let mut builder = WasiCtxBuilder::new();
        let mut builder = builder.inherit_stdin().inherit_stdout().inherit_stderr();
        //let mut buider = builder.inherit_args();
        builder = builder.inherit_env().unwrap();
        let dir = cap_std::fs::Dir::from_std_file(std::fs::File::open(".").unwrap());
        builder = builder.preopened_dir(dir, "/").unwrap();
        Mutex::new(builder.build())
    })
}

fn get_memory_from_scope<'a>(scope: &'a mut v8::HandleScope) -> GuestMemory<'a> {
    // get global object
    let context = scope.get_current_context();
    let global = context.global(scope);
    // access to global memory
    let str_instance = v8::String::new(scope, "gInstance").unwrap();
    let instance = global.get(scope, str_instance.into()).unwrap();
    let instance = instance.to_object(scope).unwrap();

    // access to instance.exports.memory.buffer
    let str_exports = v8::String::new(scope, "exports").unwrap();
    let exports = instance.get(scope, str_exports.into()).unwrap();
    let exports = exports.to_object(scope).unwrap();
    let str_memory = v8::String::new(scope, "memory").unwrap();
    let memory = exports.get(scope, str_memory.into()).unwrap();
    let memory = memory.to_object(scope).unwrap();

    // cast memory.buffer to ArrayBuffer
    let str_buffer = v8::String::new(scope, "buffer").unwrap();
    let array_buffer = memory.get(scope, str_buffer.into()).unwrap();
    let array_buffer = array_buffer.cast::<v8::ArrayBuffer>();
    let backing_store = array_buffer.get_backing_store();
    let memory: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(
            backing_store.data().unwrap().as_ptr() as *mut u8,
            backing_store.byte_length(),
        )
    };
    let memory = unsafe { &*(memory as *mut [u8] as *mut [UnsafeCell<u8>]) };
    wiggle::GuestMemory::Shared(memory)
}

pub(super) fn fd_write(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let arg0 = args.get(0);
    let arg0 = arg0.integer_value(scope).unwrap() as i32;
    let arg1 = args.get(1);
    let arg1 = arg1.integer_value(scope).unwrap() as i32;
    let arg2 = args.get(2);
    let arg2 = arg2.integer_value(scope).unwrap() as i32;
    let arg3 = args.get(3);
    let arg3 = arg3.integer_value(scope).unwrap() as i32;

    let mut memory = get_memory_from_scope(scope);
    let mut wasi_ctx = get_wasi_ctx_mut().lock().unwrap();
    let result = TokioRuntime::new()
        .unwrap()
        .block_on(preview1::fd_write(
            &mut *wasi_ctx,
            &mut memory,
            arg0,
            arg1,
            arg2,
            arg3,
        ))
        .unwrap();

    rv.set(v8::Integer::new(scope, result).into());
}
