//! JMAP mail method types.

mod email;
mod mailbox;

#[cfg(test)]
mod email_tests;

pub use email::{
    BodyPartReference, BodyValue, EMAIL_GET_METHOD, EMAIL_QUERY_METHOD, EMAIL_SET_METHOD, Email,
    EmailAddress, EmailGetArguments, EmailGetResponse, EmailQueryArguments, EmailQueryFilter,
    EmailQueryResponse, EmailSetArguments, EmailSetResponse, mark_seen_patch,
};
pub use mailbox::{MAILBOX_GET_METHOD, Mailbox, MailboxGetArguments, MailboxGetResponse};
