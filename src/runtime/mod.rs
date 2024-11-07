mod wasi;

use anyhow::{anyhow, Result};

use crate::driver::{self, Cli};

macro_rules! import_wasi_function {
    ($scope:expr, $import_wasi_p1:expr, $import_name:expr, $fn_name:ident) => {
        let $fn_name = v8::FunctionTemplate::new($scope, wasi::$fn_name);
        let $fn_name = $fn_name.get_function($scope).unwrap();
        let value_name = v8::String::new($scope, $import_name).unwrap().into();
        $import_wasi_p1.set($scope, value_name, $fn_name.into());
    };
}

pub fn run(args: &Cli) -> Result<i32> {
    init_v8();
    let mut runtime = create_runtime(args)?;
    runtime.run()
}

struct Runtime {
    isolate: v8::OwnedIsolate,
    wasm_instance: v8::Global<v8::Object>,
}

impl Runtime {
    fn run(&mut self) -> Result<i32> {
        let scope = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let wasm_instance = self.wasm_instance.open(scope);

        let str_exports = v8::String::new(scope, "exports").unwrap();
        let exports = wasm_instance.get(scope, str_exports.into()).unwrap();
        let exports = exports.to_object(scope).unwrap();

        let str_start = v8::String::new(scope, "_start").unwrap();
        let Some(start) = exports.get(scope, str_start.into()) else {
            return Err(anyhow!("Wasm module does not export _start function"));
        };
        let start = start.cast::<v8::Function>();

        // call instance.exports._start()
        let ret = start.call(scope, exports.into(), &[]).unwrap();
        if ret.type_repr() == "undefined" {
            Ok(0)
        } else if ret.type_repr() == "number" {
            if let Some(code) = ret.to_int32(scope) {
                Ok(code.value())
            } else {
                Err(anyhow!("Wasm module exited with non-i32 number"))
            }
        } else {
            Err(anyhow!(
                "Wasm module exited with value of type {}",
                ret.type_repr()
            ))
        }
    }
}

fn init_v8() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
}

