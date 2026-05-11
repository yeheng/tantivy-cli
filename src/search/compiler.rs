use serde_json::Value as JsonValue;
use tantivy::query::{AllQuery, BooleanQuery, Occur, QueryParser, RangeQuery, TermQuery};
use tantivy::schema::{FieldType, IndexRecordOption, Term};

use crate::error::{AppError, Result};
use crate::index::manager::IndexHandle;
use crate::search::model::{EsQuery, EsSearchRequest};

pub fn json_value_to_term(
    field: tantivy::schema::Field,
    value: &JsonValue,
    field_type: &tantivy::schema::FieldType,
) -> Result<Term> {
    match field_type {
        FieldType::Str(_) => {
            let s = value
                .as_str()
                .ok_or_else(|| AppError::Schema("expected string".to_string()))?;
            Ok(Term::from_field_text(field, s))
        }
        FieldType::U64(_) => {
            let v = value
                .as_u64()
                .ok_or_else(|| AppError::Schema("expected u64".to_string()))?;
            Ok(Term::from_field_u64(field, v))
        }
        FieldType::I64(_) => {
            let v = value
                .as_i64()
                .ok_or_else(|| AppError::Schema("expected i64".to_string()))?;
            Ok(Term::from_field_i64(field, v))
        }
        FieldType::F64(_) => {
            let v = value
                .as_f64()
                .ok_or_else(|| AppError::Schema("expected f64".to_string()))?;
            Ok(Term::from_field_f64(field, v))
        }
        FieldType::Bool(_) => {
            let v = value
                .as_bool()
                .ok_or_else(|| AppError::Schema("expected bool".to_string()))?;
            Ok(Term::from_field_bool(field, v))
        }
        FieldType::Date(_) => {
            let s = value
                .as_str()
                .ok_or_else(|| AppError::Schema("expected date string".to_string()))?;
            let dt = s
                .parse::<chrono::DateTime<chrono::Utc>>()
                .map_err(|e| AppError::Schema(format!("invalid date: {e}")))?;
            Ok(Term::from_field_date_for_search(
                field,
                tantivy::DateTime::from_timestamp_secs(dt.timestamp()),
            ))
        }
        _ => Err(AppError::Schema(
            "unsupported filter field type".to_string(),
        )),
    }
}

pub fn build_es_query(handle: &IndexHandle, q: &EsQuery) -> Result<Box<dyn tantivy::query::Query>> {
    match q {
        EsQuery::Bool {
            must,
            filter,
            should,
            must_not,
        } => {
            let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();
            for q in must {
                subqueries.push((Occur::Must, build_es_query(handle, q)?));
            }
            for q in filter {
                subqueries.push((Occur::Must, build_es_query(handle, q)?));
            }
            for q in should {
                subqueries.push((Occur::Should, build_es_query(handle, q)?));
            }
            for q in must_not {
                subqueries.push((Occur::MustNot, build_es_query(handle, q)?));
            }
            Ok(Box::new(BooleanQuery::new(subqueries)))
        }
        EsQuery::Match(map) => {
            let (field_name, text) = map
                .iter()
                .next()
                .ok_or_else(|| AppError::Query("match query requires a field".to_string()))?;
            let field = handle
                .schema
                .get_field(field_name)
                .map_err(|_| AppError::FieldNotFound(field_name.clone()))?;
            let query_parser = QueryParser::for_index(&handle.index, vec![field]);
            let parsed = query_parser
                .parse_query(text)
                .map_err(|e| AppError::Query(e.to_string()))?;
            Ok(Box::new(parsed))
        }
        EsQuery::Term(map) => {
            let (field_name, value) = map
                .iter()
                .next()
                .ok_or_else(|| AppError::Query("term query requires a field".to_string()))?;
            let field = handle
                .schema
                .get_field(field_name)
                .map_err(|_| AppError::FieldNotFound(field_name.clone()))?;
            let entry = handle.schema.get_field_entry(field);
            let term = json_value_to_term(field, value, entry.field_type())?;
            Ok(Box::new(TermQuery::new(term, IndexRecordOption::Basic)))
        }
        EsQuery::Range(map) => {
            let (field_name, params) = map
                .iter()
                .next()
                .ok_or_else(|| AppError::Query("range query requires a field".to_string()))?;
            let field = handle
                .schema
                .get_field(field_name)
                .map_err(|_| AppError::FieldNotFound(field_name.clone()))?;
            let entry = handle.schema.get_field_entry(field);
            let ft = entry.field_type();
            let lower = match (&params.gte, &params.gt) {
                (Some(v), None) => std::ops::Bound::Included(json_value_to_term(field, v, ft)?),
                (None, Some(v)) => std::ops::Bound::Excluded(json_value_to_term(field, v, ft)?),
                (None, None) => std::ops::Bound::Unbounded,
                (Some(_), Some(_)) => {
                    return Err(AppError::Query(
                        "cannot specify both gte and gt".to_string(),
                    ));
                }
            };
            let upper = match (&params.lte, &params.lt) {
                (Some(v), None) => std::ops::Bound::Included(json_value_to_term(field, v, ft)?),
                (None, Some(v)) => std::ops::Bound::Excluded(json_value_to_term(field, v, ft)?),
                (None, None) => std::ops::Bound::Unbounded,
                (Some(_), Some(_)) => {
                    return Err(AppError::Query(
                        "cannot specify both lte and lt".to_string(),
                    ));
                }
            };
            Ok(Box::new(RangeQuery::new(lower, upper)))
        }
        EsQuery::QueryString { query: qstr } => {
            let text_fields: Vec<_> = handle
                .schema
                .fields()
                .filter(|(_, entry)| matches!(entry.field_type(), FieldType::Str(_)))
                .map(|(field, _)| field)
                .collect();
            if text_fields.is_empty() {
                return Err(AppError::Schema("no text fields to search".to_string()));
            }
            let query_parser = QueryParser::for_index(&handle.index, text_fields);
            let parsed = query_parser
                .parse_query(qstr)
                .map_err(|e| AppError::Query(e.to_string()))?;
            Ok(Box::new(parsed))
        }
    }
}

pub fn build_final_query(
    handle: &IndexHandle,
    req: &EsSearchRequest,
) -> Result<Box<dyn tantivy::query::Query>> {
    match &req.query {
        None => Ok(Box::new(AllQuery)),
        Some(q) => build_es_query(handle, q),
    }
}
