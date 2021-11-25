use pwntools::{connection::Connection, connection::Remote};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let conn = Remote::new("127.0.0.1:12345").await?;
    conn.interactive().await?;
    Ok(())
}
