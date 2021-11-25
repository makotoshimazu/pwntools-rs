use pwntools::{connection::Connection, connection::Remote};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut remo = Remote::new("127.0.0.1:12345").await?;
    remo.interactive().await?;
    Ok(())
}
