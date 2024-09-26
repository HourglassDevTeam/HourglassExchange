use std::sync::{Arc, Mutex};
use tokio::{spawn, sync::oneshot};
use warp::{
    hyper::{body::to_bytes, Body, Client, Request},
    Filter, Rejection, Reply,
};

// 创建一个 warp 服务器的路由
fn routes(shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path("hello"))
        .map(move || {
            // 当请求处理时，发送关闭信号
            let mut tx_lock = shutdown_tx.lock().unwrap();
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
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let shutdown_tx = Arc::new(Mutex::new(Some(shutdown_tx)));

    // 启动 warp 服务器
    let addr: std::net::SocketAddr = "127.0.0.1:3030".parse().unwrap();
    let (_, server_future) = warp::serve(routes(shutdown_tx.clone()))
        .bind_with_graceful_shutdown(addr, async {
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
