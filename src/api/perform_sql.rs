use axum::{
    extract::State,
    response::Response,
    body::Body,Json
};
use futures::StreamExt;
use crate::domain::dto::SQLRequest;
use sea_orm::{DatabaseConnection, DbBackend, Statement, StreamTrait};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use sqlparser::ast::{Statement as ParserStatement, SelectItem, Expr};

pub async fn perform_sql(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<SQLRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {

    let batch_size = payload.batch_size as usize;
    let sql = payload.build_paginated_query()
        .map_err(|err| (axum::http::StatusCode::BAD_REQUEST, err))?;

    let is_valid = is_only_gallery_id_returned(&sql);

    if !is_valid {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "SQL must return only 'gallery_id' column.".to_string(),
        ));
    };

    let stmt = Statement::from_string(DbBackend::Postgres, sql);

    let query_stream = match db.stream_raw(stmt).await {
        Ok(stream) => stream,
        Err(err) => {
            return Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("SQL execution error: {}", err),
            ))
        }
    };

    let byte_stream = query_stream
    .chunks(batch_size)
    .map(move |batch| {
        // batch は Vec<Result<QueryResult, DbErr>> 型になる
        
        // メモリ効率のため、あらかじめバッファを確保（i32 = 4 bytes）
        let mut buffer = Vec::with_capacity(batch.len() * 4);

        for row_result in batch {
            match row_result {
                Ok(row) => {
                    let gallery_id: i32 = row.try_get_by_index(0).unwrap_or_default();
                    // リトルエンディアンでバッファに追加
                    buffer.extend_from_slice(&gallery_id.to_le_bytes());
                }
                Err(err) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Row retrieval error: {}", err),
                    ));
                }
            }
        }
        
        // まとまったバッファを一つの塊として流す
        Ok::<_, std::io::Error>(buffer)
    });

    Response::builder()
        .header("X-Batch-Size", batch_size.to_string())
        .header("Content-Type", "application/octet-stream")
        .body(Body::from_stream(byte_stream))
        .map_err(|err| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build response: {}", err),
        ))
}

pub fn is_only_gallery_id_returned(sql: &str) -> bool {
    let dialect = PostgreSqlDialect {};
    let ast = Parser::parse_sql(&dialect, sql).unwrap();

    if let ParserStatement::Query(query) = &ast[0] {
        if let sqlparser::ast::SetExpr::Select(select) = &*query.body {
            if select.projection.len() != 1 {
                return false;
            }

            return match &select.projection[0] {
                SelectItem::UnnamedExpr(Expr::Identifier(ident)) => {
                    ident.value.to_lowercase() == "gallery_id"
                }
                SelectItem::ExprWithAlias { alias, .. } => {
                    alias.value.to_lowercase() == "gallery_id"
                }
                _ => false,
            };
        }
    }
    false
}
