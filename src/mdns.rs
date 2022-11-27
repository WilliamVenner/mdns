use crate::{runtime::AsyncUdpSocket, Error, Response};

use std::{
    io,
    net::{IpAddr, Ipv4Addr},
};

use async_stream::try_stream;
use futures_core::Stream;
use std::sync::Arc;

#[cfg(not(target_os = "windows"))]
use net2::unix::UnixUdpBuilderExt;
use std::net::SocketAddr;

/// The IP address for the mDNS multicast socket.
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MULTICAST_PORT: u16 = 5353;

pub fn mdns_interface(
    service_name: String,
    interface: Option<Ipv4Addr>,
) -> Result<(mDNSListener, mDNSSender), Error> {
    let socket = create_socket()?;

    socket.set_multicast_loop_v4(false)?;
    socket.set_nonblocking(true)?; // explicitly set nonblocking for wider compatability

    if let Some(interface) = interface {
        socket.join_multicast_v4(&MULTICAST_ADDR, &interface)?;
    } else {
        // Join multicast on all interfaces
        let mut did_join = false;
        if let Ok(ifaces) = if_addrs::get_if_addrs() {
            for iface in ifaces
                .into_iter()
                .filter(|iface| !iface.is_loopback())
                .filter_map(|iface| {
                    if let IpAddr::V4(iface) = iface.addr.ip() {
                        Some(iface)
                    } else {
                        None
                    }
                })
            {
                if socket.join_multicast_v4(&MULTICAST_ADDR, &iface).is_ok() {
                    did_join = true;
                }
            }
        }
        if !did_join {
            socket.join_multicast_v4(&MULTICAST_ADDR, &ADDR_ANY)?;
        }
    }

    let socket = crate::runtime::make_async_socket(socket)?;

    let recv_buffer = vec![0; 4096];

    Ok((
        mDNSListener {
            recv: socket.clone(),
            recv_buffer,
        },
        mDNSSender {
            service_name,
            send: socket,
        },
    ))
}

const ADDR_ANY: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

#[cfg(not(target_os = "windows"))]
fn create_socket() -> io::Result<std::net::UdpSocket> {
    net2::UdpBuilder::new_v4()?
        .reuse_address(true)?
        .reuse_port(true)?
        .bind((ADDR_ANY, MULTICAST_PORT))
}

#[cfg(target_os = "windows")]
fn create_socket() -> io::Result<std::net::UdpSocket> {
    net2::UdpBuilder::new_v4()?
        .reuse_address(true)?
        .bind((ADDR_ANY, MULTICAST_PORT))
}

/// An mDNS sender on a specific interface.
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub struct mDNSSender {
    service_name: String,
    send: Arc<AsyncUdpSocket>,
}

impl mDNSSender {
    /// Send multicasted DNS queries.
    pub async fn send_request(&mut self) -> Result<(), Error> {
        let mut builder = dns_parser::Builder::new_query(0, false);
        let prefer_unicast = false;
        builder.add_question(
            &self.service_name,
            prefer_unicast,
            dns_parser::QueryType::PTR,
            dns_parser::QueryClass::IN,
        );
        let packet_data = builder.build().unwrap();

        let addr = SocketAddr::new(MULTICAST_ADDR.into(), MULTICAST_PORT);

        self.send.send_to(&packet_data, addr).await?;
        Ok(())
    }
}

/// An mDNS listener on a specific interface.
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub struct mDNSListener {
    pub(crate) recv: Arc<AsyncUdpSocket>,
    pub(crate) recv_buffer: Vec<u8>,
}

impl mDNSListener {
    pub fn listen(mut self) -> impl Stream<Item = Result<Response, Error>> {
        try_stream! {
            loop {
                let (count, addr) = self.recv.recv_from(&mut self.recv_buffer).await?;

                if count > 0 {
                    if let Ok(raw_packet) = dns_parser::Packet::parse(&self.recv_buffer[..count]) {
                        yield Response::from_packet(&raw_packet, addr);
                    }
                }
            }
        }
    }
}
