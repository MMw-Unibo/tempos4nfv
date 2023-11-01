use clap::Parser;
use std::{
    collections::HashMap,
    io::Write,
    net::{self, SocketAddr},
    sync::{atomic::AtomicBool, Arc},
    thread,
    time::{Duration, Instant},
};
use sysinfo::{CpuExt, System, SystemExt};
use wasmer::{imports, Engine, Imports, Instance, Module, Store, Value};

// mod invokers;

/// Simple TEMPOS Invoker example
#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Topic to send the message
    #[clap(short, long)]
    node: u32,
    #[clap(short, long)]
    topics: String,
    /// address to send the message to
    #[clap(short, long)]
    saddr: String,
    #[clap(short, long, default_value = "0")]
    test: u8,

    #[clap(short, long, default_value = "false")]
    warm: bool,
}

pub fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    let saddr: SocketAddr = args.saddr.parse()?;

    let address = std::env::var("INVKADDR")?;

    log::info!("Starting TEMPOS Invoker {} on {}", args.node, address);

    let sock = net::UdpSocket::bind(address)?;
    // sock.set_read_timeout(Some(Duration::from_millis(100)))?;
    sock.set_nonblocking(true)?;

    // init sysinfo
    let sys = System::new_all();

    let topics = args.topics.split(",").collect::<Vec<&str>>();
    for (i, topic) in topics.iter().enumerate() {
        log::debug!("registering topic: {}", topic);
        register_topic(topic, args.node, &sock, saddr)?;
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::Relaxed);
    })?;

    let r = running.clone();

    let args2 = args.clone();
    let main_thread = thread::spawn(move || {
        main_loop(r, sock, &args2, saddr);
    });

    let r = running.clone();

    // tokio::runtime::Builder::new_current_thread()
    //     .enable_all()
    //     .build()?
    //     .block_on(monitoring_loop(r, sys, args, addr));

    main_thread.join().unwrap();

    log::info!("exiting...");

    Ok(())
}

fn register_topic(
    topic: &str,
    node: u32,
    sock: &net::UdpSocket,
    saddr: SocketAddr,
) -> anyhow::Result<()> {
    let mut buf_send: Vec<u8> = Vec::with_capacity(1024);

    buf_send.write(&tempos::msg_type::REGISTRATION.to_be_bytes())?;
    buf_send.write(&node.to_be_bytes())?;

    let topic_len = topic.len() as u32;
    buf_send.write(&topic_len.to_be_bytes())?;
    buf_send.write(&topic.as_bytes())?;

    sock.send_to(&buf_send, saddr)?;

    Ok(())
}

async fn monitoring_loop(r: Arc<AtomicBool>, mut sys: System, args: Args, addr: SocketAddr) {
    let sock = tokio::net::UdpSocket::bind("127.0.0.1".parse::<SocketAddr>().unwrap())
        .await
        .unwrap();
    let mut buf = Vec::with_capacity(1024);
    while r.load(std::sync::atomic::Ordering::Relaxed) {
        sys.refresh_cpu();

        let mut load = 0.0;
        sys.cpus().iter().for_each(|cpu| load += cpu.cpu_usage());
        load = (load / sys.cpus().len() as f32) / 100.0;

        log::trace!("load: {}", load);

        buf.write(&tempos::msg_type::MONITORING.to_be_bytes())
            .unwrap();
        buf.write(&args.node.to_be_bytes()).unwrap();
        buf.write(&load.to_be_bytes()).unwrap();

        sock.send_to(&mut buf, addr).await.unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;

        buf.clear();
    }
}

struct WASMInvoker {
    engine: Engine,
    store: Store,
    module: Option<Module>,
    import_object: Option<Imports>,
    instance: Option<Instance>,
}

impl WASMInvoker {
    pub fn new() -> Self {
        let engine = Engine::headless();
        let store = Store::new(&engine);

        WASMInvoker {
            engine,
            store,
            module: None,
            import_object: None,
            instance: None,
        }
    }

    pub fn load(&mut self, path: &str) -> anyhow::Result<()> {
        self.unload();

        log::debug!("loading module: {}", path);

        let module = unsafe { Module::deserialize_from_file(&self.store, path)? };
        let import_object = imports! {};

        log::debug!("instantiating module: {}", path);

        let instance = Instance::new(&mut self.store, &module, &import_object)?;

        self.module = Some(module);
        self.import_object = Some(import_object);
        self.instance = Some(instance);

        Ok(())
    }

    pub fn unload(&mut self) {
        if let Some(instance) = &self.instance {
            drop(instance);
            self.instance = None;
        }

        if let Some(import_object) = &self.import_object {
            drop(import_object);
            self.import_object = None;
        }

        if let Some(module) = &self.module {
            drop(module);
            self.module = None;
        }
    }

    pub fn exec_function_by_name(&mut self, name: &str, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let instance = self.instance.as_ref().unwrap();
        let store = &mut self.store;

        let func = instance.exports.get_function(name)?;

        let memory = instance.exports.get_memory("memory")?;

        let memory_view = memory.view(store);
        let memory_view_size = memory_view.size();

        let max_offset = memory_view_size.bytes().0 / 8;
        memory.grow(store, 1).unwrap();

        let memory_view = memory.view(store);

        let data_offset = max_offset;
        memory_view.write(data_offset as u64, data)?;

        let out_offset = data_offset + data.len();

        let res = func.call(
            store,
            &[
                Value::I32(data_offset as i32),
                Value::I32(data.len() as i32),
                Value::I32(out_offset as i32),
            ],
        )?;

        log::debug!("invoked function: {}, with result: {:?}", name, res);

        let result = res[0].unwrap_i32();
        let mut buf = vec![0u8; result as usize];
        memory_view.read(out_offset as u64, &mut buf)?;

        Ok(buf)
    }
}

