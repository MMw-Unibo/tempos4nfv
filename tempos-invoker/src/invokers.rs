use libloading::{Library, Symbol};
use std::{
    os::raw::c_uchar,
    process::{Command, Stdio},
};
use wasmer::{imports, Instance, Module, Store, Value};

pub type FuncMain = unsafe fn(c_uchar) -> c_uchar;

pub fn dyn_load(opts: &Invkopts, _store: &Store) {
    let v: Vec<&str> = opts.func.split('/').collect();
    let (lib_name, func_name) = (v[0], v[1]);

    unsafe {
        let lib = Library::new(format!("./{}.so", lib_name)).unwrap();
        let func: Symbol<AddFunc> = lib.get(func_name.as_bytes()).unwrap();
        let _ = func(opts.multiplicator, 0);
    }
}

pub fn dyn_wasm(opts: &Invkopts, store: &Store) {
    let v: Vec<&str> = opts.func.split('/').collect();
    let (lib_name, func_name) = (v[0], v[1]);

    let module =
        unsafe { Module::deserialize_from_file(store, format!("./{}.so", lib_name)).unwrap() };

    let import_object = imports! {};

    let instance = Instance::new(&module, &import_object).unwrap();

    let test_function = instance.exports.get_function(func_name).unwrap();

    let _ = test_function
        .call(&[Value::I32(opts.multiplicator), Value::I32(0)])
        .unwrap();
}

// pub fn fork_compiled(opts: &Invkopts, _store: &Store) {
//     match Command::new(format!("./{}", opts.func))
//         .arg(opts.multiplicator.to_string())
//         .stdout(Stdio::piped())
//         .spawn()
//     {
//         Ok(mut child) => {
//             child.wait().unwrap();
//         }
//         Err(_) => {
//             log::error!(
//                 "impossible to execute command: {:?}",
//                 std::io::Error::last_os_error()
//             );
//         }
//     };
// }

// pub fn fork_python(opts: &Invkopts, _store: &Store) {
//     match Command::new("python3")
//         .arg(&opts.func)
//         .arg(opts.multiplicator.to_string())
//         .stdout(Stdio::piped())
//         .spawn()
//     {
//         Ok(mut child) => {
//             child.wait().unwrap();
//         }
//         Err(_) => {
//             log::error!(
//                 "impossible to execute command: {:?}",
//                 std::io::Error::last_os_error()
//             );
//         }
//     };
// }

// pub fn fork_node(opts: &Invkopts, _store: &Store) {
//     match Command::new("node")
//         .arg(&opts.func)
//         .arg(opts.multiplicator.to_string())
//         .stdout(Stdio::piped())
//         .spawn()
//     {
//         Ok(mut child) => {
//             child.wait().unwrap();
//         }
//         Err(_) => {
//             log::error!(
//                 "impossible to execute command: {:?}",
//                 std::io::Error::last_os_error()
//             );
//         }
//     };
// }
