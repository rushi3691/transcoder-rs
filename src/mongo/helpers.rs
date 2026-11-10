use super::models;

use mongodb::bson::{doc, oid::ObjectId};
use std::str::FromStr;

pub async fn update_video_status(
    video_collection: &mongodb::Collection<models::Video>,
    video_id: &str,
    status: models::VideoStatus,
) -> mongodb::error::Result<()> {
    let filter = doc! {"_id": ObjectId::from_str(video_id).unwrap()};

    // ! remove this, just for testing
    let doc = video_collection.find_one(filter.clone(), None).await?;
    println!("doc: {:?}", doc);


    let result = video_collection
        .update_one(
            filter,
            doc! {"$set": doc!{"status": status.to_string()}},
            None,
        )
        .await?;

    println!("result: {:?}", result);
    Ok(())
}

/// Duration in seconds, shown as the badge on thumbnails.
pub async fn set_video_duration(
    video_collection: &mongodb::Collection<models::Video>,
    video_id: &str,
    duration: f64,
) -> mongodb::error::Result<()> {
    let filter = doc! {"_id": ObjectId::from_str(video_id).unwrap()};
    video_collection
        .update_one(filter, doc! {"$set": {"duration": duration}}, None)
        .await?;
    Ok(())
}
