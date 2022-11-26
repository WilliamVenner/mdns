use crate::Error;

use futures_core::{Future, Stream};
use tokio_stream::StreamExt;
use std::{sync::Arc, time::Duration};

pub use tokio::net::UdpSocket as AsyncUdpSocket;
pub use tokio::spawn;
pub fn create_interval_stream(request_interval: Duration) -> impl Stream<Item = ()> {
    tokio_stream::once(()).chain(tokio_stream::wrappers::IntervalStream::new(tokio::time::interval_at(
        tokio::time::Instant::now() + request_interval,
        request_interval,
    )).map(|_| ()))
}

pub fn make_async_socket(socket: std::net::UdpSocket) -> Result<Arc<AsyncUdpSocket>, Error> {
    Ok(Arc::new(AsyncUdpSocket::from_std(socket)?))
}

pub async fn timeout<F, T>(timeout: Duration, future: F) -> Result<T, crate::errors::TimeoutError>
where
    F: Future<Output = T>,
{
    tokio::time::timeout(timeout, future).await
}
