//! Ingestion massive multi-sources (CSMAR L2/L3, CTP, Logs NLP) et synchronisation monotone.
#![forbid(unsafe_code)]

pub mod ctp_client;
pub mod encoding;
pub mod historical_reader;
pub mod synchronizer;
