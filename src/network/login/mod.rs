use crate::{
    error::ExchangeError,
    hourglass::{clickhouse_api::queries_operations::ClickHouseClient, hourglass_client_local_mode::HourglassClient},
};
use async_trait::async_trait;
use bcrypt::{hash, DEFAULT_COST};
use chrono::Utc;
use tokio::sync::oneshot;
use uuid::Uuid;

/// 定义用户注册请求
#[derive(Debug)]
pub struct RegisterRequest
{
    pub username: String,
    pub email: String,
    pub password: String, // 这是未加密的密码
    pub response_tx: oneshot::Sender<Result<(), ExchangeError>>,
}

/// 定义登录请求
#[derive(Debug)]
pub struct LoginRequest
{
    pub username: String,
    pub password: String, // 未加密的密码
    pub response_tx: oneshot::Sender<Result<LoginResponse, ExchangeError>>,
}

/// 登录响应结构体
#[derive(Debug)]
pub struct LoginResponse
{
    pub session_token: String, // 成功登录后返回的 session 令牌
}

/// 注销请求结构体
#[derive(Debug)]
pub struct LogoutRequest
{
    pub session_token: String,
    pub response_tx: oneshot::Sender<Result<(), ExchangeError>>,
}

#[async_trait]
pub trait Authenticator
{
    async fn register(&self, client: ClickHouseClient, username: String, email: String, password: String) -> Result<(), ExchangeError>;
    // async fn login(&self, client: ClickHouseClient,username: String, password: String) -> Result<String, ExchangeError>;
    // async fn logout(&self, session_token: String) -> Result<(), ExchangeError>;
}
