use serde::{Deserialize, Serialize};
use tantivy::schema::{
    BytesOptions, DateOptions, FacetOptions, JsonObjectOptions, NumericOptions, Schema,
    SchemaBuilder, FAST, STORED, STRING, TEXT,
};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Text,
    String,
    U64,
    I64,
    F64,
    Bool,
    Date,
    Facet,
    Bytes,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub kind: FieldKind,
    pub stored: bool,
    pub indexed: bool,
    pub fast: bool,
}

impl FieldDef {
    pub fn add_to_schema(&self, builder: &mut SchemaBuilder) -> Result<()> {
        match &self.kind {
            FieldKind::Text => {
                let mut opts = TEXT;
                if self.stored {
                    opts = opts | STORED;
                }
                if self.fast {
                    opts = opts | FAST;
                }
                builder.add_text_field(&self.name, opts);
            }
            FieldKind::String => {
                let mut opts = STRING;
                if self.stored {
                    opts = opts | STORED;
                }
                if self.fast {
                    opts = opts | FAST;
                }
                builder.add_text_field(&self.name, opts);
            }
            FieldKind::U64 => {
                let mut opts = NumericOptions::default();
                if self.stored {
                    opts = opts.set_stored();
                }
                if self.indexed {
                    opts = opts.set_indexed();
                }
                if self.fast {
                    opts = opts.set_fast();
                }
                builder.add_u64_field(&self.name, opts);
            }
            FieldKind::I64 => {
                let mut opts = NumericOptions::default();
                if self.stored {
                    opts = opts.set_stored();
                }
                if self.indexed {
                    opts = opts.set_indexed();
                }
                if self.fast {
                    opts = opts.set_fast();
                }
                builder.add_i64_field(&self.name, opts);
            }
            FieldKind::F64 => {
                let mut opts = NumericOptions::default();
                if self.stored {
                    opts = opts.set_stored();
                }
                if self.indexed {
                    opts = opts.set_indexed();
                }
                if self.fast {
                    opts = opts.set_fast();
                }
                builder.add_f64_field(&self.name, opts);
            }
            FieldKind::Bool => {
                let mut opts = NumericOptions::default();
                if self.stored {
                    opts = opts.set_stored();
                }
                if self.indexed {
                    opts = opts.set_indexed();
                }
                if self.fast {
                    opts = opts.set_fast();
                }
                builder.add_bool_field(&self.name, opts);
            }
            FieldKind::Date => {
                let mut opts = DateOptions::default();
                if self.stored {
                    opts = opts.set_stored();
                }
                if self.indexed {
                    opts = opts.set_indexed();
                }
                if self.fast {
                    opts = opts.set_fast();
                }
                builder.add_date_field(&self.name, opts);
            }
            FieldKind::Facet => {
                let mut opts = FacetOptions::default();
                if self.stored {
                    opts = opts.set_stored();
                }
                builder.add_facet_field(&self.name, opts);
            }
            FieldKind::Bytes => {
                let mut opts = BytesOptions::default();
                if self.stored {
                    opts = opts.set_stored();
                }
                if self.indexed {
                    opts = opts.set_indexed();
                }
                if self.fast {
                    opts = opts.set_fast();
                }
                builder.add_bytes_field(&self.name, opts);
            }
            FieldKind::Json => {
                let mut opts = JsonObjectOptions::default();
                if self.stored {
                    opts = opts.set_stored();
                }
                if self.indexed {
                    opts = opts.set_indexing_options(
                        tantivy::schema::TextFieldIndexing::default()
                            .set_index_option(tantivy::schema::IndexRecordOption::WithFreqsAndPositions),
                    );
                }
                if self.fast {
                    opts = opts.set_fast(None);
                }
                builder.add_json_field(&self.name, opts);
            }
        };

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDef {
    pub fields: Vec<FieldDef>,
}

impl SchemaDef {
    pub fn to_schema(&self) -> Result<Schema> {
        let mut builder = Schema::builder();
        for field in &self.fields {
            field.add_to_schema(&mut builder)?;
        }
        Ok(builder.build())
    }
}
