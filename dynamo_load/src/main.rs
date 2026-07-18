use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::types::{AttributeValue, PutRequest, WriteRequest};
use clap::{Parser, ValueEnum};
use std::{collections::HashMap, env};

/// Command-line tool for loading JSON items into a DynamoDB table.
#[derive(Parser)]
#[command(name = "dynamo_load", version)]
struct Cli {
    /// Target environment: local, dev, or prod
    #[arg(value_enum)]
    r#where: Where,

    /// Path to JSON file containing an array of items
    filename: String,

    /// DynamoDB table name to load items into
    tablename: String,
}

/// Supported DynamoDB environments.
#[derive(Debug, Clone, ValueEnum)]
enum Where {
    Local,
    Dev,
    Prod,
}

impl std::fmt::Display for Where {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Where::Local => write!(f, "local"),
            Where::Dev => write!(f, "dev"),
            Where::Prod => write!(f, "prod"),
        }
    }
}

/// Build a DynamoDB client for the given environment.
///
/// For `Local`, connects to a local DynamoDB instance at `localhost:8000`.
/// For `Dev`/`Prod`, uses the default AWS credential chain.
async fn build_client(r#where: &Where) -> aws_sdk_dynamodb::Client {
    let c = aws_config::defaults(BehaviorVersion::latest());
    
    let config = match r#where {
        Where::Local => {
            let local_url = env::var("DYNAMO_ENDPOINT_URL").unwrap_or("http://localhost:8000".to_string());
            c.endpoint_url(local_url).load().await
        },
        _ => {
            c.load().await
        }
    };

    aws_sdk_dynamodb::Client::new(&config)
}

/// Read and parse a JSON file containing an array of items.
fn load_items_from_file(filename: &str) -> Result<Vec<serde_json::Value>> {
    let content = std::fs::read_to_string(filename)
        .with_context(|| format!("Failed to read file: {filename}"))?;
    let items: Vec<serde_json::Value> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON from file: {filename}"))?;
    Ok(items)
}

/// Recursively convert a `serde_json::Value` to a DynamoDB `AttributeValue`.
fn json_value_to_attribute(value: &serde_json::Value) -> AttributeValue {
    match value {
        serde_json::Value::Null => AttributeValue::Null(true),
        serde_json::Value::Bool(b) => AttributeValue::Bool(*b),
        serde_json::Value::Number(n) => AttributeValue::N(n.to_string()),
        serde_json::Value::String(s) => AttributeValue::S(s.clone()),
        serde_json::Value::Array(arr) => {
            let items: Vec<AttributeValue> = arr.iter().map(json_value_to_attribute).collect();
            AttributeValue::L(items)
        }
        serde_json::Value::Object(map) => {
            let items: HashMap<String, AttributeValue> = map
                .iter()
                .map(|(k, v)| (k.clone(), json_value_to_attribute(v)))
                .collect();
            AttributeValue::M(items)
        }
    }
}

/// Convert a JSON value (must be an object) into a DynamoDB item map.
fn json_item_to_dynamodb_item(item: &serde_json::Value) -> Result<HashMap<String, AttributeValue>> {
    match item {
        serde_json::Value::Object(map) => {
            let dynamo_item: HashMap<String, AttributeValue> = map
                .iter()
                .map(|(k, v)| (k.clone(), json_value_to_attribute(v)))
                .collect();
            Ok(dynamo_item)
        }
        _ => anyhow::bail!("Each item in the JSON array must be a JSON object"),
    }
}

