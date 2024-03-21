extern crate serialport;
extern crate macaddr;
#[macro_use]
extern crate lazy_static;

pub mod unimotion;

// #[cfg(doctest)]
// #[macro_use]
// extern crate doc_comment;

// #[cfg(doctest)]
// doctest!("../README.md");

pub mod prelude {
    pub use crate::unimotion::*;
    pub use crate::result::*;
    pub use crossbeam_channel;
    pub(crate) use serialport;
    #[cfg(feature = "use_serde")]
    pub(crate) use serde::{Deserialize, Serialize};
}

pub mod result {
    use std::io::Error as IOError;
    use crossbeam_channel::{SendError, SendTimeoutError};
    use serialport::Error as SerialportError;

    use crate::unimotion::device::AcknowledgeType;

    #[derive(Debug)]
    pub enum UnimotionError {
        IOError(IOError),
        SerialportError(SerialportError),
        SubCommandError(u8, Vec<u8>),
        UnimotionDeviceError(UnimotionDeviceError),
        UnimotionReportError(UnimotionReportError),
        Disconnected,
        // TODO: Dispatch into their corresponding errors
        PlaceholderError(PlaceholderError),
        // CrossbeamChannelError(CrossbeamChannelError<T>)
        CrossbeamChannelError,
    }

    impl From<SerialportError> for UnimotionError {
        fn from(e: SerialportError) -> Self {
            UnimotionError::SerialportError(e)
        }
    }

    impl From<IOError> for UnimotionError {
        fn from(e: IOError) -> Self {
            UnimotionError::IOError(e)
        }
    }

    impl From<PlaceholderError> for UnimotionError {
        fn from(e: PlaceholderError) -> Self {
            UnimotionError::PlaceholderError(e)
        }
    }


    #[derive(Debug)]
    pub enum UnimotionDeviceError {
        InvalidVendorID(u16),
        InvalidProductID(u16),
        FailedStickParameterLoading,
        FailedStickCalibrationLoading,
        FailedIMUOffsetsLoading,
        FailedIMUCalibrationLoading,
        FailedColorLoading,
    }

    impl From<UnimotionDeviceError> for UnimotionError {
        fn from(e: UnimotionDeviceError) -> Self {
            UnimotionError::UnimotionDeviceError(e)
        }
    }

    #[derive(Debug)]
    pub enum UnimotionReportError {
        InvalidSimpleHidReport(InvalidSimpleHIDReport),
        InvalidStandardInputReport(InvalidStandardInputReport),
        EmptyReport,
    }

    impl From<UnimotionReportError> for UnimotionError {
        fn from(e: UnimotionReportError) -> Self {
            UnimotionError::UnimotionReportError(e)
        }
    }

    #[derive(Debug)]
    pub enum InvalidSimpleHIDReport {
        InvalidReport(Vec<u8>),
        InvalidStickDirection(u8),
    }

    impl From<InvalidSimpleHIDReport> for UnimotionReportError {
        fn from(e: InvalidSimpleHIDReport) -> Self {
            UnimotionReportError::InvalidSimpleHidReport(e)
        }
    }

    impl From<InvalidSimpleHIDReport> for UnimotionError {
        fn from(e: InvalidSimpleHIDReport) -> Self {
            let report_error = UnimotionReportError::from(e);
            UnimotionError::from(report_error)
        }
    }

    #[derive(Debug)]
    pub enum InvalidStandardInputReport {
        InvalidReport(Vec<u8>),
        InvalidExtraReport(Vec<u8>),
        Battery(u8),
        ConnectionInfo(u8),
        InvalidInputReportId(u8),
    }

    impl From<InvalidStandardInputReport> for UnimotionReportError {
        fn from(e: InvalidStandardInputReport) -> Self {
            UnimotionReportError::InvalidStandardInputReport(e)
        }
    }

    impl From<InvalidStandardInputReport> for UnimotionError {
        fn from(e: InvalidStandardInputReport) -> Self {
            let report_error = UnimotionReportError::from(e);
            UnimotionError::from(report_error)
        }
    }

// TODO: Dispatch into their corresponding errors
// BEGIN
    #[derive(Debug)]
    pub enum PlaceholderError {
        UnexpectedAckError(AcknowledgeType, AcknowledgeType),
    }

    pub enum CrossbeamChannelError<T>{
        SendError(SendError<T>),
        SendTimeoutError(SendTimeoutError<T>),
    }
// END

    pub type UnimotionResult<T> = Result<T, UnimotionError>;
}
