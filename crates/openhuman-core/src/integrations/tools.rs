use super::IntegrationClient;

mod google_places;
mod stock_prices;

pub use google_places::{GooglePlacesDetailsTool, GooglePlacesSearchTool};
pub use stock_prices::{
    StockCommodityTool, StockCryptoSeriesTool, StockExchangeRateTool, StockOptionsTool,
    StockQuoteTool,
};

/// Build this domain's tools in their persisted catalogue order.
pub(crate) fn registered_tools(
    config: std::sync::Arc<crate::config::Config>,
) -> Vec<Box<dyn tinytools::Tool>> {
    vec![
        Box::new(SessionStateTool::new(config.clone())),
        Box::new(OAuthConnectUrlTool::new(config.clone())),
        Box::new(OAuthListTool::new(config.clone())),
    ]
}
