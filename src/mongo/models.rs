use std::fmt;

use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum VideoStatus {
    UNUPLOADED,
    PENDING,
    PROCESSING,
    READY,
    FAILED,
    DELETED,
}

impl fmt::Display for VideoStatus{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VideoStatus::UNUPLOADED => write!(f, "UNUPLOADED"),
            VideoStatus::PENDING => write!(f, "PENDING"),
            VideoStatus::PROCESSING => write!(f, "PROCESSING"),
            VideoStatus::READY => write!(f, "READY"),
            VideoStatus::FAILED => write!(f, "FAILED"),
            VideoStatus::DELETED => write!(f, "DELETED"),
        }
    }
}   


#[derive(Debug, Serialize, Deserialize)]
pub struct Video {
    // #[serde(with = "hex_string_as_object_id")]
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub status: VideoStatus,
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub thumbnail: Option<String>,
    #[serde(rename = "publishedAt", skip_serializing_if = "Option::is_none")]
    pub published_at: Option<DateTime>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,
    #[serde(rename = "deletedAt", skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<ObjectId>,
}


// mongodb::bson::serde_helpers::hex_string_as_object_id;

// #[derive(Debug, Serialize, Deserialize)]
// pub struct Account {
//     // #[serde(with = "hex_string_as_object_id")]
//     #[serde(rename = "_id")]
//     pub id: Option<ObjectId>,
//     #[serde(rename = "userId")]
//     pub user_id: Option<ObjectId>,
//     pub account_type: String,
//     pub provider: String,
//     #[serde(rename = "providerAccountId")]
//     pub provider_account_id: String,
//     pub refresh_token: Option<String>,
//     pub access_token: Option<String>,
//     pub expires_at: Option<i32>,
//     pub token_type: Option<String>,
//     pub scope: Option<String>,
//     pub id_token: Option<String>,
//     pub session_state: Option<String>,
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct Session {
//     // #[serde(with = "hex_string_as_object_id")]
//     #[serde(rename = "_id")]
//     pub id: Option<ObjectId>,
//     #[serde(rename = "sessionToken")]
//     pub session_token: String,
//     // #[serde(with = "hex_string_as_object_id")]
//     pub user_id: Option<ObjectId>,
//     pub expires: DateTime,
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct User {
//     // #[serde(with = "hex_string_as_object_id")]
//     #[serde(rename = "_id")]
//     pub id: Option<ObjectId>,
//     pub name: Option<String>,
//     pub email: Option<String>,
//     #[serde(rename = "emailVerified")]
//     pub email_verified: Option<DateTime>,
//     pub image: Option<String>,
//     pub accounts: Vec<Account>,
//     pub sessions: Vec<Session>,
//     pub videos: Vec<Video>,
// }
