use crate::Error;
use futures_core::Future;
use std::{sync::Arc, time::Duration};

pub use tokio::net::UdpSocket as AsyncUdpSocket;
pub use tokio::spawn;

pub fn make_async_socket(socket: std::net::UdpSocket) -> Result<Arc<AsyncUdpSocket>, Error> {
    Ok(Arc::new(AsyncUdpSocket::from_std(socket)?))
}

pub async fn timeout<F, T>(timeout: Duration, future: F) -> Result<T, crate::errors::TimeoutError>
where
    F: Future<Output = T>,
{
    tokio::time::timeout(timeout, future).await
}