/// Write items to a DynamoDB table using batch writes (max 25 per batch).
///
/// Automatically retries unprocessed items with exponential backoff.
async fn batch_write_items(
    client: &aws_sdk_dynamodb::Client,
    table_name: &str,
    items: &[HashMap<String, AttributeValue>],
) -> Result<()> {
    for chunk in items.chunks(25) {
        let write_requests: Vec<WriteRequest> = chunk
            .iter()
            .map(|item| {
                WriteRequest::builder()
                    .put_request(
                        PutRequest::builder()
                            .set_item(Some(item.clone()))
                            .build()
                            .expect("valid PutRequest"),
                    )
                    .build()
            })
            .collect();

        let mut request_items = HashMap::new();
        request_items.insert(table_name.to_string(), write_requests);

        let mut current_items = Some(request_items);
        let mut attempts: u32 = 0;
        const MAX_ATTEMPTS: u32 = 5;

        loop {
            let response = client
                .batch_write_item()
                .set_request_items(current_items.take())
                .send()
                .await?;

            match response.unprocessed_items() {
                Some(unprocessed) if !unprocessed.is_empty() => {
                    attempts += 1;
                    if attempts >= MAX_ATTEMPTS {
                        anyhow::bail!(
                            "Failed to process {} items after {MAX_ATTEMPTS} attempts",
                            unprocessed.len()
                        );
                    }
                    current_items = Some(unprocessed.clone());
                    let delay = std::time::Duration::from_millis(100 * 2u64.pow(attempts));
                    tokio::time::sleep(delay).await;
                }
                _ => break,
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let client = build_client(&cli.r#where).await;
    let json_items = load_items_from_file(&cli.filename)?;

    if json_items.is_empty() {
        eprintln!("Warning: No items found in the JSON file.");

        return Ok(());
    }

    let dynamo_items: Vec<HashMap<String, AttributeValue>> = json_items
        .iter()
        .map(json_item_to_dynamodb_item)
        .collect::<Result<Vec<_>>>()?;

    println!(
        "Loading {} items into table '{}' ({})...",
        dynamo_items.len(),
        cli.tablename,
        cli.r#where,
    );

    batch_write_items(&client, &cli.tablename, &dynamo_items).await?;

    println!("Successfully loaded {} items.", dynamo_items.len());

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_value_to_attribute_string() {
        let attr = json_value_to_attribute(&json!("hello"));
        assert_eq!(attr.as_s().unwrap(), "hello");
    }

    #[test]
    fn test_json_value_to_attribute_number() {
        let attr = json_value_to_attribute(&json!(42));
        assert_eq!(attr.as_n().unwrap(), "42");
    }

    #[test]
    fn test_json_value_to_attribute_bool() {
        let attr = json_value_to_attribute(&json!(true));
        assert_eq!(*attr.as_bool().unwrap(), true);
    }

    #[test]
    fn test_json_value_to_attribute_null() {
        let attr = json_value_to_attribute(&serde_json::Value::Null);
        assert!(attr.as_null().is_ok());
    }

    #[test]
    fn test_json_value_to_attribute_array() {
        let attr = json_value_to_attribute(&json!([1, 2, 3]));
        let list = attr.as_l().expect("expected a list");
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_json_value_to_attribute_object() {
        let attr = json_value_to_attribute(&json!({"name": "test", "age": 30}));
        let map = attr.as_m().expect("expected a map");
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get("name").and_then(|v| v.as_s().ok()),
            Some(&"test".to_string())
        );
        assert_eq!(
            map.get("age").and_then(|v| v.as_n().ok()),
            Some(&"30".to_string())
        );
    }

    #[test]
    fn test_json_item_to_dynamodb_item_valid() {
        let item = json_item_to_dynamodb_item(&json!({"pk": "123", "name": "test"}))
            .expect("valid object");
        assert_eq!(
            item.get("pk").and_then(|v| v.as_s().ok()),
            Some(&"123".to_string())
        );
        assert_eq!(item.len(), 2);
    }

    #[test]
    fn test_json_item_to_dynamodb_item_invalid() {
        let result = json_item_to_dynamodb_item(&json!("not an object"));
        assert!(result.is_err());
    }

    #[test]
    fn test_json_item_to_dynamodb_item_nested() {
        let item = json_item_to_dynamodb_item(&json!({
            "pk": "1",
            "data": {"nested": true}
        }))
        .expect("valid nested object");
        let data = item
            .get("data")
            .and_then(|v| v.as_m().ok())
            .expect("nested map");
        assert_eq!(
            data.get("nested").and_then(|v| v.as_bool().ok()),
            Some(&true)
        );
    }
}
