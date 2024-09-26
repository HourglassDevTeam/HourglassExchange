use std::sync::{Arc, Mutex};
use tokio::{spawn, sync::oneshot};
use warp::{
    hyper::{body::to_bytes, Body, Client, Request},
    Filter, Rejection, Reply,
};

// 创建一个 warp 服务器的路由
/// 定义一个 Warp 路由，该路由使用了 `warp::path("hello")` 过滤器，
/// 并且包含一个自动关闭服务器的机制。当 `/hello` 路由被访问时，
/// 服务器会响应一段文本，并通过 `shutdown_tx` 发送关闭信号，
/// 从而优雅关闭服务器。
fn routes(shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        // `warp::path("hello")` 是一个路径过滤器，它将只匹配 URL 路径 `/hello` 的 GET 请求。
        // Warp 路由器允许你通过不同的路径和 HTTP 方法（如 GET、POST）构建不同的处理逻辑。
        // 这里，它只匹配 GET 请求，并且只有在路径是 `/hello` 时，这个路由才会被命中。
        .and(warp::path("hello"))
        .map(move || {
            // 当请求处理时，发送关闭信号
            let mut tx_lock = shutdown_tx.lock().unwrap();
            // `tx_lock.take()` 是对 `Option` 类型的调用，`take()` 方法会取出其中的值（如果存在），
            if let Some(tx) = tx_lock.take() {
                let _ = tx.send(());
                println!("Shutdown signal sent");
            }
            warp::reply::html("『如是我闻。一时，佛在舍卫国祇树给孤独园，与大比丘众千二百五十人俱。尔时，世尊食时，著衣持钵，入舍卫大城乞食。于其城中，次第乞已，还至本处。饭食讫，收衣钵，洗足已，敷座而坐。")
        })
}

#[tokio::main]
async fn main() {
    // 创建用于关闭服务器的信号通道，并使用 Arc<Mutex<Option<>>>
    // 这里使用了 `oneshot::channel()` 创建一个单次通信通道，用于发送和接收关闭信号。
    // `shutdown_tx` 是发送者，`shutdown_rx` 是接收者。
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let shutdown_tx = Arc::new(Mutex::new(Some(shutdown_tx)));

    // 启动 warp 服务器
    // `bind_with_graceful_shutdown` 方法会启动 Warp 服务器，同时在接收到关闭信号时优雅关闭服务器。
    // 它的参数包括一个 IP 地址和端口（`127.0.0.1:3030`），以及一个 Future（`shutdown_rx.await.ok()`），
    // 当该 Future 完成时，服务器将优雅关闭。
    let addr: std::net::SocketAddr = "127.0.0.1:3030".parse().unwrap();
    let (_, server_future) = warp::serve(routes(shutdown_tx.clone()))
        .bind_with_graceful_shutdown(addr, async {
            // 等待关闭信号并关闭服务器
            shutdown_rx.await.ok();
            println!("Shutting down server gracefully...");
        });

    // `server_future` 是 Future 部分，启动服务器
    let server_handle = spawn(server_future);

    // 创建一个 hyper 客户端
    let client = Client::new();

    // 构建一个请求
    let req = Request::builder()
        .uri("http://localhost:3030/hello")
        .body(Body::empty())
        .unwrap();

    // 发送请求并等待结果
    let client_handle = spawn(async move {
        let res = client.request(req).await.unwrap();
        println!("Received response with status: {}", res.status());

        // 获取响应体
        // `to_bytes(res.into_body()).await.unwrap()` 用于将 `hyper::Body` 转换为 `bytes::Bytes`。
        // `hyper::Body` 是流式传输的响应体，`to_bytes` 会收集流中的所有字节并返回一个字节数组。
        // In Warp (and more specifically in the hyper library that Warp uses under the hood),
        // the into_body method is used to extract the body of an HTTP response or request as a Body type.
        let body = to_bytes(res.into_body()).await.unwrap();
        let body_str = String::from_utf8_lossy(&body);
        println!("Response body: {}", body_str);

        // 手动发送关闭信号，模拟服务器处理完后自动关闭
        if let Some(tx) = shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }

    });

    // 等待服务器和客户端任务完成
    let (_server_result, _client_result) = tokio::join!(
        server_handle,
        client_handle
    );
}
