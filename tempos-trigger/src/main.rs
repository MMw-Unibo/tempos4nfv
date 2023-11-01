use clap::Parser;
use log;
use std::{
    io::Write,
    net::{SocketAddr, UdpSocket},
    os::fd::IntoRawFd,
};

use tempos::msg_type;

/// Simple TEMPOS Trigger example
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Topic to send the message
    #[clap(short, long)]
    topic: String,

    /// address to send the message to
    #[clap(short, long)]
    addr: String,

    /// address to send the message to
    #[clap(short, long)]
    saddr: String,

    #[clap(short, long)]
    millis: u64,

    #[clap(short = 'M', long, default_value = "0")]
    messages: u64,
}

struct NetworkInterface {
    pub name: String,
    pub description: String,
    pub index: u32,
    // pub mac: Option<MacAddr>,
    // pub ips: Vec<IpAddr>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    let saddr = args.saddr.parse::<SocketAddr>()?;

    let sock = UdpSocket::bind(&args.addr)?;

    let data = std::fs::read("Cargo.toml").unwrap();
    let data_len = data.len() as u32;

    let topic = args.topic.clone();
    let topic_len = args.topic.len() as u32;

    let mut buf = Vec::with_capacity(2048);

    let mut start = std::time::Instant::now();

    let mut interval_ms = 50;
    let mut count: i32 = 0;

    println!("id,interval,ts_send");
    loop {
        buf.clear();
        buf.write(&msg_type::INVOK.to_be_bytes())?;
        buf.write(&count.to_be_bytes())?;
        buf.write(&topic_len.to_be_bytes())?;
        buf.write(&topic.as_bytes())?;
        buf.write(&data_len.to_be_bytes())?;
        buf.write(&data).unwrap();

        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        sock.send_to(&buf, saddr)?;

        if args.messages == 0 {
            println!("{},{},{}", count, interval_ms, now_ns);
            if start.elapsed().as_millis() >= args.millis as u128 {
                if interval_ms <= 9 {
                    log::debug!("interval_ms <= 1, exiting");
                    break;
                }

                interval_ms -= 1;
                log::debug!("new interval_ms: {}", interval_ms);
                start = std::time::Instant::now();
            }
            std::thread::sleep(std::time::Duration::from_millis(interval_ms));
        } else {
            println!("{},{},{}", count, args.millis, now_ns);
            if count >= args.messages as i32 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(args.millis));
        }
        count += 1;
    }

    // let iface_name = "lo";
    // let ifaces = nix::ifaddrs::getifaddrs().unwrap();

    // let network_interfaces: Vec<NetworkInterface> = Vec::new();

    // for iface in ifaces {
    //     if iface.interface_name != iface_name {
    //         continue;
    //     }

    //     println!("Interface: {}", iface.interface_name);
    //     println!("  Flags: {:?}", iface.flags);
    //     if let Some(addr) = iface.address {
    //         if let Some(saddr) = addr.as_sockaddr_in() {
    //             let ip = std::net::Ipv4Addr::from(saddr.ip());
    //             println!("  IP: {}", ip)
    //         }
    //         if let Some(saddr) = addr.as_sockaddr_in6() {
    //             let ip = std::net::Ipv6Addr::from(saddr.ip());
    //             println!("  IP: {}", ip)
    //         }
    //     }
    //     if let Some(mask) = iface.netmask {
    //         if let Some(saddr) = mask.as_sockaddr_in() {
    //             let ip = std::net::Ipv4Addr::from(saddr.ip());
    //             println!("  IP: {}", ip)
    //         }
    //         if let Some(saddr) = mask.as_sockaddr_in6() {
    //             let ip = std::net::Ipv6Addr::from(saddr.ip());
    //             println!("  IP: {}", ip)
    //         }
    //     }
    //     println!("  Broadcast: {:?}", iface.broadcast);
    //     println!("  Destination: {:?}", iface.destination);
    // }

    // let index = nix::net::if_::if_nametoindex(iface_name).unwrap();
    // println!("Index: {}", index);

    log::info!("Done!");

    Ok(())
}
