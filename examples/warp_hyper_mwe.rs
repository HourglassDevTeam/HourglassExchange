use std::net::SocketAddr;
use tokio::{signal, spawn};
use warp::{
    hyper::{body::to_bytes, Body, Client, Request},
    Filter, Rejection, Reply,
};
// 引入signal

// 创建一个 warp 服务器的路由
fn routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    warp::get()
        .and(warp::path("hello"))
        .map(|| warp::reply::html("『如是我闻。一时，佛在舍卫国祇树给孤独园，与大比丘众千二百五十人俱。尔时，世尊食时，著衣持钵，入舍卫大城乞食。于其城中，次第乞已，还至本处。饭食讫，收衣钵，洗足已，敷座而坐。"))
}

#[tokio::main]
async fn main()
{
    // 启动 warp 服务器
    let addr: SocketAddr = "127.0.0.1:3030".parse().unwrap();
    let server = warp::serve(routes());
    let server_handle = spawn(async move { server.run(addr).await });

    // 创建一个 hyper 客户端
    let client = Client::new();

    // 构建一个请求
    let req = Request::builder().uri("http://localhost:3030/hello").body(Body::empty()).unwrap();

    // 发送请求
    let client_handle = spawn(async move {
        let res = client.request(req).await.unwrap();
        println!("Received response with status: {}", res.status());

        // 获取响应体
        let body = to_bytes(res.into_body()).await.unwrap();
        let body_str = String::from_utf8_lossy(&body);
        println!("Response body: {}", body_str);
    });

    // 等待服务器和客户端任务完成
    let (_server_result, _client_result, signal_result) = tokio::join!(server_handle, client_handle, signal::ctrl_c());

    println!("Server signal status: {:?}", signal_result);
    // 处理可能的错误
    if let Err(e) = signal_result {
        eprintln!("Error from signal::ctrl_c: {}", e);
    }
}
