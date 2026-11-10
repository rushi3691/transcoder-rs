//! CloudAMQP (LavinMQ / RabbitMQ) consumer for transcode jobs.

use lapin::{
    options::{BasicConsumeOptions, BasicQosOptions, QueueDeclareOptions},
    types::FieldTable,
    Connection, ConnectionProperties, Consumer,
};

/// Must match the queue the frontend publishes to (transcoder-frontend/lib/queue.ts).
pub const QUEUE: &str = "transcode-jobs";

/// Connect and start consuming. Keep the returned `Connection` alive while consuming.
pub async fn consume(url: &str) -> lapin::Result<(Connection, Consumer)> {
    let conn = Connection::connect(url, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;
    channel
        .queue_declare(
            QUEUE,
            QueueDeclareOptions { durable: true, ..Default::default() },
            FieldTable::default(),
        )
        .await?;
    // one unacked job at a time: the next delivery waits until the current transcode is acked
    channel.basic_qos(1, BasicQosOptions::default()).await?;
    let consumer = channel
        .basic_consume(QUEUE, "transcoder", BasicConsumeOptions::default(), FieldTable::default())
        .await?;
    Ok((conn, consumer))
}
