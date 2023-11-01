use nix::{
    libc::msghdr,
    sys::socket::{sendmsg, setsockopt, sockopt, MsgFlags, SockAddr},
};
use socket2::{Domain, Socket, Type};
use std::{os::unix::prelude::*, str::FromStr};

pub mod buffer;
pub mod message;
pub mod node;

pub mod msg_type {
    pub const REGISTRATION: u8 = 0x00;
    pub const INVOK: u8 = 0x01;
    pub const MONITORING: u8 = 0x02;
    pub const UNREGISTRATION: u8 = 0x03;
}

#[inline(always)]
pub fn message_id(header: u8) -> u8 {
    header
}

pub type txtime_flags = ::std::os::raw::c_uint;
pub const SOF_TXTIME_DEADLINE_MODE: txtime_flags = 1;
pub const SOF_TXTIME_REPORT_ERRORS: txtime_flags = 2;
pub const SOF_TXTIME_FLAGS_LAST: txtime_flags = 2;
pub const SOF_TXTIME_FLAGS_MASK: txtime_flags = 3;

pub fn open_socket(prio: i32, iface: &str) -> Socket {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None).unwrap();

    let sockfd = socket.as_raw_fd();
    setsockopt(sockfd, sockopt::Priority, &prio).unwrap();

    socket.bind_device(Some(iface.as_bytes())).unwrap();

    let mut sk_txtime = nix::libc::sock_txtime {
        clockid: nix::libc::SOF_TIMESTAMPING_TX_HARDWARE as i32,
        flags: 0,
    };

    sk_txtime.flags = SOF_TXTIME_DEADLINE_MODE | SOF_TXTIME_REPORT_ERRORS;

    setsockopt(sockfd, sockopt::TxTime, &sk_txtime).unwrap();

    socket
}

pub fn send_message(socket: &Socket, message: &[u8], addr: std::net::SocketAddr, txtime: u64) {
    let cmsg = nix::sys::socket::ControlMessage::TxTime(&txtime);

    let sockfd = socket.as_raw_fd();
    let iov = std::io::IoSlice::new(message);
    let localhost = nix::sys::socket::SockaddrIn::from_str(&addr.to_string()).unwrap();
    sendmsg(
        sockfd,
        &[iov],
        &[cmsg],
        nix::sys::socket::MsgFlags::empty(),
        Some(&localhost),
    )
    .unwrap();

    let now = nix::time::clock_gettime(nix::time::ClockId::CLOCK_REALTIME).unwrap();
}

#[inline]
pub fn normalize_timestamp_ns(ts: u64, base: u64) -> u64 {
    let tmp = ts / base;
    tmp * base
}

pub fn calculate_txtime(now: u64, period: u64, txtime: u64) -> u64 {
    /*
     * ^
     * |    threshold
     * |   <--------->    |
     * |                  |
     * |   ^              |
     * |   |         ^    |
     * |   |         |    |
     * +---+---------+----+------------------>
     *   req_offset  |    |
     *               |  rt_slot_offset
     * <---txtime--->|
     */

    let now_normalized = normalize_timestamp_ns(now, period);
    let request_offset = now - now_normalized;

    let mut send_time = txtime;

    let rt_start_offset = period - txtime;
    let rt_end_offset = period - request_offset;

    let dynamic_time = true;
    let threshold_ns = 10;
    let has_rt_slot = true;
    if has_rt_slot {
        if dynamic_time {
            send_time = now + threshold_ns;
        } else {
            if request_offset < rt_end_offset && request_offset + threshold_ns > txtime {
                send_time = now_normalized + txtime;
            } else {
                // NOTE(garbu): We are in the BE slot, so we add the period
                //      to the current time in order to send the message
                //      in the next available window and RT slot.
                send_time = now_normalized + period + txtime;
            }
        }
    } else {
        // There is no RT slot.
        // TODO(garbu): Should we send the message now or in the next period?
        send_time = now;
    }

    send_time
}
