use serde_json::Value as JsonValue;
use tantivy::TantivyDocument;
use tantivy::schema::{Field, FieldType, OwnedValue, Term};

use crate::error::{AppError, Result};

/// Convert a JSON object into a Tantivy document based on the schema.
pub fn json_to_doc(schema: &tantivy::schema::Schema, data: &JsonValue) -> Result<TantivyDocument> {
    let obj = data
        .as_object()
        .ok_or_else(|| AppError::BadRequest("document must be a JSON object".to_string()))?;

    let mut doc = TantivyDocument::default();
    for (key, val) in obj {
        let field = schema
            .get_field(key)
            .map_err(|_| AppError::FieldNotFound(key.clone()))?;
        let field_entry = schema.get_field_entry(field);
        if let JsonValue::Array(arr) = val {
            for item in arr {
                let owned_val = convert_json_to_field_value(item, field_entry.field_type())?;
                doc.add_field_value(field, &owned_val);
            }
        } else {
            let owned_val = convert_json_to_field_value(val, field_entry.field_type())?;
            doc.add_field_value(field, &owned_val);
        }
    }
    Ok(doc)
}

fn convert_json_to_field_value(
    value: &JsonValue,
    field_type: &tantivy::schema::FieldType,
) -> Result<OwnedValue> {
    match field_type {
        tantivy::schema::FieldType::Str(_) => Ok(OwnedValue::Str(
            value
                .as_str()
                .ok_or_else(|| AppError::Schema("expected string".to_string()))?
                .to_string(),
        )),
        tantivy::schema::FieldType::U64(_) => {
            Ok(OwnedValue::U64(value.as_u64().ok_or_else(|| {
                AppError::Schema("expected u64".to_string())
            })?))
        }
        tantivy::schema::FieldType::I64(_) => {
            Ok(OwnedValue::I64(value.as_i64().ok_or_else(|| {
                AppError::Schema("expected i64".to_string())
            })?))
        }
        tantivy::schema::FieldType::F64(_) => {
            Ok(OwnedValue::F64(value.as_f64().ok_or_else(|| {
                AppError::Schema("expected f64".to_string())
            })?))
        }
        tantivy::schema::FieldType::Bool(_) => {
            Ok(OwnedValue::Bool(value.as_bool().ok_or_else(|| {
                AppError::Schema("expected bool".to_string())
            })?))
        }
        tantivy::schema::FieldType::Date(_) => {
            let s = value
                .as_str()
                .ok_or_else(|| AppError::Schema("expected date string".to_string()))?;
            let dt = s
                .parse::<chrono::DateTime<chrono::Utc>>()
                .map_err(|e| AppError::Schema(format!("invalid date: {e}")))?;
            Ok(OwnedValue::Date(tantivy::DateTime::from_timestamp_micros(
                dt.timestamp_micros(),
            )))
        }
        tantivy::schema::FieldType::Facet(_) => {
            let s = value
                .as_str()
                .ok_or_else(|| AppError::Schema("expected facet string".to_string()))?;
            Ok(OwnedValue::Facet(tantivy::schema::Facet::from_text(s)?))
        }
        tantivy::schema::FieldType::Bytes(_) => {
            let s = value
                .as_str()
                .ok_or_else(|| AppError::Schema("expected base64 bytes".to_string()))?;
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(s)
                .map_err(|e| AppError::Schema(format!("base64: {e}")))?;
            Ok(OwnedValue::Bytes(bytes))
        }
        tantivy::schema::FieldType::JsonObject(_) => {
            let map = value
                .as_object()
                .ok_or_else(|| AppError::Schema("expected json object".to_string()))?;
            let mut obj = Vec::new();
            for (k, v) in map {
                obj.push((k.clone(), json_to_tantivy_value(v)?));
            }
            Ok(OwnedValue::Object(obj))
        }
        _ => Err(AppError::Schema("unsupported field type".to_string())),
    }
}

