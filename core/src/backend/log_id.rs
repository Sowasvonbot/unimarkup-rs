//! Defines log-ids for the backend section

use logid::log_id::{self, EventLevel};

use crate::log_id::LogSubGrp;

enum LogSubSubGrp {
    Loader = 1,
    Inline = 3,
}

/// Loader error log-ids for the backend section
#[derive(Debug)]
pub enum LoaderErrLogId {
    /// Log-id denoting an invalid element type
    InvalidElementType = log_id::get_log_id(
        LogSubGrp::Backend as u8,
        LogSubSubGrp::Loader as u8,
        EventLevel::Error,
        0,
    ),
}

/// Inline error log-ids for the backend section
#[derive(Debug)]
pub enum InlineErrLogId {
    /// Log-id denoting that no inline elements were detected
    NoInlineDetected = log_id::get_log_id(
        LogSubGrp::Backend as u8,
        LogSubSubGrp::Inline as u8,
        EventLevel::Error,
        0,
    ),
}
