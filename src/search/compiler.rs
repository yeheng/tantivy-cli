use tantivy::query::{AllQuery, BooleanQuery, ConstScoreQuery, Occur, QueryParser, RangeQuery, TermQuery};
use tantivy::schema::{FieldType, IndexRecordOption};

use crate::error::{AppError, Result};
use crate::index::doc::json_value_to_term;
use crate::index::manager::IndexHandle;
use crate::search::model::{EsQuery, EsSearchRequest};

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
                subqueries.push((
                    Occur::Must,
                    Box::new(ConstScoreQuery::new(build_es_query(handle, q)?, 0.0)),
                ));
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
            if map.len() != 1 {
                return Err(AppError::Query(
                    "match query requires exactly one field".to_string(),
                ));
            }
            let (field_name, text) = map
                .iter()
                .next()
                .unwrap();
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
            if map.len() != 1 {
                return Err(AppError::Query(
                    "term query requires exactly one field".to_string(),
                ));
            }
            let (field_name, value) = map
                .iter()
                .next()
                .unwrap();
            let field = handle
                .schema
                .get_field(field_name)
                .map_err(|_| AppError::FieldNotFound(field_name.clone()))?;
            let entry = handle.schema.get_field_entry(field);
            let term = json_value_to_term(field, value, entry.field_type())?;
            Ok(Box::new(TermQuery::new(term, IndexRecordOption::Basic)))
        }
        EsQuery::Range(map) => {
            if map.len() != 1 {
                return Err(AppError::Query(
                    "range query requires exactly one field".to_string(),
                ));
            }
            let (field_name, params) = map
                .iter()
                .next()
                .unwrap();
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
