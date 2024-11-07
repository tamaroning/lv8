use anyhow::{anyhow, Result};
use std::{
    cell::UnsafeCell,
    sync::{Mutex, OnceLock},
};
use tokio::runtime::Runtime as TokioRuntime;
use wasi_common::snapshots::preview_1::wasi_snapshot_preview1 as preview1;
use wasi_common::sync::WasiCtxBuilder;
use wasi_common::WasiCtx;

use crate::driver::{self, Cli};

/// Global WASI context
static WASI_CTX: OnceLock<Mutex<WasiCtx>> = OnceLock::new();

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
        let import_wasi_p1 = v8::Object::new(scope);
        let func_template = v8::FunctionTemplate::new(scope, fd_write);
        let func = func_template.get_function(scope).unwrap();
        // set function to import object
        let str_fd_write = v8::String::new(scope, "fd_write").unwrap();
        import_wasi_p1.set(scope, str_fd_write.into(), func.into());

        let str_wasi_p1 = v8::String::new(scope, "wasi_snapshot_preview1").unwrap();
        import_object.set(scope, str_wasi_p1.into(), import_wasi_p1.into());

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

fn fd_write(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
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
    let mut memory = wiggle::GuestMemory::Shared(memory);

    let arg0 = args.get(0);
    let arg0 = arg0.integer_value(scope).unwrap_or_default() as i32;
    let arg1 = args.get(1);
    let arg1 = arg1.integer_value(scope).unwrap_or_default() as i32;
    let arg2 = args.get(2);
    let arg2 = arg2.integer_value(scope).unwrap_or_default() as i32;
    let arg3 = args.get(3);
    let arg3 = arg3.integer_value(scope).unwrap_or_default() as i32;

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
