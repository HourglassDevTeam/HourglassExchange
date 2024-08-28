#[allow(dead_code)]
pub struct ClickHouseQueryBuilder
{
    select_clause: String,
    from_clause: String,
    where_clause: Option<String>,
    order_by_clause: Option<String>,
    order_direction: Option<String>, // 存储排序方向（ASC 或 DESC）
    limit_clause: Option<String>,
    offset_clause: Option<String>, // Add this line
}

impl Default for ClickHouseQueryBuilder
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[allow(dead_code)]
impl ClickHouseQueryBuilder
{
    // 初始化构造器
    pub fn new() -> Self
    {
        Self { select_clause: String::new(),
               from_clause: String::new(),
               where_clause: None,
               order_by_clause: None,
               order_direction: None,
               limit_clause: None,
               offset_clause: None }
    }

    // 设置SELECT子句
    pub fn select(mut self, fields: &str) -> Self
    {
        self.select_clause = format!("SELECT {}", fields);
        self
    }

    // 设置FROM子句
    pub fn from(mut self, database_name: &str, table_name: &str) -> Self
    {
        self.from_clause = format!("FROM {}.{}", database_name, table_name);
        self
    }

    // 添加WHERE条件
    pub fn where_clause(mut self, condition: &str) -> Self
    {
        self.where_clause = Some(format!("WHERE {}", condition));
        self
    }

    // 添加LIKE条件
    pub fn like_clause(mut self, field: &str, pattern: &str) -> Self
    {
        self.where_clause = Some(self.where_clause.map_or_else(|| format!("WHERE {} LIKE '{}'", field, pattern),
                                                               |existing_clause| format!("{} AND {} LIKE '{}'", existing_clause, field, pattern)));
        self
    }

    // 添加NOT LIKE条件
    pub fn not_like_clause(mut self, field: &str, pattern: &str) -> Self
    {
        self.where_clause = Some(self.where_clause.map_or_else(|| format!("WHERE {} NOT LIKE '{}'", field, pattern),
                                                               |existing_clause| format!("{} AND {} NOT LIKE '{}'", existing_clause, field, pattern)));
        self
    }

    // 添加ORDER BY子句
    // Add ORDER BY clause
    pub fn order(mut self, field: &str, direction: Option<&str>) -> Self
    {
        self.order_by_clause = direction.map(|d| format!("ORDER BY {} {}", field, d));
        self
    }

    // 添加LIMIT子句
    pub fn limit(mut self, limit: usize) -> Self
    {
        self.limit_clause = Some(format!("LIMIT {}", limit));
        self
    }

    pub fn offset(mut self, offset: usize) -> Self
    {
        self.offset_clause = Some(format!("OFFSET {}", offset));
        self
    }

    // 构建最终的查询
    pub fn build(self) -> String
    {
        let mut query = format!("{} {}", self.select_clause, self.from_clause);

        if let Some(where_clause) = self.where_clause {
            query.push_str(&format!(" {}", where_clause));
        }

        if let Some(order_by_clause) = self.order_by_clause {
            query.push_str(&format!(" {}", order_by_clause));
        }

        if let Some(limit_clause) = self.limit_clause {
            query.push_str(&format!(" {}", limit_clause));
        }

        query
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_where_query()
    {
        let query = ClickHouseQueryBuilder::new().select("*").from("default_db", "users").where_clause("identification = 1").build();
        assert_eq!(query, "SELECT * FROM default_db.users WHERE identification = 1");
    }

    #[test]
    fn test_like_query()
    {
        let query = ClickHouseQueryBuilder::new().select("*")
                                                 .from("default_db", "users")
                                                 .like_clause("name", "%example%")
                                                 .build();
        assert_eq!(query, "SELECT * FROM default_db.users WHERE name LIKE '%example%'");
    }

    #[test]
    fn test_not_like_query()
    {
        let query = ClickHouseQueryBuilder::new().select("*")
                                                 .from("default_db", "products")
                                                 .not_like_clause("description", "%old%")
                                                 .build();
        assert_eq!(query, "SELECT * FROM default_db.products WHERE description NOT LIKE '%old%'");
    }

    #[test]
    fn test_combined_query()
    {
        let query = ClickHouseQueryBuilder::new().select("identification, name")
                                                 .from("default_db", "users") // 添加数据库名参数
                                                 .where_clause("age > 18")
                                                 .like_clause("email", "%@mail.com")
                                                 .order("created_at", Some("DESC")) // 修改order_by的调用
                                                 .limit(10)
                                                 .build();
        assert_eq!(query,
                   "SELECT identification, name FROM default_db.users WHERE age > 18 AND email LIKE '%@mail.com' ORDER BY created_at DESC LIMIT 10");
    }

    #[test]
    fn test_query_with_multiple_conditions()
    {
        let query = ClickHouseQueryBuilder::new().select("*")
                                                 .from("default_db", "users") // 添加数据库名参数
                                                 .where_clause("identification = 1")
                                                 .like_clause("username", "%user%")
                                                 .not_like_clause("password", "%weak%")
                                                 .build();
        assert_eq!(query,
                   "SELECT * FROM default_db.users WHERE identification = 1 AND username LIKE '%user%' AND password NOT LIKE '%weak%'");
    }
}
