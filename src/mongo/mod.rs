pub mod models;
pub mod helpers;

use mongodb::{
    bson::doc,
    options::ClientOptions,
    Client,
};

pub async fn get_video_collection() -> mongodb::error::Result<mongodb::Collection<models::Video>> {
    let uri = std::env::var("MONGO_URI").expect("MONGO_URI not set");
    let client_options = ClientOptions::parse(uri).await?;

    let client = Client::with_options(client_options)?;

    // Ping the server to see if you can connect to the cluster
    client
        .database("admin")
        .run_command(doc! {"ping": 1}, None)
        .await?;
    println!("Pinged your deployment. You successfully connected to MongoDB!");

    let db = client.database("video_service");

    let video_collection = db.collection::<models::Video>("Video");

    Ok(video_collection)
}
