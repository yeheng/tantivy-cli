use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tantivy::aggregation::{AggContextParams, AggregationCollector};
use tantivy::schema::{FieldType, IndexRecordOption, Term};
use tantivy::{
    collector::{Count, MultiCollector, TopDocs},
    query::{AllQuery, BooleanQuery, Occur, QueryParser, RangeQuery, TermQuery},
    snippet::SnippetGenerator,
    DocAddress, Order, TantivyDocument,
};

use crate::error::{AppError, Result};
use crate::index::manager::IndexHandle;
use crate::index::ops::doc_to_json;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl From<SortOrder> for Order {
    fn from(o: SortOrder) -> Self {
        match o {
            SortOrder::Asc => Order::Asc,
            SortOrder::Desc => Order::Desc,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct EsSearchRequest {
    #[serde(alias = "offset", default)]
    pub from: usize,
    #[serde(alias = "limit", default = "default_size")]
    pub size: usize,
    pub query: Option<EsQuery>,
    #[serde(default)]
    pub sort: Vec<HashMap<String, SortOrder>>,
    #[serde(default)]
    pub aggs: Option<tantivy::aggregation::agg_req::Aggregations>,
    #[serde(default)]
    pub highlight_fields: Vec<String>,
    #[serde(default = "default_snippet_chars")]
    pub snippet_max_chars: usize,
    #[serde(default)]
    pub _source: Option<Vec<String>>,
}

fn default_size() -> usize {
    10
}
fn default_snippet_chars() -> usize {
    150
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EsQuery {
    Bool {
        #[serde(default)]
        must: Vec<EsQuery>,
        #[serde(default)]
        filter: Vec<EsQuery>,
        #[serde(default)]
        should: Vec<EsQuery>,
        #[serde(default)]
        must_not: Vec<EsQuery>,
    },
    Match(HashMap<String, String>),
    Term(HashMap<String, JsonValue>),
    Range(HashMap<String, RangeParams>),
    QueryString {
        query: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct RangeParams {
    #[serde(default)]
    pub gte: Option<JsonValue>,
    #[serde(default)]
    pub gt: Option<JsonValue>,
    #[serde(default)]
    pub lte: Option<JsonValue>,
    #[serde(default)]
    pub lt: Option<JsonValue>,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub doc: JsonValue,
    pub score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippets: Option<JsonValue>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub total: usize,
    pub hits: Vec<SearchHit>,
    pub query: String,
    pub limit: usize,
    pub offset: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregations: Option<JsonValue>,
}

fn json_value_to_term(
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
        _ => Err(AppError::Schema("unsupported filter field type".to_string())),
    }
}

fn build_es_query(handle: &IndexHandle, q: &EsQuery) -> Result<Box<dyn tantivy::query::Query>> {
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
            let (field_name, text) = map.iter().next().ok_or_else(|| {
                AppError::Query("match query requires a field".to_string())
            })?;
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
            let (field_name, value) = map.iter().next().ok_or_else(|| {
                AppError::Query("term query requires a field".to_string())
            })?;
            let field = handle
                .schema
                .get_field(field_name)
                .map_err(|_| AppError::FieldNotFound(field_name.clone()))?;
            let entry = handle.schema.get_field_entry(field);
            let term = json_value_to_term(field, value, entry.field_type())?;
            Ok(Box::new(TermQuery::new(term, IndexRecordOption::Basic)))
        }
        EsQuery::Range(map) => {
            let (field_name, params) = map.iter().next().ok_or_else(|| {
                AppError::Query("range query requires a field".to_string())
            })?;
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
                    return Err(AppError::Query("cannot specify both gte and gt".to_string()))
                }
            };
            let upper = match (&params.lte, &params.lt) {
                (Some(v), None) => std::ops::Bound::Included(json_value_to_term(field, v, ft)?),
                (None, Some(v)) => std::ops::Bound::Excluded(json_value_to_term(field, v, ft)?),
                (None, None) => std::ops::Bound::Unbounded,
                (Some(_), Some(_)) => {
                    return Err(AppError::Query("cannot specify both lte and lt".to_string()))
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

fn build_final_query(
    handle: &IndexHandle,
    req: &EsSearchRequest,
) -> Result<Box<dyn tantivy::query::Query>> {
    match &req.query {
        None => Ok(Box::new(AllQuery)),
        Some(q) => build_es_query(handle, q),
    }
}

/// Trait to abstract over `TopDocs` collector results.
trait TopDocsResult {
    fn doc_address(&self) -> DocAddress;
    fn to_search_hit(&self, doc_json: JsonValue, snippets: Option<JsonValue>) -> SearchHit;
}

impl TopDocsResult for (f32, DocAddress) {
    fn doc_address(&self) -> DocAddress {
        self.1
    }
    fn to_search_hit(&self, doc_json: JsonValue, snippets: Option<JsonValue>) -> SearchHit {
        SearchHit {
            doc: doc_json,
            score: Some(self.0),
            snippets,
        }
    }
}

impl<T: Clone + Send + Sync + std::fmt::Debug + 'static> TopDocsResult for (Option<T>, DocAddress) {
    fn doc_address(&self) -> DocAddress {
        self.1
    }
    fn to_search_hit(&self, doc_json: JsonValue, snippets: Option<JsonValue>) -> SearchHit {
        SearchHit {
            doc: doc_json,
            score: None,
            snippets,
        }
    }
}

fn build_snippet_gens(
    handle: &IndexHandle,
    searcher: &tantivy::Searcher,
    query: &dyn tantivy::query::Query,
    req: &EsSearchRequest,
) -> Result<Vec<(String, SnippetGenerator)>> {
    let mut gens = Vec::new();
    if !req.highlight_fields.is_empty() {
        for field_name in &req.highlight_fields {
            if let Ok(field) = handle.schema.get_field(field_name) {
                if matches!(
                    handle.schema.get_field_entry(field).field_type(),
                    FieldType::Str(_)
                ) {
                    let mut generator = SnippetGenerator::create(searcher, query, field)?;
                    generator.set_max_num_chars(req.snippet_max_chars);
                    gens.push((field_name.clone(), generator));
                }
            }
        }
    }
    Ok(gens)
}

fn process_top_docs<R: TopDocsResult>(
    handle: &IndexHandle,
    searcher: &tantivy::Searcher,
    req: &EsSearchRequest,
    snippet_gens: &[(String, SnippetGenerator)],
    top_docs: Vec<R>,
) -> Result<Vec<SearchHit>> {
    let mut hits = Vec::with_capacity(top_docs.len().saturating_sub(req.from));
    for (idx, r) in top_docs.into_iter().enumerate() {
        if idx < req.from {
            continue;
        }
        let doc = searcher.doc::<TantivyDocument>(r.doc_address())?;
        let mut doc_json = doc_to_json(&handle.schema, &doc);

        // _source filtering
        if let Some(ref sources) = req._source {
            if let JsonValue::Object(ref mut map) = doc_json {
                map.retain(|k, _| sources.contains(k));
            }
        }

        let mut snippets = serde_json::Map::new();
        for (field_name, generator) in snippet_gens {
            let snippet = generator.snippet_from_doc(&doc);
            snippets.insert(field_name.clone(), JsonValue::String(snippet.to_html()));
        }

        hits.push(r.to_search_hit(
            doc_json,
            if snippets.is_empty() {
                None
            } else {
                Some(JsonValue::Object(snippets))
            },
        ));
    }
    Ok(hits)
}

fn run_search<C, R>(
    handle: &IndexHandle,
    searcher: &tantivy::Searcher,
    query: &dyn tantivy::query::Query,
    req: &EsSearchRequest,
    snippet_gens: &[(String, SnippetGenerator)],
    top_collector: C,
    aggs: Option<&tantivy::aggregation::agg_req::Aggregations>,
) -> Result<(usize, Vec<SearchHit>, Option<JsonValue>)>
where
    C: tantivy::collector::Collector<Fruit = Vec<R>>,
    R: TopDocsResult + Send + 'static
{
    if let Some(aggs) = aggs {
        let mut collectors = MultiCollector::new();
        let top_handle = collectors.add_collector(top_collector);
        let count_handle = collectors.add_collector(Count);
        let agg_handle = collectors.add_collector(AggregationCollector::from_aggs(
            aggs.clone(),
            AggContextParams::default(),
        ));
        let mut multi_fruit = searcher.search(query, &collectors)?;
        let total = count_handle.extract(&mut multi_fruit);
        let top_docs = top_handle.extract(&mut multi_fruit);
        let agg_results = agg_handle.extract(&mut multi_fruit);
        let hits = process_top_docs(handle, searcher, req, snippet_gens, top_docs)?;
        let agg_json = serde_json::to_value(agg_results)?;
        Ok((total, hits, Some(agg_json)))
    } else {
        let mut collectors = MultiCollector::new();
        let top_handle = collectors.add_collector(top_collector);
        let count_handle = collectors.add_collector(Count);
        let mut multi_fruit = searcher.search(query, &collectors)?;
        let total = count_handle.extract(&mut multi_fruit);
        let top_docs = top_handle.extract(&mut multi_fruit);
        let hits = process_top_docs(handle, searcher, req, snippet_gens, top_docs)?;
        Ok((total, hits, None))
    }
}

pub async fn search_index(
    handle: &IndexHandle,
    req: &EsSearchRequest,
) -> Result<SearchResponse> {
    let searcher = handle.reader.searcher();
    let query = build_final_query(handle, req)?;
    let snippet_gens = build_snippet_gens(handle, &searcher, &*query, req)?;

    let (total, hits, aggregations) = if req.sort.is_empty() {
        let collector = TopDocs::with_limit(req.size + req.from).order_by_score();
        run_search(
            handle,
            &searcher,
            &*query,
            req,
            &snippet_gens,
            collector,
            req.aggs.as_ref(),
        )?
    } else {
        // Only support single-field sorting for now.
        let (field_name, order) = req.sort[0].iter().next().ok_or_else(|| {
            AppError::BadRequest("sort entry cannot be empty".to_string())
        })?;
        if field_name == "_score" {
            let collector =
                TopDocs::with_limit(req.size + req.from).order_by_score();
            run_search(
                handle,
                &searcher,
                &*query,
                req,
                &snippet_gens,
                collector,
                req.aggs.as_ref(),
            )?
        } else {
            let field = handle
                .schema
                .get_field(field_name)
                .map_err(|_| AppError::FieldNotFound(field_name.clone()))?;
            let entry = handle.schema.get_field_entry(field);
            if !entry.is_fast() {
                return Err(AppError::BadRequest(format!(
                    "sort field '{}' must be a fast field",
                    field_name
                )));
            }
            let order: Order = (*order).into();
            match entry.field_type() {
                FieldType::U64(_) => {
                    let collector = TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<u64>(field_name, order);
                    run_search(
                        handle,
                        &searcher,
                        &*query,
                        req,
                        &snippet_gens,
                        collector,
                        req.aggs.as_ref(),
                    )?
                }
                FieldType::I64(_) => {
                    let collector = TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<i64>(field_name, order);
                    run_search(
                        handle,
                        &searcher,
                        &*query,
                        req,
                        &snippet_gens,
                        collector,
                        req.aggs.as_ref(),
                    )?
                }
                FieldType::F64(_) => {
                    let collector = TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<f64>(field_name, order);
                    run_search(
                        handle,
                        &searcher,
                        &*query,
                        req,
                        &snippet_gens,
                        collector,
                        req.aggs.as_ref(),
                    )?
                }
                FieldType::Date(_) => {
                    let collector = TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<tantivy::DateTime>(field_name, order);
                    run_search(
                        handle,
                        &searcher,
                        &*query,
                        req,
                        &snippet_gens,
                        collector,
                        req.aggs.as_ref(),
                    )?
                }
                FieldType::Str(_) => {
                    let collector = TopDocs::with_limit(req.size + req.from)
                        .order_by_string_fast_field(field_name, order);
                    run_search(
                        handle,
                        &searcher,
                        &*query,
                        req,
                        &snippet_gens,
                        collector,
                        req.aggs.as_ref(),
                    )?
                }
                _ => {
                    return Err(AppError::BadRequest(format!(
                        "unsupported sort field type for '{}'",
                        field_name
                    )))
                }
            }
        }
    };

    Ok(SearchResponse {
        total,
        hits,
        query: req
            .query
            .as_ref()
            .map(|q| match q {
                EsQuery::QueryString { query } => query.clone(),
                _ => format!("{:?}", q),
            })
            .unwrap_or_default(),
        limit: req.size,
        offset: req.from,
        aggregations,
    })
}
