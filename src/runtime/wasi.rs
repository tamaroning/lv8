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

        // use command line arguments after "--" as wasm arguments
        // the first argument is the module name
        builder.arg("this.wasm").unwrap();
        let mut saw_dhiphen = false;
        for arg in std::env::args() {
            if saw_dhiphen {
                builder = builder.arg(&arg).unwrap();
            } else if arg == "--" {
                saw_dhiphen = true;
                continue;
            }
        }

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

macro_rules! wasi_function {
    ($export:ident, $name:ident,  $( $arg_name: ident : $arg_ty: ty ),*) => {
        pub(super) fn $export(
            scope: &mut v8::HandleScope,
            _args: v8::FunctionCallbackArguments,
            mut rv: v8::ReturnValue,
        ) {
            let mut _argcnt = 0;
            $(
                let $arg_name = _args.get(_argcnt);
                let $arg_name = if $arg_name.is_big_int() {
                    let bigint = $arg_name.to_big_int(scope).unwrap();
                    bigint.i64_value().0
                } else {
                    $arg_name.integer_value(scope).unwrap()
                };
                //let $arg_name = $arg_name.integer_value(scope).unwrap_or_default() as $arg_ty;
                _argcnt += 1;
            )*


            let mut memory = get_memory_from_scope(scope);
            let mut wasi_ctx = get_wasi_ctx_mut().lock().unwrap();
            let result = TokioRuntime::new()
                .unwrap()
                .block_on(preview1::$name(
                    &mut *wasi_ctx,
                    &mut memory,
                    $( $arg_name as $arg_ty ),*
                ))
                .unwrap();

            rv.set(v8::Integer::new(scope, result).into());
        }
    }
}

wasi_function!(wasi_snapshot_preview1_args_get, args_get, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_args_sizes_get, args_sizes_get, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_clock_res_get, clock_res_get, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_clock_time_get,clock_time_get, arg0: i32, arg1: i64, arg2: i32);
wasi_function!(wasi_snapshot_preview1_environ_get, environ_get, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_environ_sizes_get,environ_sizes_get, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_fd_advise,fd_advise, arg0: i32, arg1: i64, arg2: i64, arg3: i32);
wasi_function!(wasi_snapshot_preview1_fd_allocate,fd_allocate, arg0: i32, arg1: i64, arg2: i64);
wasi_function!(wasi_snapshot_preview1_fd_close,fd_close, arg0: i32);
wasi_function!(wasi_snapshot_preview1_fd_datasync,fd_datasync, arg0: i32);
wasi_function!(wasi_snapshot_preview1_fd_fdstat_get,fd_fdstat_get, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_fd_fdstat_set_flags,fd_fdstat_set_flags, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_fd_fdstat_set_rights,fd_fdstat_set_rights, arg0: i32, arg1: i64, arg2: i64);
wasi_function!(wasi_snapshot_preview1_fd_filestat_get,fd_filestat_get, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_fd_filestat_set_size,fd_filestat_set_size, arg0: i32, arg1: i64);
wasi_function!(wasi_snapshot_preview1_fd_filestat_set_times,fd_filestat_set_times, arg0: i32, arg1: i64, arg2: i64, arg3: i32);
wasi_function!(wasi_snapshot_preview1_fd_pread,fd_pread, arg0: i32, arg1: i32, arg2: i32, arg3: i64, arg4: i32);
wasi_function!(wasi_snapshot_preview1_fd_prestat_dir_name,fd_prestat_dir_name, arg0: i32, arg1: i32, arg2: i32);
wasi_function!(wasi_snapshot_preview1_fd_prestat_get,fd_prestat_get, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_fd_pwrite,fd_pwrite, arg0: i32, arg1: i32, arg2: i32, arg3: i64, arg4: i32);
wasi_function!(wasi_snapshot_preview1_fd_read,fd_read, arg0: i32, arg1: i32, arg2: i32, arg3: i32);
wasi_function!(wasi_snapshot_preview1_fd_readdir,fd_readdir, arg0: i32, arg1: i32, arg2: i32, arg3: i64, arg4: i32);
wasi_function!(wasi_snapshot_preview1_fd_renumber,fd_renumber, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_fd_seek,fd_seek, arg0: i32, arg1: i64, arg2: i32, arg3: i32);
wasi_function!(wasi_snapshot_preview1_fd_sync,fd_sync, arg0: i32);
wasi_function!(wasi_snapshot_preview1_fd_tell,fd_tell, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_fd_write,fd_write, arg0: i32, arg1: i32, arg2: i32, arg3: i32);
wasi_function!(wasi_snapshot_preview1_path_create_directory,path_create_directory, arg0: i32, arg1: i32, arg2: i32);
wasi_function!(wasi_snapshot_preview1_path_filestat_get,path_filestat_get, arg0: i32, arg1: i32, arg2: i32, arg3 :i32, arg4: i32);
wasi_function!(wasi_snapshot_preview1_path_filestat_set_times,path_filestat_set_times, arg0: i32, arg1: i32, arg2: i32, arg3: i32, arg4: i64, arg5: i64, arg6: i32);
wasi_function!(wasi_snapshot_preview1_path_link,path_link, arg0: i32, arg1: i32, arg2: i32, arg3: i32, arg4: i32, arg5: i32, arg6: i32);
wasi_function!(wasi_snapshot_preview1_path_open,path_open, arg0: i32, arg1: i32, arg2: i32, arg3: i32, arg4: i32, arg5: i64, arg6: i64, arg7: i32, arg8:i32);
wasi_function!(wasi_snapshot_preview1_path_readlink,path_readlink, arg0: i32, arg1: i32, arg2: i32, arg3: i32, arg4: i32, arg5: i32);
wasi_function!(wasi_snapshot_preview1_path_remove_directory,path_remove_directory, arg0: i32, arg1: i32, arg2: i32);
wasi_function!(wasi_snapshot_preview1_path_rename,path_rename, arg0: i32, arg1: i32, arg2: i32, arg3: i32, arg4: i32, arg5: i32);
wasi_function!(wasi_snapshot_preview1_path_symlink,path_symlink, arg0: i32, arg1: i32, arg2: i32, arg3: i32, arg4: i32);
wasi_function!(wasi_snapshot_preview1_path_unlink_file,path_unlink_file, arg0: i32, arg1: i32, arg2: i32);
wasi_function!(wasi_snapshot_preview1_poll_oneoff,poll_oneoff, arg0: i32, arg1: i32, arg2: i32, arg3: i32);
// proc_exit
wasi_function!(wasi_snapshot_preview1_proc_raise,proc_raise, arg0: i32);
wasi_function!(wasi_snapshot_preview1_random_get,random_get, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_sched_yield, sched_yield,);
wasi_function!(wasi_snapshot_preview1_sock_accept,sock_accept, arg0: i32, arg1: i32, arg2: i32);
wasi_function!(wasi_snapshot_preview1_sock_recv,sock_recv, arg0: i32, arg1: i32, arg2: i32, arg3: i32, arg4: i32, arg5: i32);
wasi_function!(wasi_snapshot_preview1_sock_send,sock_send, arg0: i32, arg1: i32, arg2: i32, arg3: i32, arg4: i32);
wasi_function!(wasi_snapshot_preview1_sock_shutdown,sock_shutdown, arg0: i32, arg1: i32);

pub(super) fn wasi_snapshot_preview1_proc_exit(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _rv: v8::ReturnValue,
) {
    let arg0 = args.get(0);
    let arg0 = arg0.integer_value(scope).unwrap() as i32;
    std::process::exit(arg0);
}
