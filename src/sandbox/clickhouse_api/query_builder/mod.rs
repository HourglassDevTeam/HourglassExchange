struct ClickHouseQueryBuilder {
    select_clause: String,
    from_clause: String,
    where_clause: Option<String>,
    order_by_clause: Option<String>,
    limit_clause: Option<String>,
}

impl ClickHouseQueryBuilder {
    // 初始化构造器
    pub fn new() -> Self {
        Self {
            select_clause: String::new(),
            from_clause: String::new(),
            where_clause: None,
            order_by_clause: None,
            limit_clause: None,
        }
    }

    // 设置SELECT子句
    pub fn select(mut self, fields: &str) -> Self {
        self.select_clause = format!("SELECT {}", fields);
        self
    }

    // 设置FROM子句
    pub fn from(mut self, table: &str) -> Self {
        self.from_clause = format!("FROM {}", table);
        self
    }

    // 添加WHERE条件
    pub fn where_clause(mut self, condition: &str) -> Self {
        self.where_clause = Some(format!("WHERE {}", condition));
        self
    }

    // 添加ORDER BY子句
    pub fn order_by(mut self, fields: &str) -> Self {
        self.order_by_clause = Some(format!("ORDER BY {}", fields));
        self
    }

    // 添加LIMIT子句
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit_clause = Some(format!("LIMIT {}", limit));
        self
    }

    // 构建最终的查询
    pub fn build(self) -> String {
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
