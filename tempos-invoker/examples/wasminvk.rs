use std::{fs::File, io::Write, thread, time::Duration};

use wasmer::{imports, Engine, Function, Instance, Module, Store, Value};

#[derive(Debug, Default, Clone, Copy)]
struct MessageMetric {
    id: usize,
    int: u128,
    ts_start: u128,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        println!("Usage: {} <id> <w> <msg>", args[0]);
        return Ok(());
    }

    let id = args[1].parse::<usize>().unwrap();
    let w: bool = args[2].parse::<bool>().unwrap();
    let n_message = args[3].parse::<usize>().unwrap();

    // create a single producer, single consumer channel
    let (tx, rx) = std::sync::mpsc::channel();

    // spawn a thread to send a message
    let thread = thread::spawn(move || {
        let mut sleep_time = 1000;
        let interval = n_message / 10;

        let bytes = std::fs::read("apps/vpn/src/lib.rs").unwrap();

        let mut file = File::create(format!("send{}.csv", id)).unwrap();
        writeln!(file, "id,int,ts_start").unwrap();

        let mut messages_txtime = Vec::with_capacity(n_message);

        println!("[trig] starting to send messages: {}", n_message);
        for i in 0..n_message {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();

            messages_txtime.push(MessageMetric {
                id: i,
                int: sleep_time as u128,
                ts_start: now,
            });

            tx.send(bytes.clone()).unwrap();

            thread::sleep(Duration::from_millis(sleep_time));

            if i % interval == 0 {
                sleep_time /= 2;
                if sleep_time < 1 {
                    sleep_time = 1;
                }
                println!(
                    "[trig] starting new round with sleep time: {}ms",
                    sleep_time
                );
            }
        }

        println!(
            "[trig] finished sending messages: {}",
            messages_txtime.len()
        );

        for msg in messages_txtime {
            writeln!(file, "{},{},{}", msg.id, msg.int, msg.ts_start).unwrap();
        }
    });

    let engine = Engine::headless();
    let mut store = Store::new(&engine);
    let mut module = unsafe { Module::deserialize_from_file(&store, "final.so")? };
    let mut import_object = imports! {};
    let mut instance = Instance::new(&mut store, &module, &import_object).unwrap();

    let mut file = File::create(format!("recv{}.csv", id)).unwrap();
    let mut messages_txtime = vec![0; n_message];
    writeln!(file, "id,ts_end").unwrap();

    let mut last_request_time_ns = 0;
    let mut average_exec_time = 0u128;
    let threashold_ns = 500000;

    println!("[invk] starting to receive messages: {}", n_message);
    for i in 0..n_message {
        let bytes = rx.recv().unwrap();

        let current_request_time_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let time_from_last_request = current_request_time_ns - last_request_time_ns;
        last_request_time_ns = current_request_time_ns;

        let unload = if w {
            time_from_last_request > average_exec_time + threashold_ns
        } else {
            true
        };

        if unload {
            drop(instance);
            drop(import_object);
            drop(module);

            module = unsafe { Module::deserialize_from_file(&store, "final.so")? };
            import_object = imports! {};
            instance = Instance::new(&mut store, &module, &import_object).unwrap();
        }

        let function = instance.exports.get::<Function>("encrypt")?;

        let len = bytes.len();
        let memory = instance.exports.get_memory("memory").unwrap();
        let memory_view = memory.view(&store);
        let memory_view_size = memory_view.size();

        let max_offset = memory_view_size.bytes().0 / 8;
        memory.grow(&mut store, 1).unwrap();

        let memory_view = memory.view(&store);

        let data_offset = max_offset;
        memory_view.write(data_offset as u64, &bytes).unwrap();

        let out_offset = data_offset + len;

        match function.call(
            &mut store,
            &[
                Value::I32(data_offset as i32),
                Value::I32(len as i32),
                Value::I32(out_offset as i32),
            ],
        ) {
            Ok(result) => {
                let result = result[0].unwrap_i32();
                if result < 0 {
                    println!("Error: {}", result);
                    return Ok(());
                }

                // let mut buf = vec![0u8; result as usize];
                // memory_view.read(out_offset as u64, &mut buf)?;

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();

                messages_txtime[i] = now;
                let exec_time = now - last_request_time_ns;

                average_exec_time = (average_exec_time * i as u128 + exec_time) / (i + 1) as u128;
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }

    println!(
        "[invk] finished receiving messages: {}",
        messages_txtime.len()
    );

    for (c, now) in messages_txtime.iter().enumerate() {
        writeln!(file, "{},{}", c, now).unwrap();
    }

    thread.join().unwrap();

    println!("[invk] finished");

    Ok(())
}
