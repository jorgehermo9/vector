use futures::FutureExt;
use tower::ServiceBuilder;
use vector_lib::{
    config::AcknowledgementsConfig,
    configurable::{component::GenerateConfig, configurable_component},
    sink::VectorSink,
};

use super::{
    service::{PostgresRetryLogic, PostgresService},
    sink::PostgresSink,
};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

use crate::{
    config::{Input, SinkConfig, SinkContext},
    sinks::{
        util::{
            BatchConfig, RealtimeSizeBasedDefaultBatchSettings, ServiceBuilderExt,
            TowerRequestConfig, UriSerde,
        },
        Healthcheck,
    },
};

/// Configuration for the `postgres` sink.
#[configurable_component(sink("postgres", "Deliver log data to a PostgreSQL database."))]
#[derive(Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PostgresConfig {
    /// TODO
    /// TODO: if I used UriSerde instead of String, I couldn't get a url string to use
    /// in the connection pool, as the password would be redacted with UriSerde::to_string
    pub endpoint: String,

    /// TODO
    pub table: String,

    #[configurable(derived)]
    #[serde(default)]
    pub batch: BatchConfig<RealtimeSizeBasedDefaultBatchSettings>,

    #[configurable(derived)]
    #[serde(default)]
    pub request: TowerRequestConfig,

    #[configurable(derived)]
    #[serde(
        default,
        deserialize_with = "crate::serde::bool_or_struct",
        skip_serializing_if = "crate::serde::is_default"
    )]
    pub acknowledgements: AcknowledgementsConfig,
}

impl GenerateConfig for PostgresConfig {
    fn generate_config() -> toml::Value {
        toml::from_str(
            r#"endpoint = "postgres://user:password@localhost/default"
            table = "default"
        "#,
        )
        .unwrap()
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "postgres")]
impl SinkConfig for PostgresConfig {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(VectorSink, Healthcheck)> {
        // TODO: make connection pool configurable. Or should we just have one connection per sink?
        // TODO: it seems that the number of connections in the pool does not affect the throughput of the sink
        // does the sink execute batches in parallel?
        let connection_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&self.endpoint)
            .await?;

        let healthcheck = healthcheck(connection_pool.clone()).boxed();

        let batch_settings = self.batch.into_batcher_settings()?;
        let request_settings = self.request.into_settings();

        let endpoint_uri: UriSerde = self.endpoint.parse()?;
        let service = PostgresService::new(
            connection_pool,
            self.table.clone(),
            // TODO: this endpoint is used for metrics' tags. It could contain passwords,
            // will it be redacted there?
            endpoint_uri.to_string(),
        );
        let service = ServiceBuilder::new()
            .settings(request_settings, PostgresRetryLogic)
            .service(service);

        let sink = PostgresSink::new(service, batch_settings);

        Ok((VectorSink::from_event_streamsink(sink), healthcheck))
    }

    fn input(&self) -> Input {
        Input::log()
    }

    fn acknowledgements(&self) -> &AcknowledgementsConfig {
        &self.acknowledgements
    }
}

async fn healthcheck(connection_pool: Pool<Postgres>) -> crate::Result<()> {
    sqlx::query("SELECT 1").execute(&connection_pool).await?;
    Ok(())
}
