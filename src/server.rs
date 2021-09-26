use std::time::Duration;
use anyhow::Result;
use tinyroute::server::{Server, TcpListener};
use tinyroute::{Agent, RouterTx};

use super::Address;

pub async fn run(
    agent: Agent<(), Address>,
    socket_addr: &str,
    router_tx: RouterTx<Address>,
) -> Result<()> {
    let listener = TcpListener::bind(socket_addr).await?;
    let server = Server::new(listener, agent);
    let mut connection_id = 0usize;
    server.run(router_tx, Some(Duration::from_secs(5 * 60)), || {
        connection_id += 1;
        Address::Connection(connection_id)
    })
    .await?;
    Ok(())
}
