use std::collections::HashMap;
use async_trait::async_trait;
use crate::{error::ExchangeError, hourglass::hourglass_client_local_mode::HourglassClient};
/// 添加一个登陆验证模块
use tokio::sync::oneshot;
use crate::hourglass::hourglass_client_local_mode::HourglassClientEvent;

// 定义登录请求结构体
#[derive(Debug)]
pub struct LoginRequest
{
    pub username: String,
    pub password: String, // 或者 token
    pub response_tx: oneshot::Sender<Result<LoginResponse, ExchangeError>>,
}

// 登录响应结构体
#[derive(Debug)]
pub struct LoginResponse
{
    pub session_token: String, // 可以返回一个令牌
}

#[async_trait]
pub trait Authenticator {
    // 验证客户端是否已登录
    fn is_authenticated(&self, authenticated_clients: &HashMap<String, String>) -> bool;
    // 认证方法，用于验证用户名和密码
    fn authenticate_client(&self, username: &str, password: &str) -> Result<String, ExchangeError>;
    async fn login(&self, username: String, password: String) -> Result<String, ExchangeError>;
}

#[async_trait]
impl Authenticator for HourglassClient {
    // 验证客户端是否已登录
    fn is_authenticated(&self, authenticated_clients: &HashMap<String, String>) -> bool {
        // 检查是否有有效的 token
        // 可以根据情况检查客户端的令牌
        !authenticated_clients.is_empty()
    }

    // 认证方法，用于验证用户名和密码
    fn authenticate_client(&self, username: &str, password: &str) -> Result<String, ExchangeError> {
        // 假设我们有一个简单的用户名密码验证逻辑
        if username == "user" && password == "pass" {
            // 生成一个 token，表示成功登录
            Ok("valid_token".to_string())
        } else {
            Err(ExchangeError::AuthenticationFailed)
        }
    }

    async fn login(&self, username: String, password: String) -> Result<String, ExchangeError> {
        let (response_tx, response_rx) = oneshot::channel();
        let login_request = LoginRequest {
            username,
            password,
            response_tx
        };

        self.client_event_tx.send(HourglassClientEvent::Login(login_request)).expect("Failed to send Login request");

        // 等待服务器返回的令牌或错误信息
        let login_response = response_rx.await.expect("Failed to receive Login response")?;

        Ok(login_response.session_token)
    }
}