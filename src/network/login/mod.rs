use crate::{error::ExchangeError, hourglass::HourglassExchange};
use bcrypt::{hash, verify, DEFAULT_COST};
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
// 定义查询结果的数据结构
#[derive(Debug, clickhouse::Row, serde::Deserialize)]
struct UserInfo
{
    pub(crate) password_hash: String,
}

/// 注销请求结构体
#[derive(Debug)]
pub struct LogoutRequest
{
    pub session_token: String,
    pub response_tx: oneshot::Sender<Result<(), ExchangeError>>,
}

impl Authentication for HourglassExchange
{
    #[allow(unused)]
    async fn handle_register(&self, username: String, email: String, password: String) -> Result<(), ExchangeError>
    {
        // 加密密码
        let password_hash = hash(password, DEFAULT_COST).map_err(|_| ExchangeError::PasswordHashError)?;

        // 创建插入用户信息的 SQL
        let insert_query = format!(
                                   "INSERT INTO accounts.user_info (id, username, email, password_hash, created_at) \
            VALUES ('{}', '{}', '{}', '{}', '{}')",
                                   Uuid::new_v4(),
                                   username,
                                   email,
                                   password_hash,
                                   Utc::now()
        );

        // 执行插入操作
        self.clickhouse_client.client.read().await.query(&insert_query).execute().await.map_err(|_| ExchangeError::DatabaseError)?;

        Ok(())
    }

    #[allow(unused)]
    async fn handle_login(&self, username: String, password: String) -> Result<String, ExchangeError>
    {
        // 查询用户的加密密码
        let select_query = format!("SELECT password_hash FROM accounts.user_info WHERE username = '{}'", username);

        // 执行查询并解析结果
        let result = self.clickhouse_client
                         .client
                         .read()
                         .await
                         .query(&select_query)
                         .fetch_one::<UserInfo>()
                         .await
                         .map_err(|_| ExchangeError::InvalidCredentials)?;

        let password_hash = result.password_hash;

        // 验证密码
        if verify(password, &password_hash).map_err(|_| ExchangeError::InvalidCredentials)? {
            let session_token = Uuid::new_v4().to_string();
            // 保存会话信息
            self.active_sessions.lock().await.insert(session_token.clone(), username.parse().unwrap());
            Ok(session_token)
        }
        else {
            Err(ExchangeError::InvalidCredentials)
        }
    }

    #[allow(unused)]
    /// 注销
    async fn handle_logout(&self, session_token: String) -> Result<(), ExchangeError>
    {
        let mut sessions = self.active_sessions.lock().await;
        if sessions.remove(&session_token).is_some() {
            Ok(())
        }
        else {
            Err(ExchangeError::InvalidSession)
        }
    }
}
#[allow(unused)]
trait Authentication
{
    async fn handle_register(&self, username: String, email: String, password: String) -> Result<(), ExchangeError>;
    async fn handle_login(&self, username: String, password: String) -> Result<String, ExchangeError>;
    // 注销
    async fn handle_logout(&self, session_token: String) -> Result<(), ExchangeError>;
    // 删除账户
    // async fn delete_account(&self) -> Result<(), ExchangeError>;
}
