pub use relic_scanner::StarRailRelicScanner;
pub use relic_scanner_config::StarRailRelicScannerConfig;
pub use scan_result::StarRailRelicScanResult;
// pub use relic_scanner_window_info::RelicScannerWindowInfo;

mod match_colors;
mod message_items;
mod relic_scanner;
mod relic_scanner_config;
pub mod relic_scanner_window_info;
mod relic_scanner_worker;
mod scan_result;
