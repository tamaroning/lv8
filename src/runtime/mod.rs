mod wasi;

use anyhow::{anyhow, Result};

use crate::driver::{self, Cli};

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
        let fdwrite_template = v8::FunctionTemplate::new(scope, wasi::fd_write);
        let fdwrite_template = fdwrite_template.get_function(scope).unwrap();
        // set function to import object
        let str_fd_write = v8::String::new(scope, "fd_write").unwrap();
        import_wasi_p1.set(scope, str_fd_write.into(), fdwrite_template.into());

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
