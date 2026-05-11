use tantivy::{
    collector::{Count, MultiCollector, TopDocs},
    Order,
};
use tantivy::aggregation::{AggContextParams, AggregationCollector};
use tantivy::schema::FieldType;

use crate::error::{AppError, Result};
use crate::index::manager::IndexHandle;
use crate::search::compiler::build_final_query;
use crate::search::model::{EsQuery, EsSearchRequest, SearchResponse};
use crate::search::result::{build_snippet_gens, process_top_docs, TopDocsResult};

fn run_search<C, R>(
    handle: &IndexHandle,
    searcher: &tantivy::Searcher,
    query: &dyn tantivy::query::Query,
    req: &EsSearchRequest,
    snippet_gens: &[(String, tantivy::snippet::SnippetGenerator)],
    top_collector: C,
    aggs: Option<&tantivy::aggregation::agg_req::Aggregations>,
) -> Result<(usize, Vec<crate::search::model::SearchHit>, Option<serde_json::Value>)>
where
    C: tantivy::collector::Collector<Fruit = Vec<R>>,
    R: TopDocsResult + Send + 'static,
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
        let (field_name, order) = req.sort[0]
            .iter()
            .next()
            .ok_or_else(|| AppError::BadRequest("sort entry cannot be empty".to_string()))?;
        if field_name == "_score" {
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
            macro_rules! execute_sort {
                ($collector:expr) => {{
                    run_search(
                        handle,
                        &searcher,
                        &*query,
                        req,
                        &snippet_gens,
                        $collector,
                        req.aggs.as_ref(),
                    )?
                }};
            }

            match entry.field_type() {
                FieldType::U64(_) => execute_sort!(
                    TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<u64>(field_name, order)
                ),
                FieldType::I64(_) => execute_sort!(
                    TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<i64>(field_name, order)
                ),
                FieldType::F64(_) => execute_sort!(
                    TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<f64>(field_name, order)
                ),
                FieldType::Date(_) => execute_sort!(
                    TopDocs::with_limit(req.size + req.from)
                        .order_by_fast_field::<tantivy::DateTime>(field_name, order)
                ),
                FieldType::Str(_) => execute_sort!(
                    TopDocs::with_limit(req.size + req.from)
                        .order_by_string_fast_field(field_name, order)
                ),
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
