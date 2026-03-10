//! Transport-agnostic JMAP request and response codec types.

pub mod core;
pub mod mail;

pub use core::{
    CORE_CAPABILITY, CapabilityMap, Invocation, JmapError, JmapResponse, MAIL_CAPABILITY,
    MethodResponse, MethodResponseData, RequestObject, ResponseShapeError, SessionResource,
};
