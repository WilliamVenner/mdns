//! Utilities for discovering devices on the LAN.
//!
//! Examples
//!
//! ```rust,no_run
//! use futures_util::{pin_mut, stream::StreamExt};
//! use mdns::{Error, Record, RecordKind};
//! use std::time::Duration;
//!
//! const SERVICE_NAME: &'static str = "_googlecast._tcp.local";
//!
//! #[cfg_attr(feature = "runtime-async-std", async_std::main)]
//! #[cfg_attr(feature = "runtime-tokio", tokio::main)]
//! async fn main() -> Result<(), Error> {
//!     let stream = mdns::discover::all(SERVICE_NAME, Duration::from_secs(15))?.listen();
//!     pin_mut!(stream);
//!
//!     while let Some(Ok(response)) = stream.next().await {
//!         println!("{:?}", response);
//!     }
//!
//!     Ok(())
//! }
//! ```

use crate::mdns::{mDNSSender, mdns_interface};
use crate::{mDNSListener, Error, Response};
use futures_core::Stream;
use futures_util::{future::ready, StreamExt};
use std::net::Ipv4Addr;

/// A multicast DNS discovery request.
///
/// This represents a single lookup of a single service name.
///
/// This object can be iterated over to yield the received mDNS responses.
pub struct Discovery {
    service_name: String,

    mdns_sender: mDNSSender,
    mdns_listener: mDNSListener,

    /// Whether we should ignore empty responses.
    ignore_empty: bool,
}

/// Gets an iterator over all responses for a given service on all interfaces.
pub fn all<S>(service_name: S) -> Result<Discovery, Error>
where
    S: AsRef<str>,
{
    let service_name = service_name.as_ref().to_string();
    let (mdns_listener, mdns_sender) = mdns_interface(service_name.clone(), None)?;

    Ok(Discovery {
        service_name,
        mdns_sender,
        mdns_listener,
        ignore_empty: true,
    })
}

/// Gets an iterator over all responses for a given service on a given interface.
pub fn interface<S>(service_name: S, interface_addr: Ipv4Addr) -> Result<Discovery, Error>
where
    S: AsRef<str>,
{
    let service_name = service_name.as_ref().to_string();
    let (mdns_listener, mdns_sender) = mdns_interface(service_name.clone(), Some(interface_addr))?;

    Ok(Discovery {
        service_name,
        mdns_sender,
        mdns_listener,
        ignore_empty: true,
    })
}

impl Discovery {
    /// Sets whether or not we should ignore empty responses.
    ///
    /// Defaults to `true`.
    pub fn ignore_empty(mut self, ignore: bool) -> Self {
        self.ignore_empty = ignore;
        self
    }

    pub fn listen(
        self,
    ) -> (
        DiscoveryScanner,
        impl Stream<Item = Result<Response, Error>>,
    ) {
        let Discovery {
            service_name,
            mdns_sender,
            mdns_listener,
            ignore_empty,
        } = self;

        (
            DiscoveryScanner(mdns_sender),
            mdns_listener.listen().filter(move |res| {
                ready(match res {
                    Ok(response) => {
                        (!response.is_empty() || !ignore_empty)
                            && response
                                .answers
                                .iter()
                                .any(|record| record.name == service_name)
                    }
                    Err(_) => true,
                })
            }),
        )
    }
}

#[derive(Clone)]
pub struct DiscoveryScanner(mDNSSender);
impl DiscoveryScanner {
    pub async fn scan(&mut self) -> Result<(), Error> {
        self.0.send_request().await
    }
}