fn json_to_tantivy_value(value: &JsonValue) -> Result<OwnedValue> {
    match value {
        JsonValue::Null => Ok(OwnedValue::Null),
        JsonValue::Bool(b) => Ok(OwnedValue::Bool(*b)),
        JsonValue::Number(n) => {
            if let Some(u) = n.as_u64() {
                Ok(OwnedValue::U64(u))
            } else if let Some(i) = n.as_i64() {
                Ok(OwnedValue::I64(i))
            } else if let Some(f) = n.as_f64() {
                Ok(OwnedValue::F64(f))
            } else {
                Err(AppError::Schema("invalid number".to_string()))
            }
        }
        JsonValue::String(s) => Ok(OwnedValue::Str(s.clone())),
        JsonValue::Array(arr) => {
            let mut vec = Vec::new();
            for v in arr {
                vec.push(json_to_tantivy_value(v)?);
            }
            Ok(OwnedValue::Array(vec))
        }
        JsonValue::Object(map) => {
            let mut obj = Vec::new();
            for (k, v) in map {
                obj.push((k.clone(), json_to_tantivy_value(v)?));
            }
            Ok(OwnedValue::Object(obj))
        }
    }
}

/// Build a Tantivy Term from a string value based on the field type.
pub fn str_to_term(field: Field, field_type: &FieldType, value: &str) -> Result<Term> {
    match field_type {
        FieldType::Str(_) => Ok(Term::from_field_text(field, value)),
        FieldType::U64(_) => Ok(Term::from_field_u64(field, value.parse()?)),
        FieldType::I64(_) => Ok(Term::from_field_i64(field, value.parse()?)),
        FieldType::F64(_) => Ok(Term::from_field_f64(field, value.parse()?)),
        FieldType::Bool(_) => Ok(Term::from_field_bool(field, value.parse()?)),
        FieldType::Date(_) => {
            let dt = value
                .parse::<chrono::DateTime<chrono::Utc>>()
                .map_err(|e| AppError::Schema(format!("invalid date: {e}")))?;
            Ok(Term::from_field_date_for_search(
                field,
                tantivy::DateTime::from_timestamp_micros(dt.timestamp_micros()),
            ))
        }
        _ => Err(AppError::Schema("unsupported field type".to_string())),
    }
}

/// Build a Tantivy Term from a JSON value based on the field type.
pub fn json_value_to_term(
    field: Field,
    value: &JsonValue,
    field_type: &FieldType,
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
                tantivy::DateTime::from_timestamp_micros(dt.timestamp_micros()),
            ))
        }
        _ => Err(AppError::Schema("unsupported filter field type".to_string())),
    }
}

fn owned_value_to_json(value: OwnedValue) -> JsonValue {
    match value {
        OwnedValue::Str(s) => JsonValue::String(s),
        OwnedValue::U64(v) => JsonValue::Number(v.into()),
        OwnedValue::I64(v) => JsonValue::Number(v.into()),
        OwnedValue::F64(v) => serde_json::Number::from_f64(v)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        OwnedValue::Bool(v) => JsonValue::Bool(v),
        OwnedValue::Date(v) => {
            let micros = v.into_timestamp_micros();
            let secs = micros.div_euclid(1_000_000);
            let rem_micros = micros.rem_euclid(1_000_000);
            let nsecs = (rem_micros * 1_000) as u32;
            match chrono::DateTime::from_timestamp(secs, nsecs) {
                Some(dt) => JsonValue::String(dt.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)),
                None => {
                    tracing::warn!(timestamp_micros = micros, "invalid timestamp from index");
                    JsonValue::Null
                }
            }
        }
        OwnedValue::Facet(v) => JsonValue::String(v.to_string()),
        OwnedValue::Bytes(v) => {
            use base64::Engine;
            JsonValue::String(base64::engine::general_purpose::STANDARD.encode(v))
        }
        OwnedValue::Array(arr) => JsonValue::Array(arr.into_iter().map(owned_value_to_json).collect()),
        OwnedValue::Object(obj) => serde_json::to_value(obj).unwrap_or(JsonValue::Null),
        OwnedValue::PreTokStr(_) | OwnedValue::IpAddr(_) => JsonValue::Null,
        OwnedValue::Null => JsonValue::Null,
    }
}

pub fn doc_to_json(schema: &tantivy::schema::Schema, doc: &TantivyDocument) -> JsonValue {
    let mut map = serde_json::Map::new();
    for (field, value) in doc.field_values() {
        let name = schema.get_field_name(field);
        let owned: OwnedValue = value.into();
        let json_val = owned_value_to_json(owned);
        if let Some(existing) = map.get_mut(name) {
            if let JsonValue::Array(arr) = existing {
                arr.push(json_val);
            } else {
                let old = existing.clone();
                *existing = JsonValue::Array(vec![old, json_val]);
            }
        } else {
            map.insert(name.to_string(), json_val);
        }
    }
    JsonValue::Object(map)
}