fn main_loop(r: Arc<AtomicBool>, sock: net::UdpSocket, args: &Args, addr: SocketAddr) {
    let mut initialized = false;
    let mut buf_send: Vec<u8> = Vec::with_capacity(2048);

    // NOTE(garbu):
    // function map where key is topic name and value is tuple of (function name, next topic name)
    // if next topic name is empty, then it means that this is the last function in the chain or that
    // the function has a side effect and does not return any data
    let mut functions_map = HashMap::new();
    functions_map.insert("vpn", ("comp", "enc"));
    functions_map.insert("enc", ("encrypt", "dec"));
    functions_map.insert("dec", ("decrypt", "dcp"));
    functions_map.insert("dcp", ("decomp", "out"));
    functions_map.insert("out", ("time", ""));

    let mut invoker = WASMInvoker::new();

    let mut trace_interval = Instant::now();

    let mut buf_recv = [0u8; 2048];
    log::debug!("starting main loop");
    println!("id,func,ts_start,ts_end");
    while r.load(std::sync::atomic::Ordering::Relaxed) {
        match sock.recv_from(&mut buf_recv) {
            Ok((size, _)) => {
                let start_ns = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                let mut read_ptr = 0;
                let msg_type = u8::from_be_bytes([buf_recv[read_ptr]]);
                read_ptr += 1;
                match msg_type {
                    tempos::msg_type::INVOK => {
                        let msg_seq = u32::from_be_bytes([
                            buf_recv[read_ptr],
                            buf_recv[read_ptr + 1],
                            buf_recv[read_ptr + 2],
                            buf_recv[read_ptr + 3],
                        ]);

                        read_ptr += 4;
                        let topic_len = u32::from_be_bytes([
                            buf_recv[read_ptr],
                            buf_recv[read_ptr + 1],
                            buf_recv[read_ptr + 2],
                            buf_recv[read_ptr + 3],
                        ]);
                        read_ptr += 4;

                        let topic =
                            std::str::from_utf8(&buf_recv[read_ptr..read_ptr + topic_len as usize])
                                .unwrap();
                        read_ptr += topic_len as usize;

                        log::debug!("invoking topic: {}", topic);

                        if !args.warm || (args.warm && !initialized) {
                            invoker.load("final.so").unwrap();
                            initialized = true;
                        }

                        let (function_name, out_topic) = functions_map.get(topic).unwrap();
                        let data_len = u32::from_be_bytes([
                            buf_recv[read_ptr],
                            buf_recv[read_ptr + 1],
                            buf_recv[read_ptr + 2],
                            buf_recv[read_ptr + 3],
                        ]);

                        read_ptr += 4;

                        let data = &buf_recv[read_ptr..read_ptr + data_len as usize];

                        if function_name.eq_ignore_ascii_case("time") {
                            let now_ns = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_nanos();
                            println!("{},{},0,{}", msg_seq, function_name, now_ns);
                            continue;
                        }

                        let data_str = String::from_utf8(data.to_vec())
                            .unwrap_or("Unable to convert to string".to_string());
                        log::debug!(
                            "invoking function: {}, with data {:?}",
                            function_name,
                            data_str
                        );

                        if let Ok(output) = invoker.exec_function_by_name(function_name, data) {
                            if let Ok(output_str) = String::from_utf8(output.to_vec()) {
                                log::debug!("output: {:?}", output_str);
                            } else {
                                log::debug!("output: {:?}", output);
                            }

                            if !out_topic.is_empty() {
                                buf_send.clear();
                                buf_send
                                    .write(&tempos::msg_type::INVOK.to_be_bytes())
                                    .unwrap();
                                buf_send.write(&msg_seq.to_be_bytes()).unwrap();

                                let out_topic_len = out_topic.len() as u32;
                                buf_send.write(&out_topic_len.to_be_bytes()).unwrap();
                                buf_send.write(&out_topic.as_bytes()).unwrap();

                                let output_len = output.len() as u32;
                                buf_send.write(&output_len.to_be_bytes()).unwrap();
                                buf_send.write(&output).unwrap();

                                log::trace!("sending message: {:?}", &buf_send);

                                if args.test == 1 {
                                    let end_ns = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_nanos();
                                    println!(
                                        "{},{},{},{}",
                                        msg_seq, function_name, start_ns, end_ns
                                    );
                                }

                                sock.send_to(&buf_send, addr).unwrap();
                            }

                            continue;
                        }

                        log::error!("failed to invoke function: {}", function_name);
                    }
                    _ => log::debug!("Unhandled message type"),
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if initialized {
                    invoker.unload();
                    initialized = false;
                    log::debug!("Unloading WASM module due to timeout");
                }

                std::thread::sleep(std::time::Duration::from_micros(10));
            }
            Err(e) => {
                println!("encountered IO error: {}", e);
            }
        }
    }

    buf_send.clear();
    buf_send
        .write(&tempos::msg_type::UNREGISTRATION.to_be_bytes())
        .unwrap();
    buf_send.write(&args.node.to_be_bytes()).unwrap();

    sock.send_to(&buf_send, addr).unwrap();
}
