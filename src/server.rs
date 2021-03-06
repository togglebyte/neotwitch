use std::time::Duration;
use anyhow::Result;
use tinyroute::server::{Server, TcpConnections};
use tinyroute::Agent;

use super::Address;

pub async fn run(
    agent: Agent<(), Address>,
    socket_addr: &str,
) -> Result<()> {
    let listener = TcpConnections::bind(socket_addr).await?;
    let server = Server::new(listener, agent);
    let mut connection_id = 0usize;
    server.run(Some(Duration::from_secs(5 * 60)), None, || {
        connection_id += 1;
        Address::Connection(connection_id)
    })
    .await?;
    Ok(())
}
