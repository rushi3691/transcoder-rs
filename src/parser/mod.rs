use serde::Deserialize;
use serde_json::Error;
#[derive(Debug, Deserialize)]
pub struct Record {
    #[serde(rename = "eventVersion")]
    pub event_version: String,
    #[serde(rename = "eventSource")]
    pub event_source: String,
    #[serde(rename = "awsRegion")]
    pub aws_region: String,
    #[serde(rename = "eventTime")]
    pub event_time: String,
    #[serde(rename = "eventName")]
    pub event_name: String,
    // userIdentity: UserIdentity,
    // requestParameters: RequestParameters,
    // responseElements: ResponseElements,
    pub s3: S3,
}

#[derive(Debug, Deserialize)]
pub struct S3 {
    #[serde(rename = "s3SchemaVersion")]
    pub s3_schema_version: String,
    #[serde(rename = "configurationId")]
    pub configuration_id: String,
    pub bucket: Bucket,
    pub object: Object,
}

#[derive(Debug, Deserialize)]
pub struct Bucket {
    pub name: String,
    // ownerIdentity: UserIdentity,
    pub arn: String,
}

#[derive(Debug, Deserialize)]
pub struct Object {
    pub key: String,
    pub size: u64,
    // eTag: String,
    pub sequencer: String,
}

#[derive(Debug, Deserialize)]
pub struct ParsedMessage {
    #[serde(rename = "Records")]
    pub records: Vec<Record>,
}

pub fn parse_msg(message: &str) -> Result<ParsedMessage, Error>{
    println!("message: {}", message);
    let parsed: ParsedMessage = serde_json::from_str(message)?;
    Ok(parsed)
}