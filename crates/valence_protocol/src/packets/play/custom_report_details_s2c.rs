use crate::packets::configuration::custom_report_details_s2c::CustomReportDetail;
use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct CustomReportDetailsS2c<'a> {
    pub details: Vec<CustomReportDetail<'a>>,
}
