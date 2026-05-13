use tantivy::aggregation::{AggContextParams, AggregationCollector};
use tantivy::schema::FieldType;
use tantivy::{
    Order,
    collector::{Count, MultiCollector, TopDocs},
};

use crate::error::{AppError, Result};
use crate::index::manager::IndexHandle;
use crate::search::compiler::build_final_query;
use crate::search::model::{EsQuery, EsSearchRequest, SearchResponse};
use crate::search::result::{TopDocsResult, build_snippet_gens, process_top_docs};

fn run_search<C, R>(
    handle: &IndexHandle,
    searcher: &tantivy::Searcher,
    query: &dyn tantivy::query::Query,
    req: &EsSearchRequest,
    snippet_gens: &[(String, tantivy::snippet::SnippetGenerator)],
    top_collector: C,
    aggs: Option<tantivy::aggregation::agg_req::Aggregations>,
) -> Result<(
    usize,
    Vec<crate::search::model::SearchHit>,
    Option<serde_json::Value>,
)>
where
    C: tantivy::collector::Collector<Fruit = Vec<R>>,
    R: TopDocsResult + Send + 'static,
{
    let mut collectors = MultiCollector::new();
    let top_handle = collectors.add_collector(top_collector);
    let count_handle = collectors.add_collector(Count);
    let agg_handle = aggs.map(|a| {
        collectors.add_collector(AggregationCollector::from_aggs(
            a,
            AggContextParams::default(),
        ))
    });
    let mut multi_fruit = searcher.search(query, &collectors)?;
    let total = count_handle.extract(&mut multi_fruit);
    let top_docs = top_handle.extract(&mut multi_fruit);
    let hits = process_top_docs(handle, searcher, req, snippet_gens, top_docs)?;
    let agg_json = match agg_handle {
        Some(h) => Some(serde_json::to_value(h.extract(&mut multi_fruit))?),
        None => None,
    };
    Ok((total, hits, agg_json))
}

/// Internal synchronous search logic, extracted so it can be run inside spawn_blocking.
/// Maximum result window for search (from + size) to prevent memory DoS.
const MAX_RESULT_WINDOW: usize = 10_000;

fn do_search(handle: &IndexHandle, req: &EsSearchRequest) -> Result<SearchResponse> {
    let window = req
        .from
        .checked_add(req.size)
        .ok_or_else(|| AppError::BadRequest("offset + size overflow".to_string()))?;
    if window > MAX_RESULT_WINDOW {
        return Err(AppError::BadRequest(format!(
            "offset + size cannot exceed {MAX_RESULT_WINDOW}"
        )));
    }

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
            req.aggs.clone(),
        )?
    } else {
        if req.sort.len() > 1 {
            return Err(AppError::BadRequest(
                "Multiple sort fields are not supported yet".to_string(),
            ));
        }
        let field_name = &req.sort[0].field;
        let order = req.sort[0].order;
        if field_name == "_score" {
            let collector = TopDocs::with_limit(req.size + req.from).order_by_score();
            run_search(
                handle,
                &searcher,
                &*query,
                req,
                &snippet_gens,
                collector,
                req.aggs.clone(),
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
            let order: Order = order.into();

            match entry.field_type() {
                FieldType::U64(_) => run_search(
                    handle,
                    &searcher,
                    &*query,
                    req,
                    &snippet_gens,
                    TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<u64>(field_name, order),
                    req.aggs.clone(),
                )?,
                FieldType::I64(_) => run_search(
                    handle,
                    &searcher,
                    &*query,
                    req,
                    &snippet_gens,
                    TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<i64>(field_name, order),
                    req.aggs.clone(),
                )?,
                FieldType::F64(_) => run_search(
                    handle,
                    &searcher,
                    &*query,
                    req,
                    &snippet_gens,
                    TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<f64>(field_name, order),
                    req.aggs.clone(),
                )?,
                FieldType::Date(_) => run_search(
                    handle,
                    &searcher,
                    &*query,
                    req,
                    &snippet_gens,
                    TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<tantivy::DateTime>(field_name, order),
                    req.aggs.clone(),
                )?,
                FieldType::Str(_) => run_search(
                    handle,
                    &searcher,
                    &*query,
                    req,
                    &snippet_gens,
                    TopDocs::with_limit(req.size + req.from)
                        .order_by_string_fast_field(field_name, order),
                    req.aggs.clone(),
                )?,
                _ => {
                    return Err(AppError::BadRequest(format!(
                        "unsupported sort field type for '{}'",
                        field_name
                    )));
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
            .and_then(|q| match q {
                EsQuery::QueryString { query } => Some(query.clone()),
                _ => None,
            })
            .unwrap_or_default(),
        limit: req.size,
        offset: req.from,
        aggregations,
    })
}

pub async fn search_index(handle: std::sync::Arc<IndexHandle>, req: &EsSearchRequest) -> Result<SearchResponse> {
    // Search can be CPU-heavy (especially with aggregations), so run it on
    // the blocking thread pool to avoid stalling the async runtime.
    let req = req.clone();
    tokio::task::spawn_blocking(move || do_search(&handle, &req))
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))?
}
