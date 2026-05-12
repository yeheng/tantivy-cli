use serde::{Deserialize, Serialize};
use tantivy::schema::{
    BytesOptions, DateOptions, FAST, FacetOptions, JsonObjectOptions, NumericOptions, STORED,
    STRING, Schema, SchemaBuilder, TEXT,
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

/// Helper to apply common stored/indexed/fast flags to NumericOptions.
fn numeric_opts(stored: bool, indexed: bool, fast: bool) -> NumericOptions {
    let mut opts = NumericOptions::default();
    if stored {
        opts = opts.set_stored();
    }
    if indexed {
        opts = opts.set_indexed();
    }
    if fast {
        opts = opts.set_fast();
    }
    opts
}

/// Helper to apply common stored/indexed/fast flags to DateOptions.
fn date_opts(stored: bool, indexed: bool, fast: bool) -> DateOptions {
    let mut opts = DateOptions::default();
    if stored {
        opts = opts.set_stored();
    }
    if indexed {
        opts = opts.set_indexed();
    }
    if fast {
        opts = opts.set_fast();
    }
    opts
}

/// Helper to apply stored flag to FacetOptions.
fn facet_opts(stored: bool) -> FacetOptions {
    let mut opts = FacetOptions::default();
    if stored {
        opts = opts.set_stored();
    }
    opts
}

/// Helper to apply stored/indexed/fast flags to BytesOptions.
fn bytes_opts(stored: bool, indexed: bool, fast: bool) -> BytesOptions {
    let mut opts = BytesOptions::default();
    if stored {
        opts = opts.set_stored();
    }
    if indexed {
        opts = opts.set_indexed();
    }
    if fast {
        opts = opts.set_fast();
    }
    opts
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
                builder.add_u64_field(&self.name, numeric_opts(self.stored, self.indexed, self.fast));
            }
            FieldKind::I64 => {
                builder.add_i64_field(&self.name, numeric_opts(self.stored, self.indexed, self.fast));
            }
            FieldKind::F64 => {
                builder.add_f64_field(&self.name, numeric_opts(self.stored, self.indexed, self.fast));
            }
            FieldKind::Bool => {
                builder.add_bool_field(&self.name, numeric_opts(self.stored, self.indexed, self.fast));
            }
            FieldKind::Date => {
                builder.add_date_field(&self.name, date_opts(self.stored, self.indexed, self.fast));
            }
            FieldKind::Facet => {
                builder.add_facet_field(&self.name, facet_opts(self.stored));
            }
            FieldKind::Bytes => {
                builder.add_bytes_field(&self.name, bytes_opts(self.stored, self.indexed, self.fast));
            }
            FieldKind::Json => {
                let mut opts = JsonObjectOptions::default();
                if self.stored {
                    opts = opts.set_stored();
                }
                if self.indexed {
                    opts = opts.set_indexing_options(
                        tantivy::schema::TextFieldIndexing::default().set_index_option(
                            tantivy::schema::IndexRecordOption::WithFreqsAndPositions,
                        ),
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
