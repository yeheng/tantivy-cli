use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde_json::json;

use crate::error::Result;
use crate::index::manager::IndexManager;
use crate::index::ops;
use crate::index::schema::SchemaDef;
use crate::search::{SearchRequest, search_index};

#[derive(Parser)]
#[command(name = "tantivy-cli")]
#[command(about = "A CLI and HTTP server for Tantivy search engine")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Base directory for indexes
    #[arg(short, long, default_value = "./indexes")]
    pub index_dir: PathBuf,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new index with a schema
    CreateIndex {
        name: String,
        #[arg(short, long)]
        schema_file: PathBuf,
    },
    /// List all indexes
    ListIndexes,
    /// Delete an index
    DeleteIndex {
        name: String,
    },
    /// Add a document to an index
    AddDoc {
        index: String,
        #[arg(short, long)]
        doc: String,
    },
    /// Get a document by field value
    GetDoc {
        index: String,
        field: String,
        value: String,
    },
    /// Delete documents by field value
    DeleteDoc {
        index: String,
        field: String,
        value: String,
    },
    /// Search an index
    Search {
        index: String,
        query: String,
        #[arg(short, long, default_value = "10")]
        limit: usize,
        #[arg(short, long, default_value = "0")]
        offset: usize,
        #[arg(long)]
        highlight: Vec<String>,
    },
    /// Show index stats
    Stats {
        index: String,
    },
    /// Rebuild (merge) index
    Rebuild {
        index: String,
    },
    /// Compress (commit) index
    Compress {
        index: String,
    },
    /// List documents in an index
    ListDocs {
        index: String,
        #[arg(short, long, default_value = "10")]
        limit: usize,
        #[arg(short, long, default_value = "0")]
        offset: usize,
    },
    /// Start HTTP server
    Serve {
        #[arg(short, long, default_value = "127.0.0.1:3000")]
        bind: String,
    },
}

pub async fn run_cli(cli: Cli, manager: &IndexManager) -> Result<()> {
    match cli.command {
        Commands::CreateIndex { name, schema_file } => {
            let schema_str = tokio::fs::read_to_string(&schema_file).await?;
            let schema_def: SchemaDef = serde_json::from_str(&schema_str)?;
            manager.create_index(&name, &schema_def).await?;
            println!("Index '{}' created.", name);
        }
        Commands::ListIndexes => {
            manager.load_all_indexes()?;
            let names = manager.list_indexes();
            if names.is_empty() {
                println!("No indexes found.");
            } else {
                for name in names {
                    println!("{}", name);
                }
            }
        }
        Commands::DeleteIndex { name } => {
            manager.delete_index(&name).await?;
            println!("Index '{}' deleted.", name);
        }
        Commands::AddDoc { index, doc } => {
            let handle = manager.open_index(&index).await?;
            let doc_json: serde_json::Value = serde_json::from_str(&doc)?;
            let id = ops::add_document(&handle, &doc_json).await?;
            println!("Document added: {}", id);
        }
        Commands::GetDoc { index, field, value } => {
            let handle = manager.open_index(&index).await?;
            let doc = ops::get_document(&handle, Some(&field), &value).await?;
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
        Commands::DeleteDoc { index, field, value } => {
            let handle = manager.open_index(&index).await?;
            let deleted = ops::delete_documents(&handle, &field, &value).await?;
            println!("Deleted {} documents.", deleted);
        }
        Commands::Search {
            index,
            query,
            limit,
            offset,
            highlight,
        } => {
            let handle = manager.open_index(&index).await?;
            let req = SearchRequest {
                query,
                limit,
                offset,
                highlight_fields: highlight,
                snippet_max_chars: 150,
            };
            let resp = search_index(&handle, &req).await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }
        Commands::Stats { index } => {
            let handle = manager.open_index(&index).await?;
            let stats = ops::index_stats(&handle).await?;
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        Commands::Rebuild { index } => {
            let handle = manager.open_index(&index).await?;
            ops::rebuild_index(&handle).await?;
            println!("Index '{}' rebuilt.", index);
        }
        Commands::Compress { index } => {
            let handle = manager.open_index(&index).await?;
            ops::compress_index(&handle).await?;
            println!("Index '{}' compressed.", index);
        }
        Commands::ListDocs { index, limit, offset } => {
            let handle = manager.open_index(&index).await?;
            let docs = ops::list_documents(&handle, limit, offset).await?;
            println!("{}", serde_json::to_string_pretty(&json!(docs))?);
        }
        Commands::Serve { bind } => {
            println!("Starting server at {} ...", bind);
            crate::server::serve(manager.clone(), &bind).await?;
        }
    }
    Ok(())
}
