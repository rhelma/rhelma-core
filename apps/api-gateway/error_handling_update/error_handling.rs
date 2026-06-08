rust
use anyhow::{Context, Result};

async fn bind_to_address(bind: &str) -> Result<TcpListener> {
    let addr: SocketAddr = bind.parse()
        .map_err(|e| anyhow!("invalid RHELMA_SECURITY_GOV__BIND '{}': {}", bind, e))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow!("failed to bind {}: {}", addr, e))?;
    Ok(listener)
}
