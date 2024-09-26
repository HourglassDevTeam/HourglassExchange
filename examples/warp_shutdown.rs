use tokio::sync::oneshot;
use warp::Filter;

#[tokio::main]
async fn main() {
    let routes = warp::any().map(|| "Hello, World!");

    let (tx, rx) = oneshot::channel();

    let (_addr, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([127, 0, 0, 1], 3030), async {
            rx.await.ok();
        });

    // Spawn the server into a runtime
    tokio::task::spawn(server);

    // Later, start the shutdown...
    let _ = tx.send(());
}