fn create_runtime(args: &driver::Cli) -> Result<Runtime> {
    let mut isolate = v8::Isolate::new(Default::default());
    let instance = {
        let scope = &mut v8::HandleScope::new(&mut isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let wasm_module = std::fs::read(&args.wasmfile_path).expect("Failed to read file");

        let module = v8::WasmModuleObject::compile(scope, &wasm_module).unwrap();

        let import_object = v8::Object::new(scope);
        let global = context.global(scope);
        let str_wasm = v8::String::new(scope, "WebAssembly").unwrap();
        let global_wasm = global
            .get(scope, str_wasm.into())
            .unwrap()
            .to_object(scope)
            .unwrap();

        // prepare imports.wasi_snapshot_preview1
        create_wasip1_import(scope, &import_object);

        let str2 = v8::String::new(scope, "Instance").unwrap();
        let instance_ctor = global_wasm.get(scope, str2.into()).unwrap();
        let instance_ctor = instance_ctor.cast::<v8::Function>();
        let instance = instance_ctor
            .new_instance(scope, &[module.into(), import_object.into()])
            .unwrap();

        // set instance to global
        let str_ginstance = v8::String::new(scope, "gInstance").unwrap();
        global.set(scope, str_ginstance.into(), instance.into());

        v8::Global::new(scope, instance)
    };

    Ok(Runtime {
        isolate,
        wasm_instance: instance,
    })
}

fn create_wasip1_import<'a>(
    scope: &'a mut v8::HandleScope,
    import_object: &v8::Local<'a, v8::Object>,
) {
    let import_wasi_p1 = v8::Object::new(scope);
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "args_get",
        wasi_snapshot_preview1_args_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "args_sizes_get",
        wasi_snapshot_preview1_args_sizes_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "environ_get",
        wasi_snapshot_preview1_environ_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "environ_sizes_get",
        wasi_snapshot_preview1_environ_sizes_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_prestat_get",
        wasi_snapshot_preview1_fd_prestat_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_prestat_dir_name",
        wasi_snapshot_preview1_fd_prestat_dir_name
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_open",
        wasi_snapshot_preview1_path_open
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_filestat_get",
        wasi_snapshot_preview1_path_filestat_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_readlink",
        wasi_snapshot_preview1_path_readlink
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_rename",
        wasi_snapshot_preview1_path_rename
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_remove_directory",
        wasi_snapshot_preview1_path_remove_directory
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_create_directory",
        wasi_snapshot_preview1_path_create_directory
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_fdstat_get",
        wasi_snapshot_preview1_fd_fdstat_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_close",
        wasi_snapshot_preview1_fd_close
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_write",
        wasi_snapshot_preview1_fd_write
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_read",
        wasi_snapshot_preview1_fd_read
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_seek",
        wasi_snapshot_preview1_fd_seek
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_filestat_get",
        wasi_snapshot_preview1_fd_filestat_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_filestat_set_size",
        wasi_snapshot_preview1_fd_filestat_set_size
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_filestat_set_times",
        wasi_snapshot_preview1_fd_filestat_set_times
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_allocate",
        wasi_snapshot_preview1_fd_allocate
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_fdstat_set_flags",
        wasi_snapshot_preview1_fd_fdstat_set_flags
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_fdstat_set_rights",
        wasi_snapshot_preview1_fd_fdstat_set_rights
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "clock_time_get",
        wasi_snapshot_preview1_clock_time_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "clock_res_get",
        wasi_snapshot_preview1_clock_res_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "poll_oneoff",
        wasi_snapshot_preview1_poll_oneoff
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "random_get",
        wasi_snapshot_preview1_random_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "proc_raise",
        wasi_snapshot_preview1_proc_raise
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_advise",
        wasi_snapshot_preview1_fd_advise
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_datasync",
        wasi_snapshot_preview1_fd_datasync
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_symlink",
        wasi_snapshot_preview1_path_symlink
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_readdir",
        wasi_snapshot_preview1_fd_readdir
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_renumber",
        wasi_snapshot_preview1_fd_renumber
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_sync",
        wasi_snapshot_preview1_fd_sync
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_tell",
        wasi_snapshot_preview1_fd_tell
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_pwrite",
        wasi_snapshot_preview1_fd_pwrite
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_pread",
        wasi_snapshot_preview1_fd_pread
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "fd_advise",
        wasi_snapshot_preview1_fd_advise
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_link",
        wasi_snapshot_preview1_path_link
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_filestat_set_times",
        wasi_snapshot_preview1_path_filestat_set_times
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_create_directory",
        wasi_snapshot_preview1_path_create_directory
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_filestat_get",
        wasi_snapshot_preview1_path_filestat_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_readlink",
        wasi_snapshot_preview1_path_readlink
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_rename",
        wasi_snapshot_preview1_path_rename
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_remove_directory",
        wasi_snapshot_preview1_path_remove_directory
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_symlink",
        wasi_snapshot_preview1_path_symlink
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "path_unlink_file",
        wasi_snapshot_preview1_path_unlink_file
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "poll_oneoff",
        wasi_snapshot_preview1_poll_oneoff
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "proc_exit",
        wasi_snapshot_preview1_proc_exit
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "proc_raise",
        wasi_snapshot_preview1_proc_raise
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "random_get",
        wasi_snapshot_preview1_random_get
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "sched_yield",
        wasi_snapshot_preview1_sched_yield
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "sock_accept",
        wasi_snapshot_preview1_sock_accept
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "sock_recv",
        wasi_snapshot_preview1_sock_recv
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "sock_send",
        wasi_snapshot_preview1_sock_send
    );
    import_wasi_function!(
        scope,
        import_wasi_p1,
        "sock_shutdown",
        wasi_snapshot_preview1_sock_shutdown
    );

    let str_wasip1 = v8::String::new(scope, "wasi_snapshot_preview1").unwrap();
    import_object.set(scope, str_wasip1.into(), import_wasi_p1.into());
}

/*
fn to_pretty_string(mut try_catch: v8::TryCatch<v8::HandleScope>) -> String {
    // TODO (enhancement): better error handling needed! wanna remove uncareful unwrap().
    let exception_string = try_catch
        .exception()
        .unwrap()
        .to_string(&mut try_catch)
        .unwrap()
        .to_rust_string_lossy(&mut try_catch);
    let message = try_catch.message().unwrap();

    let filename = message
        .get_script_resource_name(&mut try_catch)
        .map_or_else(
            || "(unknown)".into(),
            |s| {
                s.to_string(&mut try_catch)
                    .unwrap()
                    .to_rust_string_lossy(&mut try_catch)
            },
        );
    let line_number = message.get_line_number(&mut try_catch).unwrap_or_default();
    format!("{}:{}: {}", filename, line_number, exception_string)
}
*/

/*

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
wasi_function!(wasi_snapshot_preview1_proc_raise,proc_raise, arg0: i32);
wasi_function!(wasi_snapshot_preview1_random_get,random_get, arg0: i32, arg1: i32);
wasi_function!(wasi_snapshot_preview1_sched_yield, sched_yield,);
wasi_function!(wasi_snapshot_preview1_sock_accept,sock_accept, arg0: i32, arg1: i32, arg2: i32);
wasi_function!(wasi_snapshot_preview1_sock_recv,sock_recv, arg0: i32, arg1: i32, arg2: i32, arg3: i32, arg4: i32, arg5: i32);
wasi_function!(wasi_snapshot_preview1_sock_send,sock_send, arg0: i32, arg1: i32, arg2: i32, arg3: i32, arg4: i32);
wasi_function!(wasi_snapshot_preview1_sock_shutdown,sock_shutdown, arg0: i32, arg1: i32);

*/
