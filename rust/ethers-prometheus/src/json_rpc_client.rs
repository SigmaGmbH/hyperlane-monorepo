//! A wrapper around a JsonRpcClient to give insight at the request level. This
//! was designed specifically for use with the quorum provider.

use std::fmt::{Debug, Formatter};
use std::time::Instant;

use async_trait::async_trait;
use derive_builder::Builder;
use derive_new::new;
use ethers::prelude::JsonRpcClient;
use ethers_core::types::U64;
use hyperlane_core::rpc_clients::BlockNumberGetter;
use hyperlane_core::ChainCommunicationError;
use maplit::hashmap;
use prometheus::{CounterVec, IntCounterVec};
use serde::{de::DeserializeOwned, Serialize};

pub use crate::ChainInfo;

/// Some basic information about a node.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "camelCase"))]
pub struct NodeInfo {
    /// The host of the node, e.g. `alchemy.com`, `quicknode.pro`, or
    /// `localhost:8545`.
    pub host: Option<String>,
}

/// Container for all the relevant rpc client metrics.
#[derive(Clone, Builder)]
pub struct JsonRpcClientMetrics {
    /// Total number of requests made to this client.
    /// - `provider_node`: node this is connecting to, e.g. `alchemy.com`,
    ///   `quicknode.pro`, or `localhost:8545`.
    /// - `chain`: chain name (or chain id if the name is unknown) of the chain
    ///   the request was made on.
    /// - `method`: request method string.
    /// - `status`: `success` or `failure` depending on the response. A `success`
    ///   might still be an "error" but not one with the transport layer.
    #[builder(setter(into, strip_option), default)]
    request_count: Option<IntCounterVec>,

    /// Total number of seconds spent making requests.
    /// - `provider_node`: node this is connecting to, e.g. `alchemy.com`,
    ///   `quicknode.pro`, or `localhost:8545`.
    /// - `chain`: chain name (or chain id if the name is unknown) of the chain
    ///   the request was made on.
    /// - `method`: request method string.
    /// - `status`: `success` or `failure` depending on the response. A `success`
    ///   might still be an "error" but not one with the transport layer.
    #[builder(setter(into, strip_option), default)]
    request_duration_seconds: Option<CounterVec>,
}

/// Expected label names for the metric.
pub const REQUEST_COUNT_LABELS: &[&str] = &["provider_node", "chain", "method", "status"];
/// Help string for the metric.
pub const REQUEST_COUNT_HELP: &str = "Total number of requests made to this client";

/// Expected label names for the metric.
pub const REQUEST_DURATION_SECONDS_LABELS: &[&str] =
    &["provider_node", "chain", "method", "status"];
/// Help string for the metric.
pub const REQUEST_DURATION_SECONDS_HELP: &str = "Total number of seconds spent making requests";

/// Configuration for the prometheus JsonRpcClioent. This can be loaded via
/// serde.
#[derive(Default, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "camelCase"))]
pub struct PrometheusJsonRpcClientConfig {
    /// Information about what node this client is connecting to.
    pub node: Option<NodeInfo>,

    /// Information about the chain this client is for.
    pub chain: Option<ChainInfo>,
}

/// Helper functions for displaying node and chain information
pub trait PrometheusJsonRpcClientConfigExt {
    /// The "host" part of the URL this node is connecting to. E.g.
    /// `avalanche.api.onfinality.io`.
    fn node_host(&self) -> &str;
    /// Chain name this RPC client is connected to.
    fn chain_name(&self) -> &str;
}

impl PrometheusJsonRpcClientConfigExt for PrometheusJsonRpcClientConfig {
    fn node_host(&self) -> &str {
        self.node
            .as_ref()
            .and_then(|n| n.host.as_ref())
            .map(|h| h.as_str())
            .unwrap_or("unknown")
    }
    fn chain_name(&self) -> &str {
        self.chain
            .as_ref()
            .and_then(|c| c.name.as_ref())
            .map(|n| n.as_str())
            .unwrap_or("unknown")
    }
}

/// An ethers-rs JsonRpcClient wrapper that instruments requests with prometheus
/// metrics. To make this as flexible as possible, the metric vecs need to be
/// created and named externally, they should follow the naming convention here
/// and must include the described labels.
#[derive(new)]
pub struct PrometheusJsonRpcClient<C> {
    inner: C,
    metrics: JsonRpcClientMetrics,
    config: PrometheusJsonRpcClientConfig,
}

impl<C: Clone> Clone for PrometheusJsonRpcClient<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            metrics: self.metrics.clone(),
            config: self.config.clone(),
        }
    }
}

impl<C> Debug for PrometheusJsonRpcClient<C>
where
    C: JsonRpcClient,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PrometheusJsonRpcClient({:?})", self.inner)
    }
}

impl<C> PrometheusJsonRpcClient<C> {
    /// The inner RpcClient implementation
    pub fn inner(&self) -> &C {
        &self.inner
    }
}

impl<C> PrometheusJsonRpcClientConfigExt for PrometheusJsonRpcClient<C> {
    /// The "host" part of the URL this node is connecting to. E.g.
    /// `avalanche.api.onfinality.io`.
    fn node_host(&self) -> &str {
        self.config.node_host()
    }

    /// Chain name this RPC client is connected to.
    fn chain_name(&self) -> &str {
        self.config.chain_name()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> JsonRpcClient for PrometheusJsonRpcClient<C>
where
    C: JsonRpcClient,
{
    type Error = C::Error;

    async fn request<T, R>(&self, method: &str, params: T) -> Result<R, Self::Error>
    where
        T: Debug + Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        let start = Instant::now();
        let res = self.inner.request(method, params).await;
        let labels = hashmap! {
            "provider_node" => self.config.node_host(),
            "chain" => self.config.chain_name(),
            "method" => method,
            "status" => if res.is_ok() { "success" } else { "failure" }
        };
        if let Some(counter) = &self.metrics.request_count {
            counter.with(&labels).inc()
        }
        if let Some(counter) = &self.metrics.request_duration_seconds {
            counter
                .with(&labels)
                .inc_by((Instant::now() - start).as_secs_f64())
        };
        res
    }
}

impl<C: JsonRpcClient + 'static> From<PrometheusJsonRpcClient<C>> for Box<dyn BlockNumberGetter> {
    fn from(val: PrometheusJsonRpcClient<C>) -> Self {
        Box::new(JsonRpcBlockGetter::new(val))
    }
}

/// Utility struct for implementing `BlockNumberGetter`
#[derive(Debug, new)]
pub struct JsonRpcBlockGetter<T: JsonRpcClient>(T);

/// RPC method for getting the latest block number
pub const BLOCK_NUMBER_RPC: &str = "eth_blockNumber";

#[async_trait]
impl<C> BlockNumberGetter for JsonRpcBlockGetter<C>
where
    C: JsonRpcClient,
{
    async fn get_block_number(&self) -> Result<u64, ChainCommunicationError> {
        let res = self
            .0
            .request(BLOCK_NUMBER_RPC, ())
            .await
            .map(|r: U64| r.as_u64())
            .map_err(Into::into)?;
        Ok(res)
    }
}
