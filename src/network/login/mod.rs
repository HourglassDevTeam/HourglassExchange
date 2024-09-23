/// 添加一个登陆验证模块
///
use tokio::sync::mpsc::Sender;
use crate::error::ExchangeError;
use crate::hourglass::hourglass_client_local_mode::HourglassClient;

// 定义登录请求结构体
#[derive(Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,  // 或者 token
    pub response_tx: Sender<Result<LoginResponse, ExchangeError>>,
}

// 登录响应结构体
#[derive(Debug)]
pub struct LoginResponse {
    pub token: String, // 可以返回一个令牌
}
