use serde::Deserialize;

#[derive(Deserialize)]
pub struct SQLRequest {
    pub query: String,
    #[serde(default = "default_offset")]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default = "default_batch_size")]
    pub batch_size: u32,
}

impl SQLRequest  {
    fn contains_pagination_keywords(&self) -> bool {
        let q = self.query.to_lowercase();
        q.contains("limit") || q.contains("offset")
    }

    pub fn build_paginated_query(&self) -> Result<String, String> {
        if self.contains_pagination_keywords() {
            Err("The SQL query should not contain LIMIT or OFFSET clauses.".into())
        } else {
            let sql = format!("{} LIMIT {} OFFSET {}", self.query, self.limit, self.offset);
            Ok(sql)
        }
    }
}

fn default_offset() -> u32 {
    0
}

fn default_limit() -> u32 {
    u32::MAX
}

fn default_batch_size() -> u32 {
    1000
}